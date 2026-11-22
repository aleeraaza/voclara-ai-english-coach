//! Start and finish live speaking sessions.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    domain::{
        sessions::{self, Mode, NewSession},
        users,
    },
    error::{AppError, AppResult, ErrorBody},
    providers::voice::{SessionConfig, Transport},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sessions", post(start_session))
        .route("/sessions/{id}/complete", post(complete_session))
}

#[derive(Deserialize, ToSchema)]
pub struct StartSessionRequest {
    /// talk | mock | interview
    pub mode: String,
    /// Free-text topic for talk/mock. For interview, the role:
    /// software | sales | banking | nursing | call_centre.
    pub topic: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct StartSessionResponse {
    pub session_id: Uuid,
    pub mode: String,
    pub provider: String,
    pub model: String,
    pub transport: Transport,
    /// Where the app connects. Gemini: WebSocket, append `?access_token=<token>`.
    /// OpenAI: POST the WebRTC SDP offer here with `Authorization: Bearer <token>`.
    pub connect_url: String,
    /// Ephemeral provider token. Never our API keys.
    pub token: String,
    /// Connect before this or the token stops working.
    pub token_expires_at: DateTime<Utc>,
    /// Session length limit in seconds: min(minutes left, mode limit).
    pub max_secs: i32,
    /// The app must end the session by this time.
    pub session_ends_at: DateTime<Utc>,
    /// True if the provider itself cuts the session off at the limit.
    pub hard_cutoff: bool,
    pub prompt_version: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CompleteSessionRequest {
    /// Caption log from the session: `[{"role": "learner"|"tutor", "text": "...", "t_ms": 0}, ...]`.
    #[schema(value_type = Option<Vec<Object>>)]
    pub transcript: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct CompleteSessionResponse {
    pub session_id: Uuid,
    /// Seconds taken from the wallet (elapsed time, capped at the limit).
    pub seconds_charged: i32,
    pub balance_secs: i64,
}

/// Start a live session: checks minutes, builds the persona prompt and returns
/// an ephemeral provider token limited to the user's remaining time.
#[utoipa::path(
    post, path = "/api/v1/sessions", tag = "sessions",
    security(("bearer" = [])),
    request_body = StartSessionRequest,
    responses(
        (status = 201, description = "Session started", body = StartSessionResponse),
        (status = 400, description = "Invalid mode or topic", body = ErrorBody),
        (status = 401, description = "Not signed in", body = ErrorBody),
        (status = 402, description = "Not enough minutes", body = ErrorBody),
        (status = 409, description = "Another session is still running", body = ErrorBody),
        (status = 503, description = "Voice provider unavailable", body = ErrorBody)
    )
)]
pub async fn start_session(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<StartSessionRequest>,
) -> AppResult<(StatusCode, Json<StartSessionResponse>)> {
    let mode = Mode::parse(&req.mode)?;
    let topic = sessions::clean_topic(req.topic.as_deref())?;
    let profile = users::get(&state.db, user.id).await?;
    let level = profile.level.as_deref().unwrap_or("b1");
    let goal = profile.goal.as_deref().unwrap_or("daily");

    let (prompt, system_prompt) = match mode {
        Mode::Talk => {
            let p = &state.prompts.tutor;
            let t = topic.as_deref().unwrap_or("everyday life");
            (
                p,
                p.render(&[("level", level), ("goal", goal), ("topic", t)]),
            )
        }
        Mode::Mock => {
            let p = &state.prompts.examiner;
            let t = topic.as_deref().unwrap_or("a common everyday topic");
            (p, p.render(&[("level", level), ("topic", t)]))
        }
        Mode::Interview => {
            let key = topic
                .as_deref()
                .ok_or_else(|| AppError::BadRequest("interview needs a role as topic".into()))?;
            let role = sessions::interview_role(key)?;
            let p = &state.prompts.interviewer;
            (p, p.render(&[("role", role), ("level", level)]))
        }
    };

    let primary = state.voice.primary();
    let reserved = sessions::reserve(
        &state.db,
        NewSession {
            user_id: user.id,
            mode,
            topic: topic.as_deref(),
            provider: primary.name(),
            model: primary.model(),
            prompt_version: &prompt.version,
        },
        state.config.voice.usd_per_min,
    )
    .await?;

    let cfg = SessionConfig { system_prompt };
    // `max_secs` is positive (checked in `reserve`), so the conversion holds.
    let ttl = u32::try_from(reserved.max_secs).map_err(|e| AppError::Internal(e.into()))?;
    let (provider, token) = match state.voice.mint(&cfg, ttl).await {
        Ok(ok) => ok,
        Err(e) => {
            // Free the slot so the user can retry; nothing is charged.
            sessions::mark_failed(&state.db, reserved.id).await?;
            return Err(e);
        }
    };
    if provider.name() != primary.name() {
        sessions::set_provider(&state.db, reserved.id, provider.name(), provider.model()).await?;
    }

    tracing::info!(
        session_id = %reserved.id, user_id = %user.id, mode = mode.as_str(),
        provider = provider.name(), max_secs = reserved.max_secs, "session started"
    );
    Ok((
        StatusCode::CREATED,
        Json(StartSessionResponse {
            session_id: reserved.id,
            mode: mode.as_str().into(),
            provider: provider.name().into(),
            model: provider.model().into(),
            transport: token.transport,
            connect_url: token.connect_url,
            token: token.token,
            token_expires_at: token.token_expires_at,
            max_secs: reserved.max_secs,
            session_ends_at: Utc::now() + chrono::Duration::seconds(i64::from(reserved.max_secs)),
            hard_cutoff: token.hard_cutoff,
            prompt_version: prompt.version.clone(),
        }),
    ))
}

/// Finish a session: stores the transcript and charges the elapsed time.
#[utoipa::path(
    post, path = "/api/v1/sessions/{id}/complete", tag = "sessions",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Session id")),
    request_body = CompleteSessionRequest,
    responses(
        (status = 200, description = "Session completed", body = CompleteSessionResponse),
        (status = 400, description = "Transcript is not a list", body = ErrorBody),
        (status = 404, description = "No such session for this user", body = ErrorBody),
        (status = 409, description = "Session already ended", body = ErrorBody)
    )
)]
pub async fn complete_session(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteSessionRequest>,
) -> AppResult<Json<CompleteSessionResponse>> {
    if req.transcript.as_ref().is_some_and(|t| !t.is_array()) {
        return Err(AppError::BadRequest(
            "transcript must be a list of turns".into(),
        ));
    }
    let done = sessions::complete(
        &state.db,
        user.id,
        id,
        req.transcript,
        state.config.voice.usd_per_min,
    )
    .await?;
    tracing::info!(session_id = %id, user_id = %user.id, seconds = done.seconds_charged, "session completed");
    Ok(Json(CompleteSessionResponse {
        session_id: id,
        seconds_charged: done.seconds_charged,
        balance_secs: done.balance_secs,
    }))
}
