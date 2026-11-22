//! Start and finish live speaking sessions.

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::Response,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::{AuthUser, refresh},
    domain::{
        sessions::{self, EndedBy, Mode, NewSession},
        users,
    },
    error::{AppError, AppJson, AppResult, ErrorBody},
    providers::voice::{SessionConfig, Transport},
    relay::{self, EndReason, RelayOutcome},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sessions", post(start_session))
        .route("/sessions/{id}/complete", post(complete_session))
        .route("/sessions/{id}/ws", get(relay_socket))
}

#[derive(Deserialize, IntoParams)]
pub struct RelayQuery {
    /// The single-use `token` from `POST /api/v1/sessions` with `relay: true`.
    pub ticket: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct StartSessionRequest {
    /// talk | mock | interview
    pub mode: String,
    /// Free-text topic for talk/mock. For interview, the role:
    /// software | sales | banking | nursing | call_centre.
    pub topic: Option<String>,
    /// Route audio through the Voclara relay instead of straight to the
    /// provider. Gives a server-enforced time limit for every provider.
    #[serde(default)]
    pub relay: bool,
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
    /// Relay: a path on this API; open a WebSocket to it with `?ticket=<token>`.
    pub connect_url: String,
    /// Ephemeral provider token, or the single-use relay ticket. Never our API keys.
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TurnRole {
    Learner,
    Tutor,
}

/// One caption turn. The grader reads only `learner` turns.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct TranscriptTurn {
    pub role: TurnRole,
    pub text: String,
    /// Milliseconds from session start.
    pub t_ms: u64,
}

/// Upper bound on what one session can produce; stops oversized uploads.
const MAX_TURNS: usize = 2_000;
const MAX_TEXT_CHARS: usize = 100_000;

#[derive(Deserialize, ToSchema)]
pub struct CompleteSessionRequest {
    /// Caption log from the session, in order.
    pub transcript: Option<Vec<TranscriptTurn>>,
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
    AppJson(req): AppJson<StartSessionRequest>,
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
            relay: req.relay,
        },
        state.config.voice.usd_per_min,
    )
    .await?;

    if req.relay {
        // No provider token: the app connects to us, and the relay connects to
        // the provider with our key once the ticket is presented.
        let ticket = refresh::new_raw_token();
        sessions::set_relay_ticket(
            &state.db,
            reserved.id,
            &refresh::hash_token(&ticket),
            &system_prompt,
        )
        .await?;
        tracing::info!(
            session_id = %reserved.id, user_id = %user.id, mode = mode.as_str(),
            provider = primary.name(), max_secs = reserved.max_secs, "relay session reserved"
        );
        let now = Utc::now();
        return Ok((
            StatusCode::CREATED,
            Json(StartSessionResponse {
                session_id: reserved.id,
                mode: mode.as_str().into(),
                provider: primary.name().into(),
                model: primary.model().into(),
                transport: Transport::Relay,
                connect_url: format!("/api/v1/sessions/{}/ws", reserved.id),
                token: ticket,
                token_expires_at: now + chrono::Duration::seconds(sessions::RELAY_TICKET_SECS),
                max_secs: reserved.max_secs,
                session_ends_at: now + chrono::Duration::seconds(i64::from(reserved.max_secs)),
                hard_cutoff: true,
                prompt_version: prompt.version.clone(),
            }),
        ));
    }

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
        (status = 400, description = "Malformed or oversized transcript", body = ErrorBody),
        (status = 404, description = "No such session for this user", body = ErrorBody),
        (status = 409, description = "Session already ended, or it is a relay session", body = ErrorBody)
    )
)]
pub async fn complete_session(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<CompleteSessionRequest>,
) -> AppResult<Json<CompleteSessionResponse>> {
    let transcript = match req.transcript {
        Some(turns) => {
            let chars: usize = turns.iter().map(|t| t.text.chars().count()).sum();
            if turns.len() > MAX_TURNS || chars > MAX_TEXT_CHARS {
                return Err(AppError::BadRequest("transcript is too large".into()));
            }
            Some(serde_json::to_value(turns).map_err(|e| AppError::Internal(e.into()))?)
        }
        None => None,
    };
    let done = sessions::complete(
        &state.db,
        user.id,
        id,
        transcript,
        state.config.voice.usd_per_min,
        EndedBy::Client,
    )
    .await?;
    tracing::info!(session_id = %id, user_id = %user.id, seconds = done.seconds_charged, "session completed");
    Ok(Json(CompleteSessionResponse {
        session_id: id,
        seconds_charged: done.seconds_charged,
        balance_secs: done.balance_secs,
    }))
}

