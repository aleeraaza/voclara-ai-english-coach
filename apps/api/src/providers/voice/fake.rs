//! Development provider: returns a dummy token so the app's session screens
//! can be built before real API keys exist. Refused in production by config.
//!
//! [`PassthroughCodec`] is a tiny JSON protocol used by fake upstreams in
//! tests (and handy for local relay experiments):
//!
//! - relay -> upstream: `{"setup":{"prompt":...}}`, `{"audio":"<b64>"}`, `{"end_turn":true}`
//! - upstream -> relay: `{"ready":true}`, `{"audio":"<b64>","rate":24000}`,
//!   `{"caption":{"role":"tutor"|"learner","text":...,"finished":bool}}`,
//!   `{"interrupted":true}`, `{"turn_complete":true}`, `{"error":"..."}`

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};

use super::{
    CaptionRole, RelayCodec, RelayEvent, RelayTarget, SessionConfig, Transport, VoiceProvider,
    VoiceToken,
};
use crate::error::AppResult;

pub struct FakeVoice;

#[async_trait]
impl VoiceProvider for FakeVoice {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn model(&self) -> &str {
        "fake-voice"
    }

    async fn mint_ephemeral_token(
        &self,
        _cfg: &SessionConfig,
        _ttl_secs: u32,
    ) -> AppResult<VoiceToken> {
        Ok(VoiceToken {
            token: format!("fake-{}", uuid::Uuid::new_v4().simple()),
            token_expires_at: Utc::now() + chrono::Duration::seconds(60),
            connect_url: "ws://127.0.0.1:0/fake-voice".into(),
            transport: Transport::Websocket,
            hard_cutoff: false,
        })
    }

    fn relay_target(&self) -> AppResult<RelayTarget> {
        Ok(RelayTarget {
            url: "ws://127.0.0.1:0/fake-voice".into(),
            headers: vec![],
        })
    }

    fn relay_codec(&self) -> Box<dyn RelayCodec> {
        Box::new(PassthroughCodec)
    }
}

pub struct PassthroughCodec;

impl RelayCodec for PassthroughCodec {
    fn open(&mut self, cfg: &SessionConfig) -> Vec<String> {
        vec![json!({ "setup": { "prompt": cfg.system_prompt } }).to_string()]
    }

    fn audio(&mut self, pcm16_b64_16k: &str) -> AppResult<Vec<String>> {
        Ok(vec![json!({ "audio": pcm16_b64_16k }).to_string()])
    }

    fn end_turn(&mut self) -> Vec<String> {
        vec![json!({ "end_turn": true }).to_string()]
    }

    fn decode(&mut self, upstream: &str) -> Vec<RelayEvent> {
        let Ok(msg) = serde_json::from_str::<Value>(upstream) else {
            return vec![];
        };
        let mut events = Vec::new();
        if msg["ready"].as_bool() == Some(true) {
            events.push(RelayEvent::Ready);
        }
        if let Some(audio) = msg["audio"].as_str() {
            events.push(RelayEvent::Audio {
                pcm16_b64: audio.to_owned(),
                sample_rate: msg["rate"].as_u64().unwrap_or(24_000) as u32,
            });
        }
        if let Some(text) = msg["caption"]["text"].as_str() {
            let role = if msg["caption"]["role"] == "learner" {
                CaptionRole::Learner
            } else {
                CaptionRole::Tutor
            };
            events.push(RelayEvent::Caption {
                role,
                text: text.to_owned(),
                finished: msg["caption"]["finished"].as_bool().unwrap_or(false),
            });
        }
        if msg["interrupted"].as_bool() == Some(true) {
            events.push(RelayEvent::Interrupted);
        }
        if msg["turn_complete"].as_bool() == Some(true) {
            events.push(RelayEvent::TurnComplete);
        }
        if let Some(err) = msg["error"].as_str() {
            events.push(RelayEvent::Error(err.to_owned()));
        }
        events
    }
}
