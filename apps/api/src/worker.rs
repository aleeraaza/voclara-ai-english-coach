//! Background workers that drain the `jobs` table.
//!
//! Each worker loops: claim a job, run it with a timeout, mark it done or
//! failed (failures retry with backoff). Worker 0 also puts back jobs left
//! `running` by a crashed process. In tests, `run_pending` drains the queue
//! synchronously instead.

use std::time::Duration;

use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{
    domain::{
        grading::{self, GradeResult},
        jobs::{self, Job},
    },
    error::{AppError, AppResult},
    state::AppState,
};

const JOB_TIMEOUT: Duration = Duration::from_secs(180);
const IDLE_POLL: Duration = Duration::from_secs(2);
const REAP_EVERY: Duration = Duration::from_secs(60);

fn session_id(job: &Job) -> AppResult<Uuid> {
    job.payload["session_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::BadRequest(format!("job {} has no session_id", job.id)))
}

async fn dispatch(state: &AppState, job: &Job) -> AppResult<()> {
    match job.kind.as_str() {
        jobs::GRADE_SESSION => {
            let id = session_id(job)?;
            let result = grading::grade_session(
                &state.db,
                state.grader.as_ref(),
                &state.prompts,
                &state.config.grader,
                id,
            )
            .await?;
            tracing::info!(session_id = %id, ?result, "session graded");
            if result == GradeResult::Skipped {
                tracing::info!(session_id = %id, "too little learner speech to grade");
            }
            Ok(())
        }
        other => Err(AppError::BadRequest(format!("unknown job kind '{other}'"))),
    }
}

/// What to do once a job has failed for the last time.
async fn on_gave_up(state: &AppState, job: &Job) {
    // Rust 2024 let-chains: one `if` instead of three nested ones.
    if job.kind == jobs::GRADE_SESSION
        && let Ok(id) = session_id(job)
        && let Err(e) = grading::mark_failed(&state.db, id).await
    {
        tracing::error!(session_id = %id, error = ?e, "marking grading failed");
    }
}

/// Claim and run one job. `Ok(false)` means the queue had nothing ready.
pub async fn run_one(state: &AppState) -> AppResult<bool> {
    let Some(job) = jobs::claim(&state.db).await? else {
        return Ok(false);
    };
    let outcome = tokio::time::timeout(JOB_TIMEOUT, dispatch(state, &job)).await;
    let error = match outcome {
        Ok(Ok(())) => {
            jobs::succeed(&state.db, job.id).await?;
            return Ok(true);
        }
        Ok(Err(e)) => format!("{e:?}"),
        Err(_) => format!("timed out after {JOB_TIMEOUT:?}"),
    };
    let retry = jobs::fail(&state.db, &job, &error).await?;
    if retry {
        tracing::warn!(job_id = job.id, kind = %job.kind, attempt = job.attempts, %error, "job failed; will retry");
    } else {
        tracing::error!(job_id = job.id, kind = %job.kind, attempts = job.attempts, %error, "job failed for good");
        on_gave_up(state, &job).await;
    }
    Ok(true)
}

/// Run jobs until none is ready. Returns how many ran. Used by tests.
pub async fn run_pending(state: &AppState) -> AppResult<usize> {
    let mut n = 0;
    while run_one(state).await? {
        n += 1;
    }
    Ok(n)
}

/// Start `count` background workers. They run until the process exits; a
/// job interrupted by shutdown is picked up again by `reap_stuck`.
pub fn spawn(state: AppState, count: usize) -> Vec<JoinHandle<()>> {
    (0..count)
        .map(|i| {
            let state = state.clone();
            tokio::spawn(async move {
                let mut last_reap = tokio::time::Instant::now() - REAP_EVERY;
                loop {
                    if i == 0 && last_reap.elapsed() >= REAP_EVERY {
                        last_reap = tokio::time::Instant::now();
                        match jobs::reap_stuck(&state.db).await {
                            Ok(0) => {}
                            Ok(n) => {
                                tracing::warn!(count = n, "requeued jobs from a crashed worker")
                            }
                            Err(e) => tracing::error!(error = ?e, "reaping stuck jobs"),
                        }
                    }
                    match run_one(&state).await {
                        Ok(true) => {}
                        Ok(false) => tokio::time::sleep(IDLE_POLL).await,
                        Err(e) => {
                            tracing::error!(worker = i, error = ?e, "worker error");
                            tokio::time::sleep(IDLE_POLL).await;
                        }
                    }
                }
            })
        })
        .collect()
}
