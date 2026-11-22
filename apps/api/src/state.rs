use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    auth::JwtKeys,
    config::Config,
    prompts::Prompts,
    providers::{google::GoogleVerifier, otp::OtpSender, voice::VoiceProviders},
    relay::RelayLimits,
};

/// External services the app talks to. Real ones in `main`, fakes in tests.
pub struct Providers {
    pub otp: Arc<dyn OtpSender>,
    pub google: Arc<dyn GoogleVerifier>,
    pub voice: Arc<VoiceProviders>,
}

/// Shared state handed to every handler. Cheap to clone: every field is a
/// handle (`PgPool` wraps an `Arc` internally; the rest are `Arc`s).
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<JwtKeys>,
    pub prompts: Arc<Prompts>,
    pub otp: Arc<dyn OtpSender>,
    pub google: Arc<dyn GoogleVerifier>,
    pub voice: Arc<VoiceProviders>,
    /// Relay timeouts; tests shorten them.
    pub relay_limits: Arc<RelayLimits>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, providers: Providers) -> anyhow::Result<Self> {
        let jwt = Arc::new(JwtKeys::new(
            config.jwt_secret.as_bytes(),
            config.access_ttl_secs,
        ));
        Ok(Self {
            db,
            config: Arc::new(config),
            jwt,
            prompts: Arc::new(Prompts::load()?),
            otp: providers.otp,
            google: providers.google,
            voice: providers.voice,
            relay_limits: Arc::new(RelayLimits::default()),
        })
    }
}
