//! Development provider: returns a dummy token so the app's session screens
//! can be built before real API keys exist. Refused in production by config.

use async_trait::async_trait;
use chrono::Utc;

use super::{RelayTarget, SessionConfig, Transport, VoiceProvider, VoiceToken};
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
}
