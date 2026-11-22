//! Grading pipeline against a real Postgres (skipped without DATABASE_URL):
//! complete -> grade_session job -> scores -> feedback / progress.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;
use voclara_api::domain::jobs;

/// Tests here share one job queue and `run_jobs` drains all of it, so they
/// run one at a time (in production any worker may take any job, as intended).
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

macro_rules! db_app {
    () => {
        match common::test_db().await {
            Some(db) => {
                let serial = SERIAL.lock().await;
                let app = common::build(db);
                // Start from an empty queue: other test files leave graded-job
                // work behind (they complete sessions too).
                app.run_jobs().await;
                (serial, app)
            }
            None => return,
        }
    };
}

/// A user with 600 free seconds; returns (access token, user id).
async fn user(app: &common::TestApp, l1: &str) -> (String, Uuid) {
    let id: Uuid = sqlx::query_scalar("INSERT INTO users (phone, l1) VALUES ($1, $2) RETURNING id")
        .bind(common::random_phone())
        .bind(l1)
        .fetch_one(&app.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO minute_ledger (user_id, delta_secs, reason) VALUES ($1, 600, 'grant_free')",
    )
    .bind(id)
    .execute(&app.db)
    .await
    .unwrap();
    (common::access_token_for(id), id)
}

fn transcript(learner_lines: &[&str]) -> Value {
    let mut turns = vec![];
    let mut t = 0u64;
    for line in learner_lines {
        turns.push(json!({ "role": "tutor", "text": "Tell me more about that.", "t_ms": t }));
        t += 3_000;
        turns.push(json!({ "role": "learner", "text": line, "t_ms": t }));
        t += 12_000;
    }
    turns.push(json!({ "role": "tutor", "text": "Thank you!", "t_ms": t }));
    Value::Array(turns)
}

const ENOUGH: &[&str] = &[
    "Um I go to office yesterday and it was very much big day for me, you know.",
    "My manager give me new project, basically about the bank customers and their accounts.",
    "I am working there since two years and I like my team very much because they are helpful.",
];

