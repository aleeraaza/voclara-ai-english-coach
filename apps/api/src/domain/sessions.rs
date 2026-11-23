//! Live session lifecycle: reserve -> (provider token) -> complete.
//!
//! Rules that protect the margin:
//! - One active session per user. Each session's time limit is
//!   min(balance, mode limit); two at once could spend the same minutes twice.
//! - On completion we charge the wall-clock time the server saw, capped at the
//!   limit. The app cannot under-report.
//! - A session never completed (app killed, network lost) is charged its full
//!   limit once it is clearly over: we cannot know how long it really ran.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    domain::{jobs, minutes},
    error::{AppError, AppResult},
};

/// Sessions shorter than this are not worth starting.
pub const MIN_START_SECS: i64 = 30;
/// How long after its limit an unfinished session counts as abandoned.
pub const ABANDON_GRACE_SECS: i32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Talk,
    Mock,
    Interview,
}

impl Mode {
    pub fn parse(raw: &str) -> AppResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "talk" => Ok(Self::Talk),
            "mock" => Ok(Self::Mock),
            "interview" => Ok(Self::Interview),
            _ => Err(AppError::BadRequest(
                "mode must be one of: talk, mock, interview".into(),
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Talk => "talk",
            Self::Mock => "mock",
            Self::Interview => "interview",
        }
    }

    /// Longest single session per mode. Short sessions keep the provider's
    /// per-turn context (and so the cost per minute) low.
    pub fn limit_secs(self) -> i64 {
        match self {
            Self::Talk => 600,
            Self::Mock => 900,
            Self::Interview => 900,
        }
    }
}

/// Interview role packs: (key the app sends, wording used in the prompt).
pub const INTERVIEW_ROLES: &[(&str, &str)] = &[
    ("software", "software engineer"),
    ("sales", "sales executive"),
    ("banking", "banking"),
    ("nursing", "nursing"),
    ("call_centre", "call centre agent"),
];

pub fn interview_role(key: &str) -> AppResult<&'static str> {
    INTERVIEW_ROLES
        .iter()
        .find(|(k, _)| *k == key.trim().to_ascii_lowercase())
        .map(|(_, label)| *label)
        .ok_or_else(|| {
            let keys: Vec<&str> = INTERVIEW_ROLES.iter().map(|(k, _)| *k).collect();
            AppError::BadRequest(format!(
                "interview topic must be one of: {}",
                keys.join(", ")
            ))
        })
}

/// User-written topics go into the system prompt, so keep them short, on one
/// line and free of control characters.
pub fn clean_topic(raw: Option<&str>) -> AppResult<Option<String>> {
    let Some(raw) = raw else { return Ok(None) };
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.chars().count() > 120 {
        return Err(AppError::BadRequest(
            "topic must be at most 120 characters".into(),
        ));
    }
    Ok(Some(cleaned))
}

pub struct Reserved {
    pub id: Uuid,
    pub max_secs: i32,
}

pub struct NewSession<'a> {
    pub user_id: Uuid,
    pub mode: Mode,
    pub topic: Option<&'a str>,
    pub provider: &'a str,
    pub model: &'a str,
    pub prompt_version: &'a str,
    /// True: the app connects to our relay WebSocket instead of the provider.
    pub relay: bool,
    /// Mock only: the cue card this test uses.
    pub cue_card_id: Option<&'a str>,
}

/// How long a relay ticket stays valid after the session is reserved.
pub const RELAY_TICKET_SECS: i64 = 60;

/// Charge this user's abandoned sessions, in a transaction of its own.
///
/// It must commit on its own: if it shared a transaction with a session start
/// that is then refused (e.g. not enough minutes), the error would roll the
/// settlement back and the stale session would block the user forever.
/// Safe to call any time; settled sessions are no longer `active`.
pub async fn settle_abandoned_for(db: &PgPool, user_id: Uuid, usd_per_min: f64) -> AppResult<()> {
    let mut tx = db.begin().await?;
    minutes::lock_user(&mut tx, user_id).await?;
    settle_abandoned(&mut tx, user_id, usd_per_min).await?;
    tx.commit().await?;
    Ok(())
}

