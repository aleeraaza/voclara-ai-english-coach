//! Sign-in flows against a real Postgres (skipped when DATABASE_URL is unset).
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

macro_rules! db_app {
    () => {
        match common::test_db().await {
            Some(db) => common::build(db),
            None => return,
        }
    };
}

#[tokio::test]
async fn phone_signup_grants_free_minutes_and_me_works() {
    let app = db_app!();
    let phone = common::random_phone();
    let tokens = app.login_by_phone(&phone).await;
    assert_eq!(tokens["new_user"], true);
    assert_eq!(tokens["token_type"], "Bearer");
    assert_eq!(tokens["expires_in"], 900);

    let access = tokens["access_token"].as_str().unwrap();
    let (status, me) = app
        .call(Method::GET, "/api/v1/me", None, Some(access))
        .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["phone"], phone);
    assert_eq!(me["l1"], "ur");
    assert_eq!(me["plan"], "free");
    assert_eq!(me["minutes"]["balance_secs"], 600);
    assert_eq!(me["minutes"]["daily_drill_secs_left"], 120);
    assert_eq!(me["streak"]["current_days"], 0);

    // Signing in again is not a new user and does not grant more minutes.
    // Age the previous code so the 30-second resend limit does not block us.
    sqlx::query("UPDATE otp_codes SET created_at = now() - interval '1 minute' WHERE phone = $1")
        .bind(&phone)
        .execute(&app.db)
        .await
        .unwrap();
    let again = app.login_by_phone(&phone).await;
    assert_eq!(again["new_user"], false);
    assert_eq!(again["user_id"], tokens["user_id"]);
    let access = again["access_token"].as_str().unwrap();
    let (_, me) = app
        .call(Method::GET, "/api/v1/me", None, Some(access))
        .await;
    assert_eq!(me["minutes"]["balance_secs"], 600);
}

