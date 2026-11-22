//! Gemini Live API.
//!
//! `POST /v1beta/auth_tokens` creates a single-use ephemeral token. We lock the
//! whole session setup (model, system prompt, audio, transcripts) into the token
//! with `bidiGenerateContentSetup`, so the app cannot change the prompt.
//! `expireTime` makes Gemini reject messages after the session's time limit:
//! a hard cut-off enforced by the provider.
//! Docs: https://ai.google.dev/gemini-api/docs/ephemeral-tokens

use std::time::Duration;

use async_trait::async_trait;
use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::json;

use super::{RelayTarget, SessionConfig, Transport, VoiceProvider, VoiceToken, upstream_error};
use crate::error::AppResult;

const NAME: &str = "gemini";
/// The app must open its WebSocket within this window.
const START_WINDOW_SECS: i64 = 60;
/// Endpoint for ephemeral-token connections (setup locked by the token).
const WS_PATH_CONSTRAINED: &str =
    "/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContentConstrained";
/// Endpoint for API-key connections (relay mode, server side only).
const WS_PATH_API_KEY: &str =
    "/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";

pub struct GeminiLive {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Deserialize)]
struct AuthTokenResponse {
    /// The token itself, e.g. "auth_tokens/abc...".
    name: String,
}

impl GeminiLive {
    pub fn new(api_key: String, model: String, base_url: String) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    fn ws_origin(&self) -> String {
        self.base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1)
    }

    /// The request body; split out so tests can check it without a network.
    pub fn token_request(&self, cfg: &SessionConfig, ttl_secs: u32) -> serde_json::Value {
        let now = Utc::now();
        let expire = now + chrono::Duration::seconds(i64::from(ttl_secs));
        let new_session = now + chrono::Duration::seconds(START_WINDOW_SECS);
        json!({
            "uses": 1,
            "expireTime": expire.to_rfc3339_opts(SecondsFormat::Secs, true),
            "newSessionExpireTime": new_session.to_rfc3339_opts(SecondsFormat::Secs, true),
            "bidiGenerateContentSetup": {
                "model": format!("models/{}", self.model),
                "generationConfig": { "responseModalities": ["AUDIO"] },
                "systemInstruction": { "parts": [{ "text": cfg.system_prompt }] },
                // Captions for both sides; the learner's transcript is what we grade.
                "inputAudioTranscription": {},
                "outputAudioTranscription": {},
                // Gemini drops connections about every 10 minutes; resumption
                // lets the app reconnect with the same token.
                "sessionResumption": {}
            }
        })
    }
}

#[async_trait]
impl VoiceProvider for GeminiLive {
    fn name(&self) -> &'static str {
        NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn mint_ephemeral_token(
        &self,
        cfg: &SessionConfig,
        ttl_secs: u32,
    ) -> AppResult<VoiceToken> {
        let body = self.token_request(cfg, ttl_secs);
        let resp: AuthTokenResponse = self
            .http
            .post(format!("{}/v1beta/auth_tokens", self.base_url))
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| upstream_error(NAME, "creating Gemini ephemeral token", e))?
            .json()
            .await
            .map_err(|e| upstream_error(NAME, "parsing Gemini token response", e))?;

        Ok(VoiceToken {
            token: resp.name,
            token_expires_at: Utc::now() + chrono::Duration::seconds(START_WINDOW_SECS),
            // The app appends `?access_token=<token>` (or sends the header
            // `Authorization: Token <token>`), as the Gemini docs describe.
            connect_url: format!("{}{WS_PATH_CONSTRAINED}", self.ws_origin()),
            transport: Transport::Websocket,
            hard_cutoff: true,
        })
    }

    fn relay_target(&self) -> AppResult<RelayTarget> {
        Ok(RelayTarget {
            url: format!("{}{WS_PATH_API_KEY}", self.ws_origin()),
            headers: vec![("x-goog-api-key".into(), self.api_key.clone())],
        })
    }
}