/// Start and complete a direct session with `turns`; returns the session id.
async fn finished_session(app: &common::TestApp, token: &str, turns: Value) -> String {
    let (s, started) = app
        .call(
            Method::POST,
            "/api/v1/sessions",
            Some(json!({ "mode": "talk" })),
            Some(token),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{started}");
    let id = started["session_id"].as_str().unwrap().to_owned();
    let (s, done) = app
        .call(
            Method::POST,
            &format!("/api/v1/sessions/{id}/complete"),
            Some(json!({ "transcript": turns })),
            Some(token),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{done}");
    id
}

async fn feedback(app: &common::TestApp, token: &str, id: &str) -> Value {
    let (s, body) = app
        .call(
            Method::GET,
            &format!("/api/v1/sessions/{id}/feedback"),
            None,
            Some(token),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
async fn completed_session_is_graded_into_feedback() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    let id = finished_session(&app, &token, transcript(ENOUGH)).await;

    let before = feedback(&app, &token, &id).await;
    assert_eq!(before["status"], "pending");
    assert!(before["bands"].is_null());

    assert!(app.run_jobs().await >= 1);
    let fb = feedback(&app, &token, &id).await;
    assert_eq!(fb["status"], "ready", "{fb}");
    assert_eq!(fb["demo"], false);

    // Snapped down to 0.5 steps; pronunciation ignored without evidence.
    assert_eq!(fb["bands"]["fluency"], 6.0);
    assert_eq!(fb["bands"]["lexical"], 6.5);
    assert_eq!(fb["bands"]["grammar"], 5.5);
    assert!(fb["bands"]["pronunciation"].is_null());
    // Overall = mean(6.0, 6.5, 5.5) = 6.0, computed by us, not the model.
    assert_eq!(fb["bands"]["overall"], 6.0);
    assert!(
        fb["evidence"]["pronunciation"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert_eq!(
        fb["corrections"][0]["better"],
        "I went to the office yesterday"
    );
    assert_eq!(fb["l1"], "ur");
    assert_eq!(
        fb["fillers"]["total"], 3,
        "um + you know + basically: {}",
        fb["fillers"]
    );
    assert!(fb["wpm"].as_i64().unwrap() > 0);
    assert_eq!(fb["next_focus"], "Past tense verbs");
    assert_eq!(fb["grader_version"], "grader-v1");

    // What the grader was asked.
    let input = app.grader.last_input();
    assert!(
        input.system_prompt.contains("in English and in Urdu"),
        "{}",
        input.system_prompt
    );
    assert!(
        input
            .user_message
            .contains("LEARNER: Um I go to office yesterday")
    );
    assert!(input.user_message.contains("TUTOR: Tell me more"));
    assert!(input.user_message.contains("Set \"pronunciation\" to null"));

    // Grading cost is logged with tokens.
    let tokens: Value = sqlx::query_scalar(
        "SELECT tokens FROM session_costs WHERE session_id = $1 AND provider = 'grader:test'",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(tokens["input"], 1200);
    assert_eq!(tokens["output"], 300);
}

#[tokio::test]
async fn explanations_follow_the_learners_language() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "hi").await;
    finished_session(&app, &token, transcript(ENOUGH)).await;
    app.run_jobs().await;
    assert!(
        app.grader
            .last_input()
            .system_prompt
            .contains("in English and in Hindi")
    );
}

#[tokio::test]
async fn too_little_speech_is_skipped_not_guessed() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    let id = finished_session(&app, &token, transcript(&["Yes.", "No, I think so."])).await;
    let calls_before = app.grader.calls();
    app.run_jobs().await;
    let fb = feedback(&app, &token, &id).await;
    assert_eq!(fb["status"], "skipped");
    assert!(fb["bands"].is_null());
    assert_eq!(
        app.grader.calls(),
        calls_before,
        "the LLM is not called for tiny sessions"
    );
}

#[tokio::test]
async fn failed_grading_retries_with_backoff_then_succeeds() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    let id = finished_session(&app, &token, transcript(ENOUGH)).await;
    app.grader
        .fail_next
        .store(1, std::sync::atomic::Ordering::SeqCst);

    app.run_jobs().await;
    assert_eq!(feedback(&app, &token, &id).await["status"], "pending");
    let (status, attempts, delay): (String, i32, f64) = sqlx::query_as(
        "SELECT status, attempts, EXTRACT(EPOCH FROM run_at - now())::float8 FROM jobs
         WHERE kind = 'grade_session' AND payload->>'session_id' = $1",
    )
    .bind(&id)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!((status.as_str(), attempts), ("queued", 1));
    assert!(
        delay > 5.0,
        "retry is scheduled later, not immediately ({delay}s)"
    );

    // Time passes...
    sqlx::query("UPDATE jobs SET run_at = now() WHERE payload->>'session_id' = $1")
        .bind(&id)
        .execute(&app.db)
        .await
        .unwrap();
    app.run_jobs().await;
    assert_eq!(feedback(&app, &token, &id).await["status"], "ready");
}

#[tokio::test]
async fn grading_gives_up_after_the_last_attempt() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    let id = finished_session(&app, &token, transcript(ENOUGH)).await;
    sqlx::query("UPDATE jobs SET max_attempts = 1 WHERE payload->>'session_id' = $1")
        .bind(&id)
        .execute(&app.db)
        .await
        .unwrap();
    app.grader
        .fail_next
        .store(5, std::sync::atomic::Ordering::SeqCst);
    app.run_jobs().await;
    assert_eq!(feedback(&app, &token, &id).await["status"], "failed");
    let status: String =
        sqlx::query_scalar("SELECT status FROM jobs WHERE payload->>'session_id' = $1")
            .bind(&id)
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn a_session_gets_one_grading_job_and_one_claim() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    let id = finished_session(&app, &token, transcript(ENOUGH)).await;

    // Enqueueing again is a no-op (unique index).
    let mut conn = app.db.acquire().await.unwrap();
    jobs::enqueue(&mut conn, jobs::GRADE_SESSION, json!({ "session_id": id }))
        .await
        .unwrap();
    drop(conn);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE payload->>'session_id' = $1")
            .bind(&id)
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert_eq!(count, 1);

    // Two workers racing for jobs never get the same one.
    let (a, b) = tokio::join!(jobs::claim(&app.db), jobs::claim(&app.db));
    let (a, b) = (a.unwrap(), b.unwrap());
    if let (Some(a), Some(b)) = (&a, &b) {
        assert_ne!(a.id, b.id, "SKIP LOCKED must hand out different jobs");
    }
    assert!(a.is_some() || b.is_some());
}

#[tokio::test]
async fn jobs_from_a_crashed_worker_are_requeued() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    let id = finished_session(&app, &token, transcript(ENOUGH)).await;
    sqlx::query(
        "UPDATE jobs SET status = 'running', locked_at = now() - interval '1 hour'
         WHERE payload->>'session_id' = $1",
    )
    .bind(&id)
    .execute(&app.db)
    .await
    .unwrap();
    assert!(jobs::reap_stuck(&app.db).await.unwrap() >= 1);
    app.run_jobs().await;
    assert_eq!(feedback(&app, &token, &id).await["status"], "ready");
}

#[tokio::test]
async fn feedback_is_private_to_its_owner() {
    let (_serial, app) = db_app!();
    let (alice, _) = user(&app, "ur").await;
    let (bob, _) = user(&app, "ur").await;
    let id = finished_session(&app, &alice, transcript(ENOUGH)).await;
    let (s, _) = app
        .call(
            Method::GET,
            &format!("/api/v1/sessions/{id}/feedback"),
            None,
            Some(&bob),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = app
        .call(
            Method::GET,
            &format!("/api/v1/sessions/{id}/feedback"),
            None,
            None,
        )
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn progress_lists_recent_grades_and_averages_real_ones() {
    let (_serial, app) = db_app!();
    let (token, _) = user(&app, "ur").await;
    finished_session(&app, &token, transcript(ENOUGH)).await;
    app.run_jobs().await;

    app.grader.output.lock().unwrap().fluency = 7.0;
    app.grader.output.lock().unwrap().lexical = 7.0;
    app.grader.output.lock().unwrap().grammar = 7.0;
    finished_session(&app, &token, transcript(ENOUGH)).await;
    app.run_jobs().await;

    let (s, p) = app
        .call(Method::GET, "/api/v1/progress", None, Some(&token))
        .await;
    assert_eq!(s, StatusCode::OK, "{p}");
    assert_eq!(p["graded_sessions"], 2);
    assert_eq!(p["recent"].as_array().unwrap().len(), 2);
    assert_eq!(p["recent"][0]["overall"], 7.0, "newest first");
    assert_eq!(p["recent"][1]["overall"], 6.0);
    assert_eq!(p["average_overall_last5"], 6.5);
}
