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
use serde_json::{Value, json};

use super::{
    CaptionRole, RelayCodec, RelayEvent, RelayTarget, SessionConfig, Transport, VoiceProvider,
    VoiceToken, upstream_error,
};
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
            "bidiGenerateContentSetup": session_setup(&self.model, cfg),
        })
    }
}

/// The session setup, shared by direct mode (locked into the token) and relay
/// mode (sent by the server as the first upstream message).
fn session_setup(model: &str, cfg: &SessionConfig) -> serde_json::Value {
    json!({
        "model": format!("models/{model}"),
        "generationConfig": { "responseModalities": ["AUDIO"] },
        "systemInstruction": { "parts": [{ "text": cfg.system_prompt }] },
        // Captions for both sides; the learner's transcript is what we grade.
        "inputAudioTranscription": {},
        "outputAudioTranscription": {},
        // Gemini drops connections about every 10 minutes; resumption
        // lets the app reconnect with the same token.
        "sessionResumption": {}
    })
}

/// Relay-mode translator for the Gemini Live protocol (https://ai.google.dev/api/live).
pub struct GeminiCodec {
    model: String,
}

impl GeminiCodec {
    pub fn new(model: String) -> Self {
        Self { model }
    }
}

fn rate_from_mime(mime: &str) -> u32 {
    mime.split(';')
        .find_map(|p| p.trim().strip_prefix("rate="))
        .and_then(|r| r.parse().ok())
        .unwrap_or(24_000)
}

impl RelayCodec for GeminiCodec {
    fn open(&mut self, cfg: &SessionConfig) -> Vec<String> {
        vec![json!({ "setup": session_setup(&self.model, cfg) }).to_string()]
    }

    fn audio(&mut self, pcm16_b64_16k: &str) -> AppResult<Vec<String>> {
        Ok(vec![
            json!({
                "realtimeInput": {
                    "audio": { "data": pcm16_b64_16k, "mimeType": "audio/pcm;rate=16000" }
                }
            })
            .to_string(),
        ])
    }

    fn end_turn(&mut self) -> Vec<String> {
        vec![json!({ "realtimeInput": { "audioStreamEnd": true } }).to_string()]
    }

    fn decode(&mut self, upstream: &str) -> Vec<RelayEvent> {
        let Ok(msg) = serde_json::from_str::<Value>(upstream) else {
            return vec![];
        };
        let mut events = Vec::new();
        if msg.get("setupComplete").is_some() {
            events.push(RelayEvent::Ready);
        }
        if let Some(sc) = msg.get("serverContent") {
            if sc["interrupted"].as_bool() == Some(true) {
                events.push(RelayEvent::Interrupted);
            }
            if let Some(parts) = sc["modelTurn"]["parts"].as_array() {
                for part in parts {
                    let blob = &part["inlineData"];
                    let mime = blob["mimeType"].as_str().unwrap_or_default();
                    if let (Some(data), true) = (blob["data"].as_str(), mime.starts_with("audio/"))
                    {
                        events.push(RelayEvent::Audio {
                            pcm16_b64: data.to_owned(),
                            sample_rate: rate_from_mime(mime),
                        });
                    }
                }
            }
            for (key, role) in [
                ("inputTranscription", CaptionRole::Learner),
                ("outputTranscription", CaptionRole::Tutor),
            ] {
                if let Some(text) = sc[key]["text"].as_str().filter(|t| !t.is_empty()) {
                    events.push(RelayEvent::Caption {
                        role,
                        text: text.to_owned(),
                        finished: false,
                    });
                }
            }
            if sc["turnComplete"].as_bool() == Some(true) {
                events.push(RelayEvent::TurnComplete);
            }
        }
        if let Some(err) = msg.get("error") {
            events.push(RelayEvent::Error(
                err["message"].as_str().unwrap_or("Gemini error").to_owned(),
            ));
        }
        events
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

    fn relay_codec(&self) -> Box<dyn RelayCodec> {
        Box::new(GeminiCodec::new(self.model.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SessionConfig {
        SessionConfig {
            system_prompt: "You are Sana.".into(),
        }
    }

    #[test]
    fn codec_sends_server_side_setup_and_16k_audio() {
        let mut c = GeminiCodec::new("live-x".into());
        let open: Value = serde_json::from_str(&c.open(&cfg())[0]).unwrap();
        assert_eq!(open["setup"]["model"], "models/live-x");
        assert_eq!(
            open["setup"]["systemInstruction"]["parts"][0]["text"],
            "You are Sana."
        );
        let audio: Value = serde_json::from_str(&c.audio("AAA=").unwrap()[0]).unwrap();
        assert_eq!(
            audio["realtimeInput"]["audio"]["mimeType"],
            "audio/pcm;rate=16000"
        );
        let end: Value = serde_json::from_str(&c.end_turn()[0]).unwrap();
        assert_eq!(end["realtimeInput"]["audioStreamEnd"], true);
    }

    #[test]
    fn codec_decodes_server_messages() {
        let mut c = GeminiCodec::new("m".into());
        assert_eq!(c.decode(r#"{"setupComplete":{}}"#), vec![RelayEvent::Ready]);
        let msg = json!({ "serverContent": {
            "interrupted": true,
            "modelTurn": { "parts": [{ "inlineData": { "mimeType": "audio/pcm;rate=24000", "data": "AQI=" } }] },
            "inputTranscription": { "text": "I go office" },
            "outputTranscription": { "text": "You went" },
            "turnComplete": true
        }});
        assert_eq!(
            c.decode(&msg.to_string()),
            vec![
                RelayEvent::Interrupted,
                RelayEvent::Audio {
                    pcm16_b64: "AQI=".into(),
                    sample_rate: 24_000
                },
                RelayEvent::Caption {
                    role: CaptionRole::Learner,
                    text: "I go office".into(),
                    finished: false
                },
                RelayEvent::Caption {
                    role: CaptionRole::Tutor,
                    text: "You went".into(),
                    finished: false
                },
                RelayEvent::TurnComplete,
            ]
        );
        assert!(c.decode("not json").is_empty());
        assert!(c.decode(r#"{"usageMetadata":{}}"#).is_empty());
    }
}
