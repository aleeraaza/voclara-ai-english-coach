//! Live session start/complete against a real Postgres (skipped without TEST_DATABASE_URL).
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;

macro_rules! db_app {
    ($fallback:expr) => {
        match common::test_db().await {
            Some(db) => common::build_with(db, $fallback),
            None => return,
        }
    };
}

/// Sign up a fresh user (600 free seconds); returns (access token, user id).
async fn sign_in(app: &common::TestApp) -> (String, Uuid) {
    let t = app.login_by_phone(&common::random_phone()).await;
    let id = t["user_id"].as_str().unwrap().parse().unwrap();
    (t["access_token"].as_str().unwrap().to_owned(), id)
}

async fn start(app: &common::TestApp, token: &str, body: Value) -> (StatusCode, Value) {
    app.call(Method::POST, "/api/v1/sessions", Some(body), Some(token))
        .await
}

async fn complete(
    app: &common::TestApp,
    token: &str,
    id: &str,
    body: Value,
) -> (StatusCode, Value) {
    app.call(
        Method::POST,
        &format!("/api/v1/sessions/{id}/complete"),
        Some(body),
        Some(token),
    )
    .await
}

async fn age_session(app: &common::TestApp, id: &str, secs: i32) {
    sqlx::query("UPDATE sessions SET started_at = now() - make_interval(secs => $2) WHERE id = $1")
        .bind(Uuid::parse_str(id).unwrap())
        .bind(secs)
        .execute(&app.db)
        .await
        .unwrap();
}

async fn balance(app: &common::TestApp, token: &str) -> i64 {
    let (_, me) = app.call(Method::GET, "/api/v1/me", None, Some(token)).await;
    me["minutes"]["balance_secs"].as_i64().unwrap()
}

#[tokio::test]
async fn talk_session_gets_token_limited_to_mode_and_balance() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (s, body) = start(
        &app,
        &token,
        json!({ "mode": "talk", "topic": "my village" }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(body["mode"], "talk");
    assert_eq!(body["provider"], "primary");
    assert_eq!(body["token"], "primary-token");
    assert_eq!(body["transport"], "websocket");
    assert_eq!(body["max_secs"], 600, "talk limit 600 = free balance 600");
    assert_eq!(body["prompt_version"], "tutor-v1");
    assert_eq!(app.voice.last_ttl(), 600);

    let prompt = app.voice.last_prompt();
    assert!(prompt.contains("Topic: my village."), "{prompt}");
    assert!(prompt.contains("Level: b1."), "default level");
}

#[tokio::test]
async fn prompt_uses_the_learners_level_and_goal() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    app.call(
        Method::PATCH,
        "/api/v1/me",
        Some(json!({ "level": "c1", "goal": "ielts" })),
        Some(&token),
    )
    .await;
    start(&app, &token, json!({ "mode": "talk" })).await;
    let prompt = app.voice.last_prompt();
    assert!(
        prompt.contains("Level: c1. Goal: ielts. Topic: everyday life."),
        "{prompt}"
    );
}

#[tokio::test]
async fn mock_is_capped_by_balance_not_just_mode_limit() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (s, body) = start(&app, &token, json!({ "mode": "mock" })).await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(
        body["max_secs"], 600,
        "mock limit is 900 but only 600 s are left"
    );
    assert_eq!(body["prompt_version"], "examiner-v2");
}

#[tokio::test]
async fn interview_requires_a_known_role() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;

    let (s, body) = start(&app, &token, json!({ "mode": "interview" })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{body}");
    let (s, _) = start(
        &app,
        &token,
        json!({ "mode": "interview", "topic": "astronaut" }),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, body) = start(
        &app,
        &token,
        json!({ "mode": "interview", "topic": "nursing" }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(body["prompt_version"], "interviewer-v1");
    assert!(app.voice.last_prompt().contains("nursing job"));
}

#[tokio::test]
async fn bad_mode_and_missing_auth_are_rejected() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (s, body) = start(&app, &token, json!({ "mode": "karaoke" })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");

    let (s, _) = app
        .call(
            Method::POST,
            "/api/v1/sessions",
            Some(json!({ "mode": "talk" })),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn only_one_session_at_a_time() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (s, _) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, body) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::CONFLICT, "{body}");
}

#[tokio::test]
async fn complete_charges_elapsed_time_and_stores_transcript() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (_, started) = start(&app, &token, json!({ "mode": "talk" })).await;
    let id = started["session_id"].as_str().unwrap();
    age_session(&app, id, 125).await; // pretend 125 s have passed

    let transcript = json!([
        { "role": "tutor", "text": "Hi! Tell me about your day.", "t_ms": 0 },
        { "role": "learner", "text": "I go to office yesterday.", "t_ms": 2100 }
    ]);
    let (s, done) = complete(&app, &token, id, json!({ "transcript": transcript })).await;
    assert_eq!(s, StatusCode::OK, "{done}");
    let charged = done["seconds_charged"].as_i64().unwrap();
    assert!((125..=127).contains(&charged), "charged {charged}");
    assert_eq!(done["balance_secs"].as_i64().unwrap(), 600 - charged);
    assert_eq!(balance(&app, &token).await, 600 - charged);

    let (status, stored, cost): (String, Value, f64) = sqlx::query_as(
        "SELECT s.status, s.transcript, c.usd::float8 FROM sessions s
         JOIN session_costs c ON c.session_id = s.id WHERE s.id = $1",
    )
    .bind(Uuid::parse_str(id).unwrap())
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(status, "completed");
    assert_eq!(stored[1]["text"], "I go to office yesterday.");
    assert!(
        (cost - charged as f64 / 60.0 * 0.03).abs() < 1e-6,
        "cost {cost}"
    );
}

