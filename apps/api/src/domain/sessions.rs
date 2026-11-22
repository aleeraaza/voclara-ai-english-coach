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
    domain::minutes,
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
}

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
        "INSERT INTO sessions (user_id, mode, topic, provider, model, prompt_version, max_secs)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(new.user_id)
    .bind(new.mode.as_str())
    .bind(new.topic)
    .bind(new.provider)
    .bind(new.model)
    .bind(new.prompt_version)
    .bind(max_secs)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Reserved { id, max_secs })
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
}

#[derive(sqlx::FromRow)]
struct ActiveRow {
    status: String,
    max_secs: i32,
    provider: String,
    elapsed_secs: f64,
}

/// End a session: store the transcript, charge the elapsed time (capped at the
/// limit) and log its cost, all in one transaction.
pub async fn complete(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    transcript: Option<serde_json::Value>,
    usd_per_min: f64,
) -> AppResult<Completed> {
    let mut tx = db.begin().await?;
    // Same lock order as `reserve` and `debit_seconds`: user first, then session.
    minutes::lock_user(&mut tx, user_id).await?;
    let row: ActiveRow = sqlx::query_as(
        "SELECT status, max_secs, provider,
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
    record_cost(&mut tx, session_id, &row.provider, charged, usd_per_min).await?;
    tx.commit().await?;

    Ok(Completed {
        seconds_charged: charged,
        balance_secs,
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
        record_cost(&mut *conn, id, &provider, charge, usd_per_min).await?;
        tracing::info!(session_id = %id, %user_id, charge, "abandoned session charged in full");
    }
    Ok(())
}

/// Planning-number cost until real provider usage is read back (marked estimated).
async fn record_cost(
    conn: &mut PgConnection,
    session_id: Uuid,
    provider: &str,
    secs: i32,
    usd_per_min: f64,
) -> AppResult<()> {
    let usd = f64::from(secs) / 60.0 * usd_per_min;
    sqlx::query(
        "INSERT INTO session_costs (session_id, provider, input_secs, output_secs, tokens, usd)
         VALUES ($1, $2, $3, 0, '{\"estimated\": true}', $4)",
    )
    .bind(session_id)
    .bind(provider)
    .bind(secs)
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
