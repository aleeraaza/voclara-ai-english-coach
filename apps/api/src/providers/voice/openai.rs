//! OpenAI Realtime API (fallback provider).
//!
//! `POST /v1/realtime/client_secrets` returns an `ek_...` secret the app uses
//! to open a WebRTC call at `/v1/realtime/calls`.
//!
//! Limits of Direct mode with OpenAI (per the docs): the secret's expiry only
//! limits *starting* a session; a started session keeps running, and the client
//! may override session settings. So we keep the secret alive for 60 s only, the
//! app stops the call at `session_ends_at`, and we never charge beyond the
//! limit. A true server-side cut-off for OpenAI needs relay mode: see
//! [`OpenAiCodec`] and `crate::relay`.
//! Docs: https://platform.openai.com/docs/api-reference/realtime-sessions

use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    CaptionRole, RelayCodec, RelayEvent, RelayTarget, SessionConfig, Transport, VoiceProvider,
    VoiceToken, pcm, upstream_error,
};
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

    fn relay_codec(&self) -> Box<dyn RelayCodec> {
        Box::new(OpenAiCodec {
            transcribe_model: self.transcribe_model.clone(),
        })
    }
}

/// OpenAI Realtime takes and returns 24 kHz PCM16.
const OPENAI_RATE: u32 = 24_000;
const CLIENT_RATE: u32 = 16_000;

/// Relay-mode translator for the OpenAI Realtime WebSocket protocol.
/// Server-side voice activity detection decides when the learner has finished.
pub struct OpenAiCodec {
    transcribe_model: Option<String>,
}

impl RelayCodec for OpenAiCodec {
    fn open(&mut self, cfg: &SessionConfig) -> Vec<String> {
        let mut input = json!({
            "format": { "type": "audio/pcm", "rate": OPENAI_RATE },
            "turn_detection": { "type": "server_vad" }
        });
        if let Some(stt) = &self.transcribe_model {
            input["transcription"] = json!({ "model": stt });
        }
        vec![
            json!({
                "type": "session.update",
                "session": {
                    "type": "realtime",
                    "instructions": cfg.system_prompt,
                    "output_modalities": ["audio"],
                    "audio": {
                        "input": input,
                        "output": { "format": { "type": "audio/pcm", "rate": OPENAI_RATE } }
                    }
                }
            })
            .to_string(),
        ]
    }

    fn audio(&mut self, pcm16_b64_16k: &str) -> AppResult<Vec<String>> {
        let samples = pcm::decode_pcm16(pcm16_b64_16k)?;
        let up = pcm::resample(&samples, CLIENT_RATE, OPENAI_RATE);
        Ok(vec![
            json!({ "type": "input_audio_buffer.append", "audio": pcm::encode_pcm16(&up) })
                .to_string(),
        ])
    }

    fn end_turn(&mut self) -> Vec<String> {
        // Server VAD commits the buffer and starts the reply by itself.
        vec![]
    }

    fn decode(&mut self, upstream: &str) -> Vec<RelayEvent> {
        let Ok(msg) = serde_json::from_str::<Value>(upstream) else {
            return vec![];
        };
        let text = |key: &str| msg[key].as_str().unwrap_or_default().to_owned();
        match msg["type"].as_str().unwrap_or_default() {
            "session.updated" => vec![RelayEvent::Ready],
            // GA name first, beta name second.
            "response.output_audio.delta" | "response.audio.delta" => vec![RelayEvent::Audio {
                pcm16_b64: text("delta"),
                sample_rate: OPENAI_RATE,
            }],
            "response.output_audio_transcript.delta" | "response.audio_transcript.delta" => {
                vec![RelayEvent::Caption {
                    role: CaptionRole::Tutor,
                    text: text("delta"),
                    finished: false,
                }]
            }
            "conversation.item.input_audio_transcription.completed" => vec![RelayEvent::Caption {
                role: CaptionRole::Learner,
                text: text("transcript"),
                finished: true,
            }],
            "input_audio_buffer.speech_started" => vec![RelayEvent::Interrupted],
            "response.done" => vec![RelayEvent::TurnComplete],
            "error" => vec![RelayEvent::Error(
                msg["error"]["message"]
                    .as_str()
                    .unwrap_or("OpenAI error")
                    .to_owned(),
            )],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_sets_instructions_and_upsamples_audio() {
        let mut c = OpenAiCodec {
            transcribe_model: Some("stt-x".into()),
        };
        let open: Value = serde_json::from_str(
            &c.open(&SessionConfig {
                system_prompt: "You are Sana.".into(),
            })[0],
        )
        .unwrap();
        assert_eq!(open["type"], "session.update");
        assert_eq!(open["session"]["instructions"], "You are Sana.");
        assert_eq!(
            open["session"]["audio"]["input"]["transcription"]["model"],
            "stt-x"
        );

        let chunk = pcm::encode_pcm16(&vec![100; 1600]); // 100 ms at 16 kHz
        let sent: Value = serde_json::from_str(&c.audio(&chunk).unwrap()[0]).unwrap();
        assert_eq!(sent["type"], "input_audio_buffer.append");
        let up = pcm::decode_pcm16(sent["audio"].as_str().unwrap()).unwrap();
        assert_eq!(up.len(), 2400, "100 ms at 24 kHz");
        assert!(c.end_turn().is_empty());
    }

    #[test]
    fn codec_decodes_ga_and_beta_events() {
        let mut c = OpenAiCodec {
            transcribe_model: None,
        };
        assert_eq!(
            c.decode(r#"{"type":"session.updated"}"#),
            vec![RelayEvent::Ready]
        );
        for t in ["response.output_audio.delta", "response.audio.delta"] {
            let ev = c.decode(&json!({ "type": t, "delta": "AQI=" }).to_string());
            assert_eq!(
                ev,
                vec![RelayEvent::Audio {
                    pcm16_b64: "AQI=".into(),
                    sample_rate: 24_000
                }]
            );
        }
        let learner = c.decode(
            &json!({ "type": "conversation.item.input_audio_transcription.completed", "transcript": "Hello" })
                .to_string(),
        );
        assert_eq!(
            learner,
            vec![RelayEvent::Caption {
                role: CaptionRole::Learner,
                text: "Hello".into(),
                finished: true
            }]
        );
        assert_eq!(
            c.decode(r#"{"type":"input_audio_buffer.speech_started"}"#),
            vec![RelayEvent::Interrupted]
        );
        assert_eq!(
            c.decode(r#"{"type":"error","error":{"message":"quota"}}"#),
            vec![RelayEvent::Error("quota".into())]
        );
        assert!(c.decode(r#"{"type":"rate_limits.updated"}"#).is_empty());
    }
}
