use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    auth::JwtKeys,
    config::Config,
    providers::{google::GoogleVerifier, otp::OtpSender},
};

/// Shared state handed to every handler. Cheap to clone: every field is a
/// handle (`PgPool` wraps an `Arc` internally; the rest are `Arc`s).
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<JwtKeys>,
    pub otp: Arc<dyn OtpSender>,
    pub google: Arc<dyn GoogleVerifier>,
}

impl AppState {
    pub fn new(
        db: PgPool,
        config: Config,
        otp: Arc<dyn OtpSender>,
        google: Arc<dyn GoogleVerifier>,
    ) -> Self {
        let jwt = Arc::new(JwtKeys::new(
            config.jwt_secret.as_bytes(),
            config.access_ttl_secs,
        ));
        Self {
            db,
            config: Arc::new(config),
            jwt,
            otp,
            google,
        }
    }
}
