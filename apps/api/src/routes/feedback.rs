//! Session feedback (scores, corrections) and the progress overview.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    domain::{grading::MIN_LEARNER_WORDS, metrics::Fillers},
    error::{AppError, AppResult, ErrorBody},
    providers::grader::{Correction, Evidence},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sessions/{id}/feedback", get(get_feedback))
        .route("/progress", get(get_progress))
}

#[derive(Serialize, ToSchema)]
pub struct Bands {
    pub fluency: f64,
    pub lexical: f64,
    pub grammar: f64,
    /// `null` until pronunciation scoring is available for the session.
    pub pronunciation: Option<f64>,
    /// IELTS-style overall band (estimated).
    pub overall: f64,
}

#[derive(Serialize, ToSchema)]
pub struct FeedbackResponse {
    pub session_id: Uuid,
    pub mode: String,
    /// none | pending | ready | failed | skipped (too little speech to grade).
    pub status: String,
    /// True when the scores come from the demo grader, not a real model.
    pub demo: bool,
    /// Language of the `why_l1` explanations.
    pub l1: Option<String>,
    pub bands: Option<Bands>,
    pub evidence: Option<Evidence>,
    pub corrections: Vec<Correction>,
    pub fillers: Option<Fillers>,
    /// Estimated learner speaking speed.
    pub wpm: Option<i32>,
    pub next_focus: Option<String>,
    pub grader_version: Option<String>,
    /// Learner words needed for a session to be graded.
    pub min_words: usize,
}

#[derive(sqlx::FromRow)]
struct FeedbackRow {
    mode: String,
    grading_status: String,
    fluency: Option<f64>,
    lexical: Option<f64>,
    grammar: Option<f64>,
    pronunciation: Option<f64>,
    overall: Option<f64>,
    evidence: Option<Value>,
    corrections: Option<Value>,
    fillers: Option<Value>,
    wpm: Option<i32>,
    next_focus: Option<String>,
    grader_version: Option<String>,
    l1: Option<String>,
}

/// Feedback for one session. Poll while `status` is `pending`.
#[utoipa::path(
    get, path = "/api/v1/sessions/{id}/feedback", tag = "feedback",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Session id")),
    responses(
        (status = 200, description = "Feedback (or its status)", body = FeedbackResponse),
        (status = 404, description = "No such session for this user", body = ErrorBody)
    )
)]
pub async fn get_feedback(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<FeedbackResponse>> {
    let row: FeedbackRow = sqlx::query_as(
        "SELECT s.mode, s.grading_status,
                sc.fluency::float8 AS fluency, sc.lexical::float8 AS lexical,
                sc.grammar::float8 AS grammar, sc.pronunciation::float8 AS pronunciation,
                sc.overall::float8 AS overall, sc.evidence, sc.corrections, sc.fillers,
                sc.wpm, sc.next_focus, sc.grader_version, sc.l1
         FROM sessions s LEFT JOIN scores sc ON sc.session_id = s.id
         WHERE s.id = $1 AND s.user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let ready = row.grading_status == "ready";
    let bands = match (ready, row.fluency, row.lexical, row.grammar, row.overall) {
        (true, Some(fluency), Some(lexical), Some(grammar), Some(overall)) => Some(Bands {
            fluency,
            lexical,
            grammar,
            pronunciation: row.pronunciation,
            overall,
        }),
        _ => None,
    };
    Ok(Json(FeedbackResponse {
        session_id: id,
        mode: row.mode,
        status: row.grading_status,
        demo: row.grader_version.as_deref() == Some("fake"),
        l1: row.l1,
        bands,
        evidence: if ready { parse(row.evidence) } else { None },
        corrections: if ready {
            parse(row.corrections).unwrap_or_default()
        } else {
            vec![]
        },
        fillers: if ready { parse(row.fillers) } else { None },
        wpm: row.wpm.filter(|_| ready),
        next_focus: row.next_focus.filter(|_| ready),
        grader_version: row.grader_version,
        min_words: MIN_LEARNER_WORDS,
    }))
}

/// A closure can't do this: its argument and return types are fixed at its
/// first use, while a generic fn is instantiated once per target type.
fn parse<T: serde::de::DeserializeOwned>(v: Option<Value>) -> Option<T> {
    v.and_then(|v| serde_json::from_value(v).ok())
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ProgressItem {
    pub session_id: Uuid,
    pub mode: String,
    pub started_at: DateTime<Utc>,
    pub overall: f64,
    pub fluency: f64,
    pub lexical: f64,
    pub grammar: f64,
    pub pronunciation: Option<f64>,
    /// Demo grade from the fake grader.
    pub demo: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ProgressResponse {
    /// All graded sessions (real grades only).
    pub graded_sessions: i64,
    /// Mean overall band of the last 5 real grades.
    pub average_overall_last5: Option<f64>,
    /// Newest first, up to 20.
    pub recent: Vec<ProgressItem>,
}

/// Band trend for the progress screen.
#[utoipa::path(
    get, path = "/api/v1/progress", tag = "feedback",
    security(("bearer" = [])),
    responses((status = 200, description = "Recent graded sessions", body = ProgressResponse))
)]
pub async fn get_progress(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<ProgressResponse>> {
    let recent: Vec<ProgressItem> = sqlx::query_as(
        "SELECT s.id AS session_id, s.mode, s.started_at,
                sc.overall::float8 AS overall, sc.fluency::float8 AS fluency,
                sc.lexical::float8 AS lexical, sc.grammar::float8 AS grammar,
                sc.pronunciation::float8 AS pronunciation,
                (sc.grader_version = 'fake') AS demo
         FROM sessions s JOIN scores sc ON sc.session_id = s.id
         WHERE s.user_id = $1 AND s.grading_status = 'ready'
         ORDER BY s.started_at DESC
         LIMIT 20",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let (graded_sessions, average_overall_last5): (i64, Option<f64>) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM sessions s JOIN scores sc ON sc.session_id = s.id
             WHERE s.user_id = $1 AND s.grading_status = 'ready' AND sc.grader_version <> 'fake'),
            (SELECT avg(overall)::float8 FROM (
                SELECT sc.overall FROM sessions s JOIN scores sc ON sc.session_id = s.id
                WHERE s.user_id = $1 AND s.grading_status = 'ready' AND sc.grader_version <> 'fake'
                ORDER BY s.started_at DESC LIMIT 5) last5)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ProgressResponse {
        graded_sessions,
        average_overall_last5: average_overall_last5.map(|v| (v * 10.0).round() / 10.0),
        recent,
    }))
}
