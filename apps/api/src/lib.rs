//! Voclara API library. `main.rs` only wires config, the database and the
//! server; everything testable lives here.

pub mod auth;
pub mod config;
pub mod content;
pub mod domain;
pub mod error;
pub mod openapi;
pub mod prompts;
pub mod providers;
pub mod relay;
pub mod routes;
pub mod share_card;
pub mod state;
pub mod telemetry;
pub mod worker;

use axum::{
    Router,
    http::{HeaderName, Request},
    routing::get,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub use state::AppState;

/// Migrations embedded at compile time from `apps/api/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Browsers may call the API only from the configured origins (the web app).
/// Native apps send no Origin header, so CORS does not affect them.
fn cors(origins: &[String]) -> CorsLayer {
    use axum::http::{HeaderValue, Method, header};
    let origins: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Build the full HTTP application.
pub fn app(state: AppState) -> Router {
    let cors = cors(&state.config.cors_origins);
    Router::new()
        .merge(routes::health::router())
        .route("/openapi.json", get(openapi::spec))
        .nest("/api/v1", routes::v1())
        // Layers run bottom-up on the request: set id -> trace -> propagate id.
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        // Spans log the path only, never the query string: relay tickets
        // (and any future tokens in URLs) must not end up in logs.
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &Request<_>| {
                tracing::info_span!("http", method = %req.method(), path = %req.uri().path())
            }),
        )
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
        .layer(cors)
        .with_state(state)
}
