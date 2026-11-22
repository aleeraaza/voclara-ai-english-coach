//! The signed-in user's profile, wallet and settings.

use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    domain::{minutes, sessions, users},
    error::{AppJson, AppResult, ErrorBody},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/me", get(get_me).patch(update_me))
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub id: Uuid,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    /// Explanation language: ur | hi | bn | ar | en.
    pub l1: String,
    pub level: Option<String>,
    pub goal: Option<String>,
    pub country: Option<String>,
    pub timezone: String,
    /// Active plan from entitlements, or "free".
    pub plan: String,
    pub minutes: MinutesInfo,
    pub streak: StreakInfo,
}

#[derive(Serialize, ToSchema)]
pub struct MinutesInfo {
    /// Live conversation seconds left in the wallet.
    pub balance_secs: i64,
    /// Free drill seconds left today.
    pub daily_drill_secs_left: i64,
}

#[derive(Serialize, ToSchema)]
pub struct StreakInfo {
    pub current_days: i32,
    pub best_days: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateMeRequest {
    pub name: Option<String>,
    /// ur | hi | bn | ar | en
    pub l1: Option<String>,
    /// a1 | a2 | b1 | b2 | c1 | c2
    pub level: Option<String>,
    /// ielts | interview | work | abroad | daily
    pub goal: Option<String>,
}

async fn load(state: &AppState, id: Uuid) -> AppResult<MeResponse> {
    let user = users::get(&state.db, id).await?;
    // Charge any abandoned session first so the balance shown is the real one.
    sessions::settle_abandoned_for(&state.db, id, state.config.voice.usd_per_min).await?;
    let plan = users::current_plan(&state.db, id).await?;
    let balance_secs = minutes::balance(&state.db, id).await?;
    let daily_drill_secs_left = minutes::drill_secs_left_today(&state.db, id).await?;
    let (current_days, best_days) = users::streak(&state.db, id).await?;
    Ok(MeResponse {
        id: user.id,
        phone: user.phone,
        email: user.email,
        name: user.name,
        l1: user.l1,
        level: user.level,
        goal: user.goal,
        country: user.country,
        timezone: user.timezone,
        plan,
        minutes: MinutesInfo {
            balance_secs,
            daily_drill_secs_left,
        },
        streak: StreakInfo {
            current_days,
            best_days,
        },
    })
}

/// Profile, plan, minutes and streak of the signed-in user.
#[utoipa::path(
    get, path = "/api/v1/me", tag = "me",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current user", body = MeResponse),
        (status = 401, description = "Missing or invalid access token", body = ErrorBody)
    )
)]
pub async fn get_me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<MeResponse>> {
    Ok(Json(load(&state, user.id).await?))
}

/// Update name, explanation language, level or goal. Omitted fields are unchanged.
#[utoipa::path(
    patch, path = "/api/v1/me", tag = "me",
    security(("bearer" = [])),
    request_body = UpdateMeRequest,
    responses(
        (status = 200, description = "Updated user", body = MeResponse),
        (status = 400, description = "Invalid value", body = ErrorBody),
        (status = 401, description = "Missing or invalid access token", body = ErrorBody)
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    AppJson(req): AppJson<UpdateMeRequest>,
) -> AppResult<Json<MeResponse>> {
    let mut settings = users::Settings {
        name: req.name,
        l1: req.l1,
        level: req.level,
        goal: req.goal,
    };
    settings.validate()?;
    users::update_settings(&state.db, user.id, &settings).await?;
    Ok(Json(load(&state, user.id).await?))
}
