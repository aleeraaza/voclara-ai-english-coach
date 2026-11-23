//! IELTS-style mock flow against a real Postgres (skipped without TEST_DATABASE_URL):
//! cue card selection, the scripted examiner prompt, mock credits, feedback
//! with the cue card, and the share card image.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use voclara_api::content::Content;

/// Tests that grade sessions drain the shared job queue: one at a time.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

macro_rules! db_app {
    () => {
        match common::test_db().await {
            Some(db) => common::build(db),
            None => return,
        }
    };
}

/// A user with `secs` of practice time; returns (access token, user id).
async fn user(app: &common::TestApp, secs: i32) -> (String, Uuid) {
    let id: Uuid = sqlx::query_scalar("INSERT INTO users (phone) VALUES ($1) RETURNING id")
        .bind(common::random_phone())
        .fetch_one(&app.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO minute_ledger (user_id, delta_secs, reason) VALUES ($1, $2, 'grant_free')",
    )
    .bind(id)
    .bind(secs)
    .execute(&app.db)
    .await
    .unwrap();
    (common::access_token_for(id), id)
}

async fn start_mock(app: &common::TestApp, token: &str, body: Value) -> (StatusCode, Value) {
    app.call(Method::POST, "/api/v1/sessions", Some(body), Some(token))
        .await
}

async fn age(app: &common::TestApp, id: &str, secs: i32) {
    sqlx::query("UPDATE sessions SET started_at = now() - make_interval(secs => $2) WHERE id = $1")
        .bind(Uuid::parse_str(id).unwrap())
        .bind(secs)
        .execute(&app.db)
        .await
        .unwrap();
}

