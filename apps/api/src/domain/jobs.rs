//! A small Postgres-backed job queue (the `jobs` table).
//!
//! - `claim` takes the oldest ready job with `FOR UPDATE SKIP LOCKED`, so any
//!   number of workers (in one process or many) never take the same job.
//! - A failed job is retried with exponential backoff until `max_attempts`.
//! - A job left `running` by a crashed worker is put back by `reap_stuck`.

use std::time::Duration;

use serde_json::Value;
use sqlx::{PgConnection, PgPool};

use crate::error::AppResult;

pub const GRADE_SESSION: &str = "grade_session";

/// A job running longer than this is assumed abandoned by a dead worker.
pub const STUCK_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Job {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    /// Including the current run.
    pub attempts: i32,
    pub max_attempts: i32,
}

/// Queue a job. Kinds with a uniqueness rule (one grading per session) are
/// silently de-duplicated, so callers can enqueue without checking first.
pub async fn enqueue(conn: &mut PgConnection, kind: &str, payload: Value) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO jobs (kind, payload) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(kind)
    .bind(payload)
    .execute(conn)
    .await?;
    Ok(())
}

/// Take the next ready job, if any, and mark it running.
pub async fn claim(db: &PgPool) -> AppResult<Option<Job>> {
    let job = sqlx::query_as(
        "UPDATE jobs
         SET status = 'running', locked_at = now(), attempts = attempts + 1, updated_at = now()
         WHERE id = (
             SELECT id FROM jobs
             WHERE status = 'queued' AND run_at <= now()
             ORDER BY run_at
             LIMIT 1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, kind, payload, attempts, max_attempts",
    )
    .fetch_optional(db)
    .await?;
    Ok(job)
}

pub async fn succeed(db: &PgPool, id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE jobs SET status = 'done', locked_at = NULL, last_error = NULL, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

/// Backoff before retry `attempts` (1-based): 15 s, 30 s, 60 s ... capped at 1 h.
pub fn backoff(attempts: i32) -> Duration {
    let exp = attempts.clamp(1, 12) as u32 - 1;
    Duration::from_secs((15u64 << exp).min(3600))
}

/// Record a failure. Returns `true` if the job will be retried, `false` if
/// it has used all its attempts and is now `failed` for good.
pub async fn fail(db: &PgPool, job: &Job, error: &str) -> AppResult<bool> {
    let retry = job.attempts < job.max_attempts;
    // Errors can contain provider responses; keep them short in the table.
    let error: String = error.chars().take(500).collect();
    if retry {
        sqlx::query(
            "UPDATE jobs
             SET status = 'queued', locked_at = NULL, last_error = $2,
                 run_at = now() + make_interval(secs => $3), updated_at = now()
             WHERE id = $1",
        )
        .bind(job.id)
        .bind(&error)
        .bind(backoff(job.attempts).as_secs_f64())
        .execute(db)
        .await?;
    } else {
        sqlx::query(
            "UPDATE jobs SET status = 'failed', locked_at = NULL, last_error = $2, updated_at = now()
             WHERE id = $1",
        )
        .bind(job.id)
        .bind(&error)
        .execute(db)
        .await?;
    }
    Ok(retry)
}

/// Put jobs from crashed workers back in the queue. Returns how many.
pub async fn reap_stuck(db: &PgPool) -> AppResult<u64> {
    let res = sqlx::query(
        "UPDATE jobs SET status = 'queued', locked_at = NULL, updated_at = now()
         WHERE status = 'running' AND locked_at < now() - make_interval(secs => $1)",
    )
    .bind(STUCK_AFTER.as_secs_f64())
    .execute(db)
    .await?;
    Ok(res.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_caps() {
        assert_eq!(backoff(1), Duration::from_secs(15));
        assert_eq!(backoff(2), Duration::from_secs(30));
        assert_eq!(backoff(3), Duration::from_secs(60));
        assert_eq!(backoff(20), Duration::from_secs(3600));
        assert_eq!(backoff(0), Duration::from_secs(15));
    }
}
