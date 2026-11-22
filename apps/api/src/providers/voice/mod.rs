//! Realtime voice providers behind one trait.
//!
//! **Direct mode** (MVP): the API mints a short-lived token and the app talks
//! to the provider itself, so no audio passes through our server.
//! **Relay mode** (step 5): the API opens the upstream connection itself;
//! [`VoiceProvider::relay_target`] says where and how to connect.

pub mod fake;
pub mod gemini;
pub mod openai;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    config::{VoiceConfig, VoiceProviderKind},
    error::{AppError, AppResult},
};

/// Build one provider from config. `Config::validate` has already checked that
/// the chosen provider has its key and model, so `None` here is a bug.
fn build(kind: VoiceProviderKind, cfg: &VoiceConfig) -> anyhow::Result<Arc<dyn VoiceProvider>> {
    use anyhow::Context;
    Ok(match kind {
        VoiceProviderKind::Gemini => Arc::new(gemini::GeminiLive::new(
            cfg.gemini_api_key.clone().context("GEMINI_API_KEY")?,
            cfg.gemini_live_model.clone().context("GEMINI_LIVE_MODEL")?,
            cfg.gemini_base_url.clone(),
        )?),
        VoiceProviderKind::OpenAi => Arc::new(openai::OpenAiRealtime::new(
            cfg.openai_api_key.clone().context("OPENAI_API_KEY")?,
            cfg.openai_realtime_model
                .clone()
                .context("OPENAI_REALTIME_MODEL")?,
            cfg.openai_transcribe_model.clone(),
            cfg.openai_base_url.clone(),
        )?),
        VoiceProviderKind::Fake => Arc::new(fake::FakeVoice),
    })
}

/// Primary plus optional fallback, as configured.
pub fn from_config(cfg: &VoiceConfig) -> anyhow::Result<VoiceProviders> {
    let primary = build(cfg.primary, cfg)?;
    let fallback = cfg.fallback.map(|k| build(k, cfg)).transpose()?;
    Ok(VoiceProviders::new(primary, fallback))
}

/// What the provider needs to run one session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Fully rendered system prompt (tutor, examiner or interviewer).
    pub system_prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Connect a WebSocket to `connect_url` with the token (Gemini Live).
    Websocket,
    /// POST an SDP offer to `connect_url` with the token as bearer (OpenAI).
    Webrtc,
}

/// A token the app uses to talk to the provider directly.
#[derive(Debug, Clone)]
pub struct VoiceToken {
    pub token: String,
    /// After this the token cannot start a session.
    pub token_expires_at: DateTime<Utc>,
    pub connect_url: String,
    pub transport: Transport,
    /// True when the provider itself stops the session at the time limit.
    /// False means only the app stops it (and we charge up to the limit).
    pub hard_cutoff: bool,
}

/// Where the server connects in relay mode.
#[derive(Debug, Clone)]
pub struct RelayTarget {
    pub url: String,
    /// Auth headers for the upstream request. Contain secrets: never log.
    pub headers: Vec<(String, String)>,
}

#[async_trait]
pub trait VoiceProvider: Send + Sync {
    /// Short name stored on sessions and cost rows: "gemini", "openai", "fake".
    fn name(&self) -> &'static str;
    /// The configured model id.
    fn model(&self) -> &str;
    /// Mint a token valid for one session of at most `ttl_secs`.
    async fn mint_ephemeral_token(
        &self,
        cfg: &SessionConfig,
        ttl_secs: u32,
    ) -> AppResult<VoiceToken>;
    /// Upstream address and auth for relay mode.
    fn relay_target(&self) -> AppResult<RelayTarget>;
}

/// The primary provider plus an optional fallback, tried when the primary is
/// unavailable (outage, quota, retired model).
pub struct VoiceProviders {
    primary: Arc<dyn VoiceProvider>,
    fallback: Option<Arc<dyn VoiceProvider>>,
}

impl VoiceProviders {
    pub fn new(primary: Arc<dyn VoiceProvider>, fallback: Option<Arc<dyn VoiceProvider>>) -> Self {
        Self { primary, fallback }
    }

    pub fn primary(&self) -> &dyn VoiceProvider {
        self.primary.as_ref()
    }

    /// Mint with the primary; on `Unavailable` try the fallback. Returns the
    /// provider that actually issued the token, so the session records it.
    pub async fn mint(
        &self,
        cfg: &SessionConfig,
        ttl_secs: u32,
    ) -> AppResult<(&dyn VoiceProvider, VoiceToken)> {
        match self.primary.mint_ephemeral_token(cfg, ttl_secs).await {
            Ok(token) => Ok((self.primary.as_ref(), token)),
            Err(AppError::Unavailable) => {
                let Some(fallback) = &self.fallback else {
                    return Err(AppError::Unavailable);
                };
                tracing::warn!(
                    primary = self.primary.name(),
                    fallback = fallback.name(),
                    "primary voice provider unavailable; using fallback"
                );
                let token = fallback.mint_ephemeral_token(cfg, ttl_secs).await?;
                Ok((fallback.as_ref(), token))
            }
            Err(e) => Err(e),
        }
    }
}

/// Map a provider HTTP failure to 503 and log it without secrets or prompts.
pub(crate) fn upstream_error(
    provider: &'static str,
    what: &str,
    err: impl std::fmt::Display,
) -> AppError {
    tracing::error!(provider, error = %err, "{what} failed");
    AppError::Unavailable
}