async fn complete(app: &common::TestApp, token: &str, id: &str, transcript: Value) {
    let (s, body) = app
        .call(
            Method::POST,
            &format!("/api/v1/sessions/{id}/complete"),
            Some(json!({ "transcript": transcript })),
            Some(token),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{body}");
}

fn answers() -> Value {
    let lines = [
        "I am working as a bank officer in Lahore and I enjoy helping customers every day.",
        "The journey I remember is a train trip to Karachi with my cousins two years ago.",
        "It was memorable because we talked the whole night and watched the sunrise together.",
    ];
    let mut turns = vec![];
    for (i, line) in lines.iter().enumerate() {
        let t = i as u64 * 15_000;
        turns.push(json!({ "role": "tutor", "text": "Thank you. Next question.", "t_ms": t }));
        turns.push(json!({ "role": "learner", "text": line, "t_ms": t + 3_000 }));
    }
    Value::Array(turns)
}

async fn mocks_left(app: &common::TestApp, user_id: Uuid) -> i32 {
    sqlx::query_scalar("SELECT mocks_left FROM entitlements WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&app.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn mock_start_returns_a_cue_card_and_a_scripted_examiner() {
    let app = db_app!();
    let content = Content::load().unwrap();
    let (token, _) = user(&app, 900).await;

    let (s, body) = start_mock(&app, &token, json!({ "mode": "mock" })).await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(body["prompt_version"], "examiner-v2");
    assert_eq!(body["max_secs"], 900);

    let card_id = body["cue_card"]["id"].as_str().unwrap();
    let card = content.cue_card(card_id).expect("a card from the bank");
    assert_eq!(body["cue_card"]["prompt"], card.prompt.as_str());
    assert_eq!(
        body["cue_card"]["bullets"].as_array().unwrap().len(),
        card.bullets.len()
    );
    // Part 1 and Part 3 questions stay on the server.
    assert!(body["cue_card"].get("part1").is_none());
    assert!(body["cue_card"].get("part3").is_none());

    // The examiner gets the whole script for this card, with the app's signals.
    let prompt = app.voice.last_prompt();
    assert!(prompt.contains(&card.prompt), "{prompt}");
    assert!(prompt.contains(&card.part1[0]) && prompt.contains(&card.part3[0]));
    assert!(prompt.contains(&card.rounding_off[0]));
    assert!(prompt.contains("Now, Part 2.") && prompt.contains("Now, Part 3."));
    assert!(!prompt.contains('{'), "unfilled placeholder: {prompt}");

    let (stored, topic): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT cue_card_id, topic FROM sessions WHERE id = $1")
            .bind(Uuid::parse_str(body["session_id"].as_str().unwrap()).unwrap())
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert_eq!(stored.as_deref(), Some(card_id));
    assert_eq!(topic.as_deref(), Some(card.topic.as_str()));
}

#[tokio::test]
async fn a_chosen_cue_card_is_used_and_unknown_ones_are_rejected() {
    let app = db_app!();
    let (token, _) = user(&app, 900).await;

    let (s, body) = start_mock(
        &app,
        &token,
        json!({ "mode": "mock", "cue_card_id": "nope-not-a-card" }),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{body}");

    let (s, body) = start_mock(
        &app,
        &token,
        json!({ "mode": "mock", "cue_card_id": "describe-a-journey" }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(body["cue_card"]["id"], "describe-a-journey");
    assert!(
        app.voice
            .last_prompt()
            .contains("Describe a memorable journey")
    );
}

#[tokio::test]
async fn talk_and_interview_get_no_cue_card() {
    let app = db_app!();
    let (token, _) = user(&app, 900).await;
    let (s, body) = start_mock(&app, &token, json!({ "mode": "talk" })).await;
    assert_eq!(s, StatusCode::CREATED);
    assert!(body["cue_card"].is_null(), "{body}");
}

#[tokio::test]
async fn new_mocks_avoid_recently_used_cards() {
    let app = db_app!();
    let content = Content::load().unwrap();
    let (token, user_id) = user(&app, 900).await;

    // History: every card but one was used in a recent mock.
    let all: Vec<&str> = content.cue_cards().iter().map(|c| c.id.as_str()).collect();
    let (left, used) = all.split_last().unwrap();
    for (i, id) in used.iter().enumerate() {
        sqlx::query(
            "INSERT INTO sessions (user_id, mode, provider, prompt_version, status, max_secs,
                                   cue_card_id, started_at)
             VALUES ($1, 'mock', 'primary', 'examiner-v2', 'completed', 900, $2,
                     now() - make_interval(mins => $3))",
        )
        .bind(user_id)
        .bind(id)
        .bind(i as i32 + 1)
        .execute(&app.db)
        .await
        .unwrap();
    }
    assert!(used.len() as i64 <= 40, "history fits the look-back window");

    let (s, body) = start_mock(&app, &token, json!({ "mode": "mock" })).await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(
        body["cue_card"]["id"], *left,
        "the only card not used recently"
    );
}

#[tokio::test]
async fn a_full_mock_uses_one_mock_credit_and_a_short_one_none() {
    let app = db_app!();
    let (token, user_id) = user(&app, 3_000).await;
    sqlx::query("INSERT INTO entitlements (user_id, plan, mocks_left) VALUES ($1, 'mock_pack', 2)")
        .bind(user_id)
        .execute(&app.db)
        .await
        .unwrap();

    // Dropped after 30 s: an abandoned attempt, not a mock taken.
    let (_, short) = start_mock(&app, &token, json!({ "mode": "mock" })).await;
    let id = short["session_id"].as_str().unwrap();
    age(&app, id, 30).await;
    complete(&app, &token, id, json!([])).await;
    assert_eq!(mocks_left(&app, user_id).await, 2);

    // A real mock (5 minutes) uses one credit.
    let (_, full) = start_mock(&app, &token, json!({ "mode": "mock" })).await;
    let id = full["session_id"].as_str().unwrap();
    age(&app, id, 300).await;
    complete(&app, &token, id, json!([])).await;
    assert_eq!(mocks_left(&app, user_id).await, 1);

    // Talk sessions never touch mock credits.
    let (_, talk) = start_mock(&app, &token, json!({ "mode": "talk" })).await;
    let id = talk["session_id"].as_str().unwrap();
    age(&app, id, 300).await;
    complete(&app, &token, id, json!([])).await;
    assert_eq!(mocks_left(&app, user_id).await, 1);
}

#[tokio::test]
async fn graded_mock_feedback_shows_its_cue_card_and_shares_as_png() {
    let app = db_app!();
    let _serial = SERIAL.lock().await;
    app.run_jobs().await;
    let (token, _) = user(&app, 900).await;

    let (_, started) = start_mock(
        &app,
        &token,
        json!({ "mode": "mock", "cue_card_id": "describe-a-journey" }),
    )
    .await;
    let id = started["session_id"].as_str().unwrap().to_owned();
    age(&app, &id, 240).await;
    complete(&app, &token, &id, answers()).await;

    // Not graded yet: no card to share.
    let (status, _, _) = share_card(&app, &token, &id).await;
    assert_eq!(status, StatusCode::CONFLICT);

    app.run_jobs().await;
    let (s, fb) = app
        .call(
            Method::GET,
            &format!("/api/v1/sessions/{id}/feedback"),
            None,
            Some(&token),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{fb}");
    assert_eq!(fb["status"], "ready", "{fb}");
    assert_eq!(fb["mode"], "mock");
    assert_eq!(fb["cue_card"]["id"], "describe-a-journey");
    assert_eq!(fb["cue_card"]["topic"], "Travel");

    let (status, content_type, png) = share_card(&app, &token, &id).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "image/png");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

    // Someone else's session is not found, not leaked.
    let (other, _) = user(&app, 600).await;
    let (status, _, _) = share_card(&app, &other, &id).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn share_card(app: &common::TestApp, token: &str, id: &str) -> (StatusCode, String, Vec<u8>) {
    let resp = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/sessions/{id}/share-card.png"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, content_type, bytes)
}