/// Create the session row with its time limit, or refuse.
pub async fn reserve(db: &PgPool, new: NewSession<'_>, usd_per_min: f64) -> AppResult<Reserved> {
    settle_abandoned_for(db, new.user_id, usd_per_min).await?;

    let mut tx = db.begin().await?;
    minutes::lock_user(&mut tx, new.user_id).await?;

    let running: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM sessions WHERE user_id = $1 AND status = 'active'")
            .bind(new.user_id)
            .fetch_optional(&mut *tx)
            .await?;
    if running.is_some() {
        return Err(AppError::Conflict(
            "a session is already running; complete it first".into(),
        ));
    }

    let balance = minutes::balance(&mut *tx, new.user_id).await?;
    let max_secs = balance.min(new.mode.limit_secs());
    if max_secs < MIN_START_SECS {
        return Err(AppError::InsufficientMinutes);
    }
    // Safe: max_secs <= mode limit (900), which fits in i32.
    let max_secs = i32::try_from(max_secs).map_err(|e| AppError::Internal(e.into()))?;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO sessions (user_id, mode, topic, provider, model, prompt_version, max_secs, transport, cue_card_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(new.user_id)
    .bind(new.mode.as_str())
    .bind(new.topic)
    .bind(new.provider)
    .bind(new.model)
    .bind(new.prompt_version)
    .bind(max_secs)
    .bind(if new.relay { "relay" } else { "direct" })
    .bind(new.cue_card_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Reserved { id, max_secs })
}

/// Store a relay session's single-use ticket (hashed) and its rendered prompt.
pub async fn set_relay_ticket(
    db: &PgPool,
    id: Uuid,
    ticket_hash: &[u8],
    system_prompt: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE sessions
         SET relay_ticket_hash = $2,
             relay_ticket_expires_at = now() + make_interval(secs => $3),
             relay_prompt = $4
         WHERE id = $1 AND transport = 'relay'",
    )
    .bind(id)
    .bind(ticket_hash)
    .bind(RELAY_TICKET_SECS as f64)
    .bind(system_prompt)
    .execute(db)
    .await?;
    Ok(())
}

/// What the relay needs once the app has connected.
#[derive(sqlx::FromRow)]
pub struct OpenedRelay {
    pub user_id: Uuid,
    pub provider: String,
    pub system_prompt: Option<String>,
    /// Seconds left before the session's hard limit (measured from start).
    pub remaining_secs: f64,
}

/// Consume a relay ticket: valid, unexpired, for an active relay session.
/// One statement, so two connects racing with the same ticket cannot both win.
pub async fn open_relay(db: &PgPool, id: Uuid, ticket_hash: &[u8]) -> AppResult<OpenedRelay> {
    sqlx::query_as(
        "WITH s AS (
            SELECT id, user_id, provider, relay_prompt, max_secs, started_at FROM sessions
            WHERE id = $1 AND transport = 'relay' AND status = 'active'
              AND relay_ticket_hash = $2 AND relay_ticket_expires_at > now()
            FOR UPDATE
         )
         UPDATE sessions
         SET relay_ticket_hash = NULL, relay_ticket_expires_at = NULL, relay_prompt = NULL
         FROM s WHERE sessions.id = s.id
         RETURNING s.user_id, s.provider, s.relay_prompt AS system_prompt,
                   GREATEST(0, s.max_secs - EXTRACT(EPOCH FROM now() - s.started_at))::float8
                       AS remaining_secs",
    )
    .bind(id)
    .bind(ticket_hash)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::Unauthorized)
}

/// Record which provider actually served the session (the fallback may have).
pub async fn set_provider(db: &PgPool, id: Uuid, provider: &str, model: &str) -> AppResult<()> {
    sqlx::query("UPDATE sessions SET provider = $2, model = $3 WHERE id = $1")
        .bind(id)
        .bind(provider)
        .bind(model)
        .execute(db)
        .await?;
    Ok(())
}

