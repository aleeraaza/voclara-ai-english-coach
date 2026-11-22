//! HTTP-level tests that need no running database.
// Test helpers may unwrap (CLAUDE.md allows unwrap in tests); clippy's
// allow-unwrap-in-tests only covers #[test] fns, not their helpers.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use voclara_api::{
    AppState,
    config::{Config, LogFormat},
};

/// App whose pool points at a port nothing listens on. `connect_lazy` makes no
/// connection until a query runs, so routes that skip the DB work normally.
fn app_without_db() -> axum::Router {
    let url = "postgres://voclara:voclara@127.0.0.1:1/voclara";
    let db = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy(url)
        .unwrap();
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        database_url: url.into(),
        db_max_connections: 1,
        run_migrations: false,
        log_format: LogFormat::Pretty,
    };
    voclara_api::app(AppState::new(db, config))
}

async fn get(path: &str) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
    let resp = app_without_db()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, headers, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn health_is_ok_without_db() {
    let (status, headers, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(headers.contains_key("x-request-id"));
}

#[tokio::test]
async fn ready_is_503_when_db_is_down() {
    let (status, _, body) = get("/ready").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "unavailable");
}

#[tokio::test]
async fn openapi_spec_lists_health() {
    let (status, _, body) = get("/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["info"]["title"], "Voclara API");
    assert!(body["paths"]["/health"].is_object());
}

#[tokio::test]
async fn unknown_route_is_404() {
    let resp = app_without_db()
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
