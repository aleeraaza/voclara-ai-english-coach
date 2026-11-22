//! The real Gemini and OpenAI clients against a local mock server: checks the
//! exact request we send (auth header, locked prompt, expiry) and how we read
//! the reply. No network or API keys needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use voclara_api::{
    error::AppError,
    providers::voice::{
        SessionConfig, Transport, VoiceProvider, gemini::GeminiLive, openai::OpenAiRealtime,
    },
};

#[derive(Clone, Default)]
struct Seen {
    last: Arc<Mutex<Option<(HeaderMap, Value)>>>,
}

/// Start a mock upstream on a random port. `reply` is (status, JSON body).
async fn mock(path: &'static str, reply: (StatusCode, Value)) -> (String, Seen) {
    let seen = Seen::default();
    let app = Router::new()
        .route(
            path,
            post(
                move |State(seen): State<Seen>, headers: HeaderMap, Json(body): Json<Value>| {
                    let reply = reply.clone();
                    async move {
                        *seen.last.lock().unwrap() = Some((headers, body));
                        (reply.0, Json(reply.1))
                    }
                },
            ),
        )
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn cfg() -> SessionConfig {
    SessionConfig {
        system_prompt: "You are Sana, a warm, patient English speaking coach.".into(),
    }
}

#[tokio::test]
async fn gemini_locks_setup_into_a_single_use_token() {
    let (base, seen) = mock(
        "/v1beta/auth_tokens",
        (StatusCode::OK, json!({ "name": "auth_tokens/abc123" })),
    )
    .await;
    let gemini = GeminiLive::new("gm-key".into(), "live-model-x".into(), base.clone()).unwrap();

    let before = Utc::now();
    let token = gemini.mint_ephemeral_token(&cfg(), 600).await.unwrap();
    assert_eq!(token.token, "auth_tokens/abc123");
    assert_eq!(token.transport, Transport::Websocket);
    assert!(token.hard_cutoff);
    assert!(token.connect_url.starts_with("ws://127.0.0.1:"));
    assert!(
        token
            .connect_url
            .ends_with("BidiGenerateContentConstrained")
    );

    let (headers, body) = seen.last.lock().unwrap().clone().expect("no request");
    assert_eq!(headers["x-goog-api-key"], "gm-key");
    assert_eq!(body["uses"], 1);
    let setup = &body["bidiGenerateContentSetup"];
    assert_eq!(setup["model"], "models/live-model-x");
    assert_eq!(setup["generationConfig"]["responseModalities"][0], "AUDIO");
    assert_eq!(
        setup["systemInstruction"]["parts"][0]["text"],
        cfg().system_prompt
    );
    assert!(
        setup["inputAudioTranscription"].is_object(),
        "learner captions needed for grading"
    );

    // expireTime is the hard stop: ~600 s from now.
    let expire: DateTime<Utc> = body["expireTime"].as_str().unwrap().parse().unwrap();
    let secs = (expire - before).num_seconds();
    assert!((599..=601).contains(&secs), "expireTime {secs}s ahead");
    let start: DateTime<Utc> = body["newSessionExpireTime"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((start - before).num_seconds() <= 61);
}

#[tokio::test]
async fn gemini_upstream_error_is_unavailable() {
    let (base, _) = mock(
        "/v1beta/auth_tokens",
        (StatusCode::TOO_MANY_REQUESTS, json!({ "error": "quota" })),
    )
    .await;
    let gemini = GeminiLive::new("k".into(), "m".into(), base).unwrap();
    assert!(matches!(
        gemini.mint_ephemeral_token(&cfg(), 60).await,
        Err(AppError::Unavailable)
    ));
}

#[tokio::test]
async fn gemini_unreachable_is_unavailable() {
    let gemini = GeminiLive::new("k".into(), "m".into(), "http://127.0.0.1:1".into()).unwrap();
    assert!(matches!(
        gemini.mint_ephemeral_token(&cfg(), 60).await,
        Err(AppError::Unavailable)
    ));
}

#[tokio::test]
async fn openai_mints_a_short_lived_client_secret() {
    let expires = Utc::now().timestamp() + 60;
    let (base, seen) = mock(
        "/v1/realtime/client_secrets",
        (
            StatusCode::OK,
            json!({ "value": "ek_test", "expires_at": expires, "session": {} }),
        ),
    )
    .await;
    let openai = OpenAiRealtime::new(
        "sk-test".into(),
        "realtime-mini-x".into(),
        Some("stt-x".into()),
        base.clone(),
    )
    .unwrap();

    let token = openai.mint_ephemeral_token(&cfg(), 600).await.unwrap();
    assert_eq!(token.token, "ek_test");
    assert_eq!(token.transport, Transport::Webrtc);
    assert!(
        !token.hard_cutoff,
        "OpenAI cannot be cut off server-side in direct mode"
    );
    assert_eq!(token.connect_url, format!("{base}/v1/realtime/calls"));
    assert_eq!(token.token_expires_at.timestamp(), expires);

    let (headers, body) = seen.last.lock().unwrap().clone().expect("no request");
    assert_eq!(headers["authorization"], "Bearer sk-test");
    assert_eq!(body["expires_after"]["seconds"], 60);
    assert_eq!(body["session"]["type"], "realtime");
    assert_eq!(body["session"]["model"], "realtime-mini-x");
    assert_eq!(body["session"]["instructions"], cfg().system_prompt);
    assert_eq!(
        body["session"]["audio"]["input"]["transcription"]["model"],
        "stt-x"
    );
}

#[tokio::test]
async fn openai_omits_transcription_when_not_configured() {
    let (base, seen) = mock(
        "/v1/realtime/client_secrets",
        (
            StatusCode::OK,
            json!({ "value": "ek", "expires_at": Utc::now().timestamp() + 60 }),
        ),
    )
    .await;
    let openai = OpenAiRealtime::new("k".into(), "m".into(), None, base).unwrap();
    openai.mint_ephemeral_token(&cfg(), 60).await.unwrap();
    let (_, body) = seen.last.lock().unwrap().clone().unwrap();
    assert!(body["session"].get("audio").is_none());
}

#[test]
fn relay_targets_use_server_side_keys() {
    let gemini = GeminiLive::new("gm".into(), "m".into(), "https://example.test".into()).unwrap();
    let t = gemini.relay_target().unwrap();
    assert!(t.url.starts_with("wss://example.test/ws/"));
    assert!(t.url.ends_with("BidiGenerateContent"));
    assert_eq!(t.headers[0], ("x-goog-api-key".to_owned(), "gm".to_owned()));

    let openai =
        OpenAiRealtime::new("sk".into(), "rt".into(), None, "https://api.test".into()).unwrap();
    let t = openai.relay_target().unwrap();
    assert_eq!(t.url, "wss://api.test/v1/realtime?model=rt");
}