/// The provider could not issue a token: free the slot, charge nothing.
pub async fn mark_failed(db: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE sessions SET status = 'failed', ended_at = now(), seconds_used = 0
         WHERE id = $1 AND status = 'active'",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

pub struct Completed {
    pub seconds_charged: i32,
    pub balance_secs: i64,
    /// A mock that used one of the user's prepaid mock credits.
    pub mock_credit_used: bool,
}

/// A mock shorter than this was abandoned, not taken: it costs no mock credit.
pub const MOCK_CREDIT_MIN_SECS: i32 = 120;

/// Recent cue cards for this user, newest first, so a new mock can avoid them.
pub async fn recent_cue_cards(db: &PgPool, user_id: Uuid, limit: i64) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT cue_card_id FROM sessions
         WHERE user_id = $1 AND mode = 'mock' AND cue_card_id IS NOT NULL
         ORDER BY started_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

/// Take one mock credit from the user's active entitlement that expires
/// first, if any has credits left. Plans without mocks leave mocks on minutes.
async fn use_mock_credit(conn: &mut PgConnection, user_id: Uuid) -> AppResult<bool> {
    let used = sqlx::query(
        "UPDATE entitlements SET mocks_left = mocks_left - 1, updated_at = now()
         WHERE id = (
             SELECT id FROM entitlements
             WHERE user_id = $1 AND status = 'active' AND mocks_left > 0
               AND starts_at <= now() AND (ends_at IS NULL OR ends_at > now())
             ORDER BY ends_at NULLS LAST, created_at
             LIMIT 1
             FOR UPDATE
         )",
    )
    .bind(user_id)
    .execute(conn)
    .await?
    .rows_affected();
    Ok(used == 1)
}

#[derive(sqlx::FromRow)]
struct ActiveRow {
    status: String,
    mode: String,
    transport: String,
    max_secs: i32,
    provider: String,
    elapsed_secs: f64,
}

/// Who is ending the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndedBy {
    /// `POST /sessions/{id}/complete` from the app (direct mode only).
    Client,
    /// The relay, when its sockets close. Carries the measured audio seconds.
    Relay { input_secs: i32, output_secs: i32 },
}

/// End a session: store the transcript, charge the elapsed time (capped at the
/// limit) and log its cost, all in one transaction.
pub async fn complete(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    transcript: Option<serde_json::Value>,
    usd_per_min: f64,
    ended_by: EndedBy,
) -> AppResult<Completed> {
    let mut tx = db.begin().await?;
    // Same lock order as `reserve` and `debit_seconds`: user first, then session.
    minutes::lock_user(&mut tx, user_id).await?;
    let row: ActiveRow = sqlx::query_as(
        "SELECT status, mode, transport, max_secs, provider,
                EXTRACT(EPOCH FROM now() - started_at)::float8 AS elapsed_secs
         FROM sessions WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if row.status != "active" {
        return Err(AppError::Conflict(format!(
            "session already {}",
            row.status
        )));
    }
    // A relay session ends when the relay closes. Letting the app "complete"
    // it early would stop the meter while the relay keeps streaming.
    if ended_by == EndedBy::Client && row.transport == "relay" {
        return Err(AppError::Conflict(
            "relay sessions end when the relay socket closes".into(),
        ));
    }

    // `as` saturates for floats, and elapsed is clamped to [0, max_secs].
    let charged = (row.elapsed_secs.ceil().max(0.0) as i32).min(row.max_secs);

    sqlx::query(
        "UPDATE sessions
         SET status = 'completed', ended_at = now(), seconds_used = $2,
             transcript = COALESCE($3, transcript)
         WHERE id = $1",
    )
    .bind(session_id)
    .bind(charged)
    .bind(transcript)
    .execute(&mut *tx)
    .await?;

    let balance_secs = if charged > 0 {
        minutes::debit_locked(&mut tx, user_id, session_id, charged).await?
    } else {
        minutes::balance(&mut *tx, user_id).await?
    };
    let usage = match ended_by {
        EndedBy::Relay {
            input_secs,
            output_secs,
        } => Usage::Measured {
            input_secs,
            output_secs,
        },
        EndedBy::Client => Usage::Estimated,
    };
    record_cost(
        &mut tx,
        session_id,
        &row.provider,
        charged,
        usd_per_min,
        usage,
    )
    .await?;

    let mock_credit_used = row.mode == Mode::Mock.as_str()
        && charged >= MOCK_CREDIT_MIN_SECS
        && use_mock_credit(&mut tx, user_id).await?;

    // Queue grading in the same transaction: a completed session always has
    // exactly one grading job (a unique index de-duplicates retries).
    jobs::enqueue(
        &mut tx,
        jobs::GRADE_SESSION,
        serde_json::json!({ "session_id": session_id }),
    )
    .await?;
    sqlx::query("UPDATE sessions SET grading_status = 'pending' WHERE id = $1")
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Completed {
        seconds_charged: charged,
        balance_secs,
        mock_credit_used,
    })
}

