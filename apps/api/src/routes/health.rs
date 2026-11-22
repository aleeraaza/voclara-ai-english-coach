use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
};

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// "ok" for liveness, "ready" for readiness.
    pub status: String,
    /// Crate version of the running API.
    pub version: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}

/// Liveness: the process is up. Never touches the database.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = 200, description = "Process is alive", body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// Readiness: the API can reach Postgres.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "health",
    responses(
        (status = 200, description = "Database reachable", body = HealthResponse),
        (status = 503, description = "Database unreachable", body = ErrorBody)
    )
)]
pub async fn ready(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "readiness check failed");
            AppError::Unavailable
        })?;
    Ok(Json(HealthResponse {
        status: "ready".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
