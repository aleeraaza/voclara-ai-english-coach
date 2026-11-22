//! Real Gemini/OpenAI grader clients against a local mock server: checks the
//! request we send and how the reply is read. No network or keys needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use voclara_api::{
    error::AppError,
    providers::grader::{GradeInput, Grader, gemini::GeminiGrader, openai::OpenAiGrader},
};

type Seen = Arc<Mutex<Option<(String, HeaderMap, Value)>>>;

async fn mock(reply: (StatusCode, Value)) -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .fallback(post(
            move |State(seen): State<Seen>,
                  uri: axum::http::Uri,
                  headers: HeaderMap,
                  Json(body): Json<Value>| {
                let reply = reply.clone();
                async move {
                    *seen.lock().unwrap() = Some((uri.path().to_owned(), headers, body));
                    (reply.0, Json(reply.1))
                }
            },
        ))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn grade_json() -> Value {
    json!({
        "fluency": 6.5, "lexical": 6.0, "grammar": 5.5, "pronunciation": null,
        "evidence": { "fluency": ["I go office"], "lexical": [], "grammar": ["I go office"], "pronunciation": [] },
        "corrections": [{ "said": "I go office", "better": "I went to the office",
                          "why_en": "Past tense.", "why_l1": "ماضی کا صیغہ" }],
        "next_focus": "Past tense"
    })
}

fn input() -> GradeInput {
    GradeInput {
        system_prompt: "You are an experienced IELTS Speaking examiner.".into(),
        user_message: "LEARNER: I go office".into(),
    }
}

#[tokio::test]
async fn gemini_requests_json_schema_output_and_reads_tokens() {
    let (base, seen) = mock((
        StatusCode::OK,
        json!({
            "candidates": [{ "content": { "parts": [{ "text": grade_json().to_string() }] }, "finishReason": "STOP" }],
            "usageMetadata": { "promptTokenCount": 900, "candidatesTokenCount": 210 }
        }),
    ))
    .await;
    let g = GeminiGrader::new("gm-key".into(), "grader-x".into(), base).unwrap();
    let (out, usage) = g.grade(&input()).await.unwrap();
    assert_eq!(out.fluency, 6.5);
    assert_eq!(out.corrections[0].better, "I went to the office");
    assert_eq!((usage.input_tokens, usage.output_tokens), (900, 210));

    let (path, headers, body) = seen.lock().unwrap().clone().unwrap();
    assert_eq!(path, "/v1beta/models/grader-x:generateContent");
    assert_eq!(headers["x-goog-api-key"], "gm-key");
    assert_eq!(
        body["systemInstruction"]["parts"][0]["text"],
        input().system_prompt
    );
    assert_eq!(
        body["contents"][0]["parts"][0]["text"],
        "LEARNER: I go office"
    );
    assert_eq!(
        body["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(
        body["generationConfig"]["responseJsonSchema"]["type"],
        "object"
    );
}

#[tokio::test]
async fn gemini_without_a_candidate_is_an_error() {
    let (base, _) = mock((
        StatusCode::OK,
        json!({ "candidates": [{ "finishReason": "SAFETY" }] }),
    ))
    .await;
    let g = GeminiGrader::new("k".into(), "m".into(), base).unwrap();
    let err = g.grade(&input()).await.unwrap_err();
    assert!(format!("{err:?}").contains("SAFETY"), "{err:?}");
}

#[tokio::test]
async fn openai_uses_strict_json_schema() {
    let (base, seen) = mock((
        StatusCode::OK,
        json!({
            "choices": [{ "message": { "role": "assistant", "content": grade_json().to_string(), "refusal": null } }],
            "usage": { "prompt_tokens": 800, "completion_tokens": 150 }
        }),
    ))
    .await;
    let g = OpenAiGrader::new("sk-test".into(), "grader-mini".into(), base).unwrap();
    let (out, usage) = g.grade(&input()).await.unwrap();
    assert_eq!(out.grammar, 5.5);
    assert_eq!((usage.input_tokens, usage.output_tokens), (800, 150));

    let (path, headers, body) = seen.lock().unwrap().clone().unwrap();
    assert_eq!(path, "/v1/chat/completions");
    assert_eq!(headers["authorization"], "Bearer sk-test");
    assert_eq!(body["model"], "grader-mini");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(
        body["response_format"]["json_schema"]["schema"]["additionalProperties"],
        false
    );
}

#[tokio::test]
async fn openai_refusal_is_an_error() {
    let (base, _) = mock((
        StatusCode::OK,
        json!({ "choices": [{ "message": { "content": null, "refusal": "I can't help with that." } }] }),
    ))
    .await;
    let g = OpenAiGrader::new("k".into(), "m".into(), base).unwrap();
    assert!(g.grade(&input()).await.is_err());
}

#[tokio::test]
async fn provider_errors_are_retryable_unavailable() {
    let (base, _) = mock((StatusCode::TOO_MANY_REQUESTS, json!({ "error": "quota" }))).await;
    for g in [
        Box::new(GeminiGrader::new("k".into(), "m".into(), base.clone()).unwrap())
            as Box<dyn Grader>,
        Box::new(OpenAiGrader::new("k".into(), "m".into(), base.clone()).unwrap()),
    ] {
        assert!(matches!(
            g.grade(&input()).await,
            Err(AppError::Unavailable)
        ));
    }
}

#[tokio::test]
async fn malformed_model_json_is_an_error() {
    let (base, _) = mock((
        StatusCode::OK,
        json!({ "candidates": [{ "content": { "parts": [{ "text": "{\"fluency\": \"high\"}" }] } }] }),
    ))
    .await;
    let g = GeminiGrader::new("k".into(), "m".into(), base).unwrap();
    assert!(g.grade(&input()).await.is_err());
}