/// Relay mode: upgrade to a WebSocket and pump audio between the app and the
/// provider until the session's time is up. See `crate::relay` for the
/// message protocol. Authenticated by the single-use ticket (browsers and some
/// mobile WebSocket clients cannot send an Authorization header).
#[utoipa::path(
    get, path = "/api/v1/sessions/{id}/ws", tag = "sessions",
    params(("id" = Uuid, Path, description = "Session id"), RelayQuery),
    responses(
        (status = 101, description = "Switched to the relay WebSocket"),
        (status = 401, description = "Ticket missing, wrong, used or expired", body = ErrorBody),
        (status = 503, description = "Voice provider unavailable", body = ErrorBody)
    )
)]
pub async fn relay_socket(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<RelayQuery>,
    ws: WebSocketUpgrade,
) -> AppResult<Response> {
    let ticket = query.ticket.ok_or(AppError::Unauthorized)?;
    // Consumes the ticket: a second connect with it gets 401.
    let opened = sessions::open_relay(&state.db, id, &refresh::hash_token(&ticket)).await?;

    let provider = state.voice.primary();
    if provider.name() != opened.provider {
        // The provider config changed since the session was reserved.
        sessions::mark_failed(&state.db, id).await?;
        return Err(AppError::Unavailable);
    }
    let target = provider.relay_target()?;
    let codec = provider.relay_codec();
    let cfg = SessionConfig {
        system_prompt: opened.system_prompt.unwrap_or_default(),
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(opened.remaining_secs);
    let limits = state.relay_limits.as_ref().clone();
    let user_id = opened.user_id;

    tracing::info!(session_id = %id, %user_id, provider = provider.name(),
        remaining_secs = opened.remaining_secs, "relay opened");
    Ok(ws
        .max_message_size(relay::MAX_CLIENT_MESSAGE)
        .max_frame_size(relay::MAX_CLIENT_MESSAGE)
        .on_upgrade(move |socket| async move {
            let outcome = relay::run(socket, target, codec, cfg, deadline, limits).await;
            finish_relay(&state, id, user_id, outcome).await;
        }))
}

/// Charge and store a finished relay session. Runs after the sockets close,
/// so errors can only be logged.
async fn finish_relay(state: &AppState, id: Uuid, user_id: Uuid, outcome: RelayOutcome) {
    if outcome.reason == EndReason::ProviderUnavailable {
        tracing::warn!(session_id = %id, "relay could not reach the provider; nothing charged");
        if let Err(e) = sessions::mark_failed(&state.db, id).await {
            tracing::error!(session_id = %id, error = ?e, "marking relay session failed");
        }
        return;
    }
    let transcript = serde_json::to_value(&outcome.transcript).ok();
    let ended_by = EndedBy::Relay {
        input_secs: outcome.audio_in_secs.round() as i32,
        output_secs: outcome.audio_out_secs.round() as i32,
    };
    match sessions::complete(
        &state.db,
        user_id,
        id,
        transcript,
        state.config.voice.usd_per_min,
        ended_by,
    )
    .await
    {
        Ok(done) => tracing::info!(
            session_id = %id, %user_id, reason = outcome.reason.as_str(),
            seconds = done.seconds_charged, "relay session completed"
        ),
        Err(e) => tracing::error!(session_id = %id, error = ?e, "completing relay session"),
    }
}
