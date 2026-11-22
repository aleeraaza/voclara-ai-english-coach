//! OpenAI Realtime API (fallback provider).
//!
//! `POST /v1/realtime/client_secrets` returns an `ek_...` secret the app uses
//! to open a WebRTC call at `/v1/realtime/calls`.
//!
//! Limits of Direct mode with OpenAI (per the docs): the secret's expiry only
//! limits *starting* a session; a started session keeps running, and the client
//! may override session settings. So we keep the secret alive for 60 s only, the
//! app stops the call at `session_ends_at`, and we never charge beyond the
//! limit. A true server-side cut-off for OpenAI needs relay mode (step 5).
//! Docs: https://platform.openai.com/docs/api-reference/realtime-sessions

use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_json::json;

use super::{RelayTarget, SessionConfig, Transport, VoiceProvider, VoiceToken, upstream_error};
use crate::error::{AppError, AppResult};

const NAME: &str = "openai";
/// The app must start the WebRTC call within this window.
const SECRET_TTL_SECS: i64 = 60;

pub struct OpenAiRealtime {
    http: reqwest::Client,
    api_key: String,
    model: String,
    transcribe_model: Option<String>,
    base_url: String,
}

#[derive(Deserialize)]
struct ClientSecretResponse {
    value: String,
    expires_at: i64,
}

impl OpenAiRealtime {
    pub fn new(
        api_key: String,
        model: String,
        transcribe_model: Option<String>,
        base_url: String,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            api_key,
            model,
            transcribe_model,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    pub fn token_request(&self, cfg: &SessionConfig) -> serde_json::Value {
        let mut session = json!({
            "type": "realtime",
            "model": self.model,
            "instructions": cfg.system_prompt,
            "output_modalities": ["audio"],
        });
        if let Some(stt) = &self.transcribe_model {
            session["audio"] = json!({ "input": { "transcription": { "model": stt } } });
        }
        json!({
            "expires_after": { "anchor": "created_at", "seconds": SECRET_TTL_SECS },
            "session": session
        })
    }
}

#[async_trait]
impl VoiceProvider for OpenAiRealtime {
    fn name(&self) -> &'static str {
        NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn mint_ephemeral_token(
        &self,
        cfg: &SessionConfig,
        _ttl_secs: u32,
    ) -> AppResult<VoiceToken> {
        let resp: ClientSecretResponse = self
            .http
            .post(format!("{}/v1/realtime/client_secrets", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&self.token_request(cfg))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| upstream_error(NAME, "creating OpenAI client secret", e))?
            .json()
            .await
            .map_err(|e| upstream_error(NAME, "parsing OpenAI client secret", e))?;

        let token_expires_at = Utc
            .timestamp_opt(resp.expires_at, 0)
            .single()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("bad expires_at from OpenAI")))?;
        Ok(VoiceToken {
            token: resp.value,
            token_expires_at,
            connect_url: format!("{}/v1/realtime/calls", self.base_url),
            transport: Transport::Webrtc,
            hard_cutoff: false,
        })
    }

    fn relay_target(&self) -> AppResult<RelayTarget> {
        let ws = self
            .base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        Ok(RelayTarget {
            url: format!("{ws}/v1/realtime?model={}", self.model),
            headers: vec![("authorization".into(), format!("Bearer {}", self.api_key))],
        })
    }
}
