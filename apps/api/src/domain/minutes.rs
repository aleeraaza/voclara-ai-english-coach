//! The minute wallet. All amounts are integer seconds. The ledger is
//! append-only: grants are positive rows, usage is negative rows, and the
//! balance is their sum.

use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// 10 live minutes, granted once when an account is created.
pub const FREE_GRANT_SECS: i32 = 600;
/// Free users get 2 minutes of cheap-mode drills per day (in their timezone).
pub const FREE_DAILY_DRILL_SECS: i64 = 120;

/// Reasons a grant can be made. Usage goes through [`debit_seconds`] only.
#[derive(Debug, Clone, Copy)]
pub enum GrantReason {
    Purchase,
    Subscription,
    Refund,
    Adjustment,
}

impl GrantReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Purchase => "purchase",
            Self::Subscription => "subscription",
            Self::Refund => "refund",
            Self::Adjustment => "adjustment",
        }
    }
}

/// Current balance in seconds (0 for a user with no ledger rows).
///
/// Generic over the executor so it works with `&PgPool` and with `&mut *tx`
/// inside a transaction. `'e` ties the borrow of the executor to the query.
pub async fn balance<'e, E: PgExecutor<'e>>(ex: E, user_id: Uuid) -> AppResult<i64> {
    let secs: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(delta_secs), 0)::bigint FROM minute_ledger WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(ex)
    .await?;
    Ok(secs)
}

/// Grant the one-time free minutes. Returns `false` if already granted
/// (a partial unique index makes a second grant impossible).
pub async fn grant_free(conn: &mut PgConnection, user_id: Uuid) -> AppResult<bool> {
    let res = sqlx::query(
        "INSERT INTO minute_ledger (user_id, delta_secs, reason) VALUES ($1, $2, 'grant_free')
         ON CONFLICT (user_id) WHERE reason = 'grant_free' DO NOTHING",
    )
    .bind(user_id)
    .bind(FREE_GRANT_SECS)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Add seconds to a wallet (purchases, subscriptions, refunds, admin fixes).
pub async fn grant(
    conn: &mut PgConnection,
    user_id: Uuid,
    secs: i32,
    reason: GrantReason,
    purchase_id: Option<Uuid>,
) -> AppResult<()> {
    if secs <= 0 {
        return Err(AppError::BadRequest("grant must be positive".into()));
    }
    sqlx::query(
        "INSERT INTO minute_ledger (user_id, delta_secs, reason, purchase_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(secs)
    .bind(reason.as_str())
    .bind(purchase_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Charge `secs` of usage for `session_id`. Atomic and race-free:
///
/// 1. Lock the user's row (`FOR UPDATE`) so concurrent debits for the same
///    user run one after another, never interleaved.
/// 2. Read the balance and refuse if it would go negative.
/// 3. Insert the usage row. A unique index allows one usage row per session,
///    so a retried "session complete" cannot charge twice.
///
/// Returns the remaining balance.
pub async fn debit_seconds(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    secs: i32,
) -> AppResult<i64> {
    if secs <= 0 {
        return Err(AppError::BadRequest("debit must be positive".into()));
    }
    let mut tx = db.begin().await?;
    lock_user(&mut tx, user_id).await?;
    let remaining = debit_locked(&mut tx, user_id, session_id, secs).await?;
    tx.commit().await?;
    Ok(remaining)
}

/// Lock the user's row for the rest of the transaction. Everything that reads
/// a balance and then writes based on it takes this lock first, always in the
/// same order (user, then session), so concurrent requests cannot deadlock.
pub async fn lock_user(conn: &mut PgConnection, user_id: Uuid) -> AppResult<()> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(conn)
        .await?
        .map(|_| ())
        .ok_or(AppError::NotFound)
}

/// The debit itself. The caller must already hold [`lock_user`] in the same
/// transaction; that lock is what makes the balance check race-free.
pub async fn debit_locked(
    conn: &mut PgConnection,
    user_id: Uuid,
    session_id: Uuid,
    secs: i32,
) -> AppResult<i64> {
    if secs <= 0 {
        return Err(AppError::BadRequest("debit must be positive".into()));
    }
    let current = balance(&mut *conn, user_id).await?;
    if current < i64::from(secs) {
        return Err(AppError::InsufficientMinutes);
    }

    let inserted = sqlx::query(
        "INSERT INTO minute_ledger (user_id, delta_secs, reason, session_id)
         VALUES ($1, $2, 'usage', $3)",
    )
    .bind(user_id)
    .bind(-secs)
    .bind(session_id)
    .execute(conn)
    .await;
    match inserted {
        Ok(_) => Ok(current - i64::from(secs)),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            Err(AppError::Conflict("session already charged".into()))
        }
        Err(e) => Err(e.into()),
    }
}

/// Seconds of free drill time left today, in the user's own timezone.
pub async fn drill_secs_left_today<'e, E: PgExecutor<'e>>(ex: E, user_id: Uuid) -> AppResult<i64> {
    let left: Option<i64> = sqlx::query_scalar(
        "SELECT GREATEST(0, $2 - COALESCE(SUM(s.seconds_used), 0))::bigint
         FROM users u
         LEFT JOIN sessions s
           ON s.user_id = u.id
          AND s.mode = 'drill'
          AND (s.started_at AT TIME ZONE u.timezone)::date = (now() AT TIME ZONE u.timezone)::date
         WHERE u.id = $1
         GROUP BY u.id",
    )
    .bind(user_id)
    .bind(FREE_DAILY_DRILL_SECS)
    .fetch_optional(ex)
    .await?;
    left.ok_or(AppError::NotFound)
}