#[tokio::test]
async fn charge_never_exceeds_the_session_limit() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (_, started) = start(&app, &token, json!({ "mode": "talk" })).await;
    let id = started["session_id"].as_str().unwrap();
    age_session(&app, id, 650).await; // past the 600 s limit, within grace
    let (s, done) = complete(&app, &token, id, json!({})).await;
    assert_eq!(s, StatusCode::OK, "{done}");
    assert_eq!(done["seconds_charged"], 600);
    assert_eq!(done["balance_secs"], 0);
}

#[tokio::test]
async fn completing_twice_or_someone_elses_session_fails() {
    let app = db_app!(false);
    let (alice, _) = sign_in(&app).await;
    let (bob, _) = sign_in(&app).await;
    let (_, started) = start(&app, &alice, json!({ "mode": "talk" })).await;
    let id = started["session_id"].as_str().unwrap();

    let (s, _) = complete(&app, &bob, id, json!({})).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "bob cannot end alice's session");

    let (s, _) = complete(&app, &alice, id, json!({})).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = complete(&app, &alice, id, json!({})).await;
    assert_eq!(s, StatusCode::CONFLICT, "no double completion");

    let (s, _) = complete(&app, &alice, &Uuid::new_v4().to_string(), json!({})).await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, body) = complete(&app, &alice, id, json!({ "transcript": "not a list" })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{body}");
    let bad_role = json!({ "transcript": [{ "role": "narrator", "text": "x", "t_ms": 0 }] });
    let (s, body) = complete(&app, &alice, id, bad_role).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn abandoned_session_is_charged_in_full_when_the_next_starts() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (_, first) = start(&app, &token, json!({ "mode": "talk" })).await;
    let first_id = first["session_id"].as_str().unwrap();
    // Started 20 minutes ago, never completed: limit 600 s + 120 s grace has passed.
    age_session(&app, first_id, 1200).await;

    let (s, body) = start(&app, &token, json!({ "mode": "talk" })).await;
    // The abandoned session used all 600 free seconds, so nothing is left.
    assert_eq!(s, StatusCode::PAYMENT_REQUIRED, "{body}");
    let status: String = sqlx::query_scalar("SELECT status FROM sessions WHERE id = $1")
        .bind(Uuid::parse_str(first_id).unwrap())
        .fetch_one(&app.db)
        .await
        .unwrap();
    assert_eq!(status, "expired");
    assert_eq!(balance(&app, &token).await, 0);
}

#[tokio::test]
async fn me_shows_the_balance_after_an_abandoned_session() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (_, first) = start(&app, &token, json!({ "mode": "talk" })).await;
    age_session(&app, first["session_id"].as_str().unwrap(), 1200).await;
    // No new start: just looking at the profile settles the stale session.
    assert_eq!(balance(&app, &token).await, 0);
    // And a refused start afterwards stays refused, not stuck on a ghost session.
    let (s, _) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::PAYMENT_REQUIRED);
}

#[tokio::test]
async fn too_little_balance_is_402() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let (_, started) = start(&app, &token, json!({ "mode": "talk" })).await;
    let id = started["session_id"].as_str().unwrap();
    age_session(&app, id, 580).await;
    complete(&app, &token, id, json!({})).await; // ~20 s left, below the 30 s minimum

    let (s, body) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(body["error"]["code"], "insufficient_minutes");
}

#[tokio::test]
async fn provider_outage_frees_the_slot_and_charges_nothing() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    app.voice.set_failing(true);
    let (s, body) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "{body}");

    app.voice.set_failing(false);
    let (s, body) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(
        s,
        StatusCode::CREATED,
        "the failed session must not block: {body}"
    );
    assert_eq!(balance(&app, &token).await, 600);
}

#[tokio::test]
async fn fallback_provider_serves_when_primary_is_down() {
    let app = db_app!(true);
    let (token, _) = sign_in(&app).await;
    app.voice.set_failing(true);
    let (s, body) = start(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(body["provider"], "backup");
    assert_eq!(body["token"], "backup-token");

    let provider: String = sqlx::query_scalar("SELECT provider FROM sessions WHERE id = $1")
        .bind(Uuid::parse_str(body["session_id"].as_str().unwrap()).unwrap())
        .fetch_one(&app.db)
        .await
        .unwrap();
    assert_eq!(
        provider, "backup",
        "the session records who really served it"
    );
}

/// Two "start" requests racing must not both get a session (and the same minutes).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_starts_yield_one_session() {
    let app = db_app!(false);
    let (token, _) = sign_in(&app).await;
    let app = std::sync::Arc::new(app);
    let mut handles = Vec::new();
    for _ in 0..5 {
        let app = app.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            start(&app, &token, json!({ "mode": "talk" })).await.0
        }));
    }
    let mut created = 0;
    for h in handles {
        match h.await.unwrap() {
            StatusCode::CREATED => created += 1,
            StatusCode::CONFLICT => {}
            other => panic!("unexpected status {other}"),
        }
    }
    assert_eq!(created, 1);
}