#[tokio::test]
async fn patch_me_updates_settings_and_validates() {
    let app = db_app!();
    let tokens = app.login_by_phone(&common::random_phone()).await;
    let access = tokens["access_token"].as_str().unwrap();

    let (status, me) = app
        .call(
            Method::PATCH,
            "/api/v1/me",
            Some(json!({ "l1": "HI", "level": "b2", "goal": "ielts", "name": " Ayesha " })),
            Some(access),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["l1"], "hi");
    assert_eq!(me["level"], "b2");
    assert_eq!(me["goal"], "ielts");
    assert_eq!(me["name"], "Ayesha");

    let (status, body) = app
        .call(
            Method::PATCH,
            "/api/v1/me",
            Some(json!({ "l1": "fr" })),
            Some(access),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn resend_is_rate_limited() {
    let app = db_app!();
    let phone = common::random_phone();
    let (s, _) = app
        .post("/api/v1/auth/otp/send", json!({ "phone": phone }))
        .await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let (s, body) = app
        .post("/api/v1/auth/otp/send", json!({ "phone": phone }))
        .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"]["code"], "rate_limited");
}

#[tokio::test]
async fn wrong_codes_lock_the_code_after_five_attempts() {
    let app = db_app!();
    let phone = common::random_phone();
    app.post("/api/v1/auth/otp/send", json!({ "phone": phone }))
        .await;
    let real = app.otp.last_code(&phone);
    let wrong = if real == "000000" { "111111" } else { "000000" };

    for _ in 0..5 {
        let (s, body) = app
            .post(
                "/api/v1/auth/otp/verify",
                json!({ "phone": phone, "code": wrong }),
            )
            .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "invalid_code");
    }
    // Even the right code is refused now.
    let (s, _) = app
        .post(
            "/api/v1/auth/otp/verify",
            json!({ "phone": phone, "code": real }),
        )
        .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn a_code_works_only_once() {
    let app = db_app!();
    let phone = common::random_phone();
    app.post("/api/v1/auth/otp/send", json!({ "phone": phone }))
        .await;
    let code = app.otp.last_code(&phone);
    let body = json!({ "phone": phone, "code": code });
    let (s, _) = app.post("/api/v1/auth/otp/verify", body.clone()).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = app.post("/api/v1/auth/otp/verify", body).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn expired_code_is_rejected() {
    let app = db_app!();
    let phone = common::random_phone();
    app.post("/api/v1/auth/otp/send", json!({ "phone": phone }))
        .await;
    let code = app.otp.last_code(&phone);
    sqlx::query("UPDATE otp_codes SET expires_at = now() - interval '1 second' WHERE phone = $1")
        .bind(&phone)
        .execute(&app.db)
        .await
        .unwrap();
    let (s, body) = app
        .post(
            "/api/v1/auth/otp/verify",
            json!({ "phone": phone, "code": code }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_code");
}

#[tokio::test]
async fn refresh_rotates_and_reuse_revokes_the_family() {
    let app = db_app!();
    let login = app.login_by_phone(&common::random_phone()).await;
    let r1 = login["refresh_token"].as_str().unwrap().to_owned();

    // r1 -> r2: works, and returns a different token.
    let (s, rotated) = app
        .post("/api/v1/auth/refresh", json!({ "refresh_token": r1 }))
        .await;
    assert_eq!(s, StatusCode::OK);
    let r2 = rotated["refresh_token"].as_str().unwrap().to_owned();
    assert_ne!(r1, r2);
    assert_eq!(rotated["user_id"], login["user_id"]);
    let access = rotated["access_token"].as_str().unwrap();
    let (s, _) = app
        .call(Method::GET, "/api/v1/me", None, Some(access))
        .await;
    assert_eq!(s, StatusCode::OK);

    // Replaying r1 is treated as theft: 401, and r2 dies with it.
    let (s, _) = app
        .post("/api/v1/auth/refresh", json!({ "refresh_token": r1 }))
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = app
        .post("/api/v1/auth/refresh", json!({ "refresh_token": r2 }))
        .await;
    assert_eq!(
        s,
        StatusCode::UNAUTHORIZED,
        "family should be revoked after reuse"
    );
}

#[tokio::test]
async fn other_devices_survive_a_reuse_on_one_device() {
    let app = db_app!();
    let phone = common::random_phone();
    let phone_login = app.login_by_phone(&phone).await;
    sqlx::query("UPDATE otp_codes SET created_at = now() - interval '1 minute' WHERE phone = $1")
        .bind(&phone)
        .execute(&app.db)
        .await
        .unwrap();
    let web_login = app.login_by_phone(&phone).await; // second device = second family

    let a1 = phone_login["refresh_token"].as_str().unwrap();
    app.post("/api/v1/auth/refresh", json!({ "refresh_token": a1 }))
        .await;
    app.post("/api/v1/auth/refresh", json!({ "refresh_token": a1 }))
        .await; // reuse

    let b1 = web_login["refresh_token"].as_str().unwrap();
    let (s, _) = app
        .post("/api/v1/auth/refresh", json!({ "refresh_token": b1 }))
        .await;
    assert_eq!(s, StatusCode::OK, "the other family must be untouched");
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let app = db_app!();
    let login = app.login_by_phone(&common::random_phone()).await;
    let r = login["refresh_token"].as_str().unwrap();
    let (s, _) = app
        .post("/api/v1/auth/logout", json!({ "refresh_token": r }))
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = app
        .post("/api/v1/auth/refresh", json!({ "refresh_token": r }))
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_refresh_token_is_401() {
    let app = db_app!();
    let (s, _) = app
        .post(
            "/api/v1/auth/refresh",
            json!({ "refresh_token": "made-up" }),
        )
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn google_signup_then_signin_is_same_user() {
    let app = db_app!();
    let sub = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("{sub}@example.com");
    let token = format!("google:{sub}:{email}:true");

    let (s, first) = app
        .post("/api/v1/auth/google", json!({ "id_token": token }))
        .await;
    assert_eq!(s, StatusCode::OK, "{first}");
    assert_eq!(first["new_user"], true);
    let (s, second) = app
        .post("/api/v1/auth/google", json!({ "id_token": token }))
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(second["new_user"], false);
    assert_eq!(second["user_id"], first["user_id"]);

    let access = second["access_token"].as_str().unwrap();
    let (_, me) = app
        .call(Method::GET, "/api/v1/me", None, Some(access))
        .await;
    assert_eq!(me["email"], email);
    assert_eq!(me["minutes"]["balance_secs"], 600);
}

#[tokio::test]
async fn google_links_to_existing_account_with_same_verified_email() {
    let app = db_app!();
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());
    let user_id: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO users (phone, email) VALUES ($1, $2) RETURNING id")
            .bind(common::random_phone())
            .bind(&email)
            .fetch_one(&app.db)
            .await
            .unwrap();

    let token = format!(
        "google:{}:{}:true",
        uuid::Uuid::new_v4().simple(),
        email.to_uppercase()
    );
    let (s, body) = app
        .post("/api/v1/auth/google", json!({ "id_token": token }))
        .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["user_id"], user_id.to_string());
    assert_eq!(body["new_user"], false);
}

#[tokio::test]
async fn unverified_google_email_does_not_link() {
    let app = db_app!();
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());
    let victim: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO users (phone, email) VALUES ($1, $2) RETURNING id")
            .bind(common::random_phone())
            .bind(&email)
            .fetch_one(&app.db)
            .await
            .unwrap();

    let token = format!("google:{}:{email}:false", uuid::Uuid::new_v4().simple());
    let (s, body) = app
        .post("/api/v1/auth/google", json!({ "id_token": token }))
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(
        body["user_id"],
        victim.to_string(),
        "must not take over the account"
    );
    assert_eq!(body["new_user"], true);
}