/// Charge sessions that ran out without being completed. Caller holds the user lock.
async fn settle_abandoned(
    conn: &mut PgConnection,
    user_id: Uuid,
    usd_per_min: f64,
) -> AppResult<()> {
    let stale: Vec<(Uuid, i32, String)> = sqlx::query_as(
        "SELECT id, max_secs, provider FROM sessions
         WHERE user_id = $1 AND status = 'active'
           AND started_at + make_interval(secs => max_secs + $2) < now()
         FOR UPDATE",
    )
    .bind(user_id)
    .bind(ABANDON_GRACE_SECS)
    .fetch_all(&mut *conn)
    .await?;

    for (id, max_secs, provider) in stale {
        // The limit was within the balance at start and only one session runs
        // at a time, so this fits; `min` guards against manual ledger edits.
        let balance = minutes::balance(&mut *conn, user_id).await?;
        let charge = i64::from(max_secs).min(balance.max(0));
        let charge = i32::try_from(charge).map_err(|e| AppError::Internal(e.into()))?;
        sqlx::query(
            "UPDATE sessions
             SET status = 'expired', seconds_used = $2,
                 ended_at = started_at + make_interval(secs => max_secs)
             WHERE id = $1",
        )
        .bind(id)
        .bind(charge)
        .execute(&mut *conn)
        .await?;
        if charge > 0 {
            minutes::debit_locked(&mut *conn, user_id, id, charge).await?;
        }
        record_cost(
            &mut *conn,
            id,
            &provider,
            charge,
            usd_per_min,
            Usage::Estimated,
        )
        .await?;
        tracing::info!(session_id = %id, %user_id, charge, "abandoned session charged in full");
    }
    Ok(())
}

/// What we know about a session's audio for the cost log.
#[derive(Debug, Clone, Copy)]
enum Usage {
    /// Direct mode: the audio never passed through us.
    Estimated,
    /// Relay mode: counted frame by frame.
    Measured { input_secs: i32, output_secs: i32 },
}

/// USD is the planning number per live minute until real provider pricing is
/// read back; the audio seconds are measured in relay mode.
async fn record_cost(
    conn: &mut PgConnection,
    session_id: Uuid,
    provider: &str,
    secs: i32,
    usd_per_min: f64,
    usage: Usage,
) -> AppResult<()> {
    let usd = f64::from(secs) / 60.0 * usd_per_min;
    let (input_secs, output_secs, tokens) = match usage {
        Usage::Estimated => (secs, 0, serde_json::json!({ "estimated": true })),
        Usage::Measured {
            input_secs,
            output_secs,
        } => (
            input_secs,
            output_secs,
            serde_json::json!({ "estimated": true, "audio": "measured" }),
        ),
    };
    sqlx::query(
        "INSERT INTO session_costs (session_id, provider, input_secs, output_secs, tokens, usd)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind(provider)
    .bind(input_secs)
    .bind(output_secs)
    .bind(tokens)
    .bind(usd)
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_parse_and_have_limits() {
        assert_eq!(Mode::parse(" Talk ").unwrap(), Mode::Talk);
        assert_eq!(Mode::parse("mock").unwrap().limit_secs(), 900);
        assert!(
            Mode::parse("drill").is_err(),
            "drills use the cheap pipeline, not live voice"
        );
    }

    #[test]
    fn topics_are_cleaned() {
        assert_eq!(clean_topic(None).unwrap(), None);
        assert_eq!(clean_topic(Some("   ")).unwrap(), None);
        assert_eq!(
            clean_topic(Some("my\nfirst   job\t interview"))
                .unwrap()
                .as_deref(),
            Some("my first job interview")
        );
        assert!(clean_topic(Some(&"x".repeat(121))).is_err());
    }

    #[test]
    fn interview_roles_map_to_prompt_wording() {
        assert_eq!(interview_role("Call_Centre").unwrap(), "call centre agent");
        assert!(interview_role("astronaut").is_err());
    }
}
