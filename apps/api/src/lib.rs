//! Voclara API library. `main.rs` only wires config, the database and the
//! server; everything testable lives here.

pub mod auth;
pub mod config;
pub mod domain;
pub mod error;
pub mod openapi;
pub mod prompts;
pub mod providers;
pub mod routes;
pub mod state;
pub mod telemetry;

use axum::{Router, http::HeaderName, routing::get};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub use state::AppState;

/// Migrations embedded at compile time from `apps/api/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Build the full HTTP application.
pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(routes::health::router())
        .route("/openapi.json", get(openapi::spec))
        .nest("/api/v1", routes::v1())
        // Layers run bottom-up on the request: set id -> trace -> propagate id.
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
        .with_state(state)
}
