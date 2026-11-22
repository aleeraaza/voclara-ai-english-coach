//! HTTP-level tests that need no running database.
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

#[tokio::test]
async fn health_is_ok_without_db() {
    let app = common::build_without_db();
    let (status, body) = app.call(Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn responses_carry_a_request_id() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let app = common::build_without_db();
    let resp = app
        .router
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(resp.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn ready_is_503_when_db_is_down() {
    let app = common::build_without_db();
    let (status, body) = app.call(Method::GET, "/ready", None, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "unavailable");
}

#[tokio::test]
async fn openapi_spec_lists_routes_and_bearer_scheme() {
    let app = common::build_without_db();
    let (status, body) = app.call(Method::GET, "/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["info"]["title"], "Voclara API");
    for path in [
        "/health",
        "/api/v1/auth/otp/send",
        "/api/v1/auth/refresh",
        "/api/v1/me",
    ] {
        assert!(body["paths"][path].is_object(), "{path} missing from spec");
    }
    assert_eq!(
        body["components"]["securitySchemes"]["bearer"]["scheme"],
        "bearer"
    );
}

#[tokio::test]
async fn unknown_route_is_404() {
    let app = common::build_without_db();
    let (status, _) = app.call(Method::GET, "/nope", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn me_requires_a_valid_bearer_token() {
    let app = common::build_without_db();
    let (status, body) = app.call(Method::GET, "/api/v1/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");

    let (status, _) = app
        .call(Method::GET, "/api/v1/me", None, Some("not-a-token"))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bad_phone_is_rejected_before_touching_db() {
    let app = common::build_without_db();
    let (status, body) = app
        .post("/api/v1/auth/otp/send", json!({ "phone": "03001234567" }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn invalid_google_token_is_401() {
    let app = common::build_without_db();
    let (status, _) = app
        .post("/api/v1/auth/google", json!({ "id_token": "garbage" }))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
