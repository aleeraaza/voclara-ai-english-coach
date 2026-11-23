//! Configuration loaded from environment variables (and `.env` in development).

use std::{env, net::SocketAddr, str::FromStr};

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

impl FromStr for LogFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            other => anyhow::bail!("LOG_FORMAT must be 'pretty' or 'json', got '{other}'"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
}

impl FromStr for AppEnv {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "production" | "prod" => Ok(Self::Production),
            other => anyhow::bail!("APP_ENV must be 'development' or 'production', got '{other}'"),
        }
    }
}

/// Which service delivers one-time codes. Only `log` exists until a
/// WhatsApp/SMS provider is chosen; it is refused in production.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtpProvider {
    Log,
}

impl FromStr for OtpProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "log" => Ok(Self::Log),
            other => anyhow::bail!("unknown OTP_PROVIDER '{other}' (supported: log)"),
        }
    }
}

/// Which realtime speech-to-speech service runs live sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceProviderKind {
    Gemini,
    OpenAi,
    /// Returns dummy tokens so the app can be developed without API keys.
    Fake,
}

impl FromStr for VoiceProviderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gemini" => Ok(Self::Gemini),
            "openai" => Ok(Self::OpenAi),
            "fake" => Ok(Self::Fake),
            other => anyhow::bail!("unknown voice provider '{other}' (gemini | openai | fake)"),
        }
    }
}

/// Voice provider settings. Model names always come from here, never from code.
#[derive(Clone)]
pub struct VoiceConfig {
    pub primary: VoiceProviderKind,
    pub fallback: Option<VoiceProviderKind>,
    pub gemini_api_key: Option<String>,
    pub gemini_live_model: Option<String>,
    pub gemini_base_url: String,
    pub openai_api_key: Option<String>,
    pub openai_realtime_model: Option<String>,
    /// Optional speech-to-text model for OpenAI input captions.
    pub openai_transcribe_model: Option<String>,
    pub openai_base_url: String,
    /// Planning cost per live minute, used for session_costs until real
    /// usage numbers are read from the provider.
    pub usd_per_min: f64,
}

impl VoiceConfig {
    /// A config that needs no keys: the fake provider only.
    pub fn fake() -> Self {
        Self {
            primary: VoiceProviderKind::Fake,
            fallback: None,
            gemini_api_key: None,
            gemini_live_model: None,
            gemini_base_url: "https://generativelanguage.googleapis.com".into(),
            openai_api_key: None,
            openai_realtime_model: None,
            openai_transcribe_model: None,
            openai_base_url: "https://api.openai.com".into(),
            usd_per_min: 0.03,
        }
    }

    fn from_env() -> Result<Self> {
        let fallback = match optional("VOICE_FALLBACK") {
            Some(v) => Some(v.parse()?),
            None => None,
        };
        Ok(Self {
            primary: parse_or("VOICE_PROVIDER", "fake")?,
            fallback,
            gemini_api_key: optional("GEMINI_API_KEY"),
            gemini_live_model: optional("GEMINI_LIVE_MODEL"),
            gemini_base_url: parse_or(
                "GEMINI_BASE_URL",
                "https://generativelanguage.googleapis.com",
            )?,
            openai_api_key: optional("OPENAI_API_KEY"),
            openai_realtime_model: optional("OPENAI_REALTIME_MODEL"),
            openai_transcribe_model: optional("OPENAI_TRANSCRIBE_MODEL"),
            openai_base_url: parse_or("OPENAI_BASE_URL", "https://api.openai.com")?,
            usd_per_min: parse_or("VOICE_USD_PER_MIN", "0.03")?,
        })
    }

    fn validate(&self, env: AppEnv) -> Result<()> {
        for kind in std::iter::once(self.primary).chain(self.fallback) {
            match kind {
                VoiceProviderKind::Gemini => {
                    if self.gemini_api_key.is_none() || self.gemini_live_model.is_none() {
                        anyhow::bail!("Gemini voice needs GEMINI_API_KEY and GEMINI_LIVE_MODEL");
                    }
                }
                VoiceProviderKind::OpenAi => {
                    if self.openai_api_key.is_none() || self.openai_realtime_model.is_none() {
                        anyhow::bail!(
                            "OpenAI voice needs OPENAI_API_KEY and OPENAI_REALTIME_MODEL"
                        );
                    }
                }
                VoiceProviderKind::Fake => {
                    if env == AppEnv::Production {
                        anyhow::bail!("the fake voice provider cannot run in production");
                    }
                }
            }
        }
        if self.fallback == Some(self.primary) {
            anyhow::bail!("VOICE_FALLBACK must differ from VOICE_PROVIDER");
        }
        if !(0.0..=1.0).contains(&self.usd_per_min) {
            anyhow::bail!("VOICE_USD_PER_MIN looks wrong (expected 0-1 USD)");
        }
        Ok(())
    }
}

/// Which text LLM grades finished sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraderKind {
    Gemini,
    OpenAi,
    /// Deterministic demo feedback, for building the app without keys.
    Fake,
}

impl FromStr for GraderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gemini" => Ok(Self::Gemini),
            "openai" => Ok(Self::OpenAi),
            "fake" => Ok(Self::Fake),
            other => anyhow::bail!("unknown grader '{other}' (gemini | openai | fake)"),
        }
    }
}

/// Grading settings. API keys are shared with the voice config
/// (GEMINI_API_KEY / OPENAI_API_KEY); the model is separate.
#[derive(Clone)]
pub struct GraderConfig {
    pub provider: GraderKind,
    pub model: Option<String>,
    /// Price per million tokens, for session_costs. Set from the provider's
    /// pricing page; 0 means "not set" (tokens are still logged).
    pub usd_per_1m_input: f64,
    pub usd_per_1m_output: f64,
    /// Background job workers per API process.
    pub worker_concurrency: usize,
}

impl GraderConfig {
    pub fn fake() -> Self {
        Self {
            provider: GraderKind::Fake,
            model: None,
            usd_per_1m_input: 0.0,
            usd_per_1m_output: 0.0,
            worker_concurrency: 2,
        }
    }

    fn from_env() -> Result<Self> {
        Ok(Self {
            provider: parse_or("GRADER_PROVIDER", "fake")?,
            model: optional("GRADER_MODEL"),
            usd_per_1m_input: parse_or("GRADER_USD_PER_1M_INPUT", "0")?,
            usd_per_1m_output: parse_or("GRADER_USD_PER_1M_OUTPUT", "0")?,
            worker_concurrency: parse_or("WORKER_CONCURRENCY", "2")?,
        })
    }

    fn validate(&self, env: AppEnv, voice: &VoiceConfig) -> Result<()> {
        match self.provider {
            GraderKind::Gemini => {
                if voice.gemini_api_key.is_none() || self.model.is_none() {
                    anyhow::bail!("the Gemini grader needs GEMINI_API_KEY and GRADER_MODEL");
                }
            }
            GraderKind::OpenAi => {
                if voice.openai_api_key.is_none() || self.model.is_none() {
                    anyhow::bail!("the OpenAI grader needs OPENAI_API_KEY and GRADER_MODEL");
                }
            }
            GraderKind::Fake => {
                if env == AppEnv::Production {
                    anyhow::bail!("the fake grader cannot run in production");
                }
            }
        }
        if !(1..=32).contains(&self.worker_concurrency) {
            anyhow::bail!("WORKER_CONCURRENCY must be 1-32");
        }
        if self.usd_per_1m_input < 0.0 || self.usd_per_1m_output < 0.0 {
            anyhow::bail!("grader prices cannot be negative");
        }
        Ok(())
    }
}

const DEV_SECRET_PREFIX: &str = "dev-only";

/// Runtime configuration. Deliberately not `Debug`: it holds secrets.
#[derive(Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub db_max_connections: u32,
    pub run_migrations: bool,
    pub log_format: LogFormat,
    /// HMAC key for access tokens and OTP hashing. At least 32 bytes.
    pub jwt_secret: String,
    pub access_ttl_secs: i64,
    pub refresh_ttl_days: i64,
    /// OAuth client ids allowed as the `aud` of Google ID tokens
    /// (Android, web, iOS clients each have their own).
    pub google_client_ids: Vec<String>,
    pub otp_provider: OtpProvider,
    pub voice: VoiceConfig,
    pub grader: GraderConfig,
    /// Browser origins allowed to call the API (the web app; Expo web in dev).
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let config = Self {
            app_env: parse_or("APP_ENV", "development")?,
            bind_addr: parse_or("BIND_ADDR", "127.0.0.1:8080")?,
            database_url: required("DATABASE_URL")?,
            db_max_connections: parse_or("DB_MAX_CONNECTIONS", "10")?,
            run_migrations: parse_or("RUN_MIGRATIONS", "true")?,
            log_format: parse_or("LOG_FORMAT", "pretty")?,
            jwt_secret: required("JWT_SECRET")?,
            access_ttl_secs: parse_or("ACCESS_TTL_SECS", "900")?,
            refresh_ttl_days: parse_or("REFRESH_TTL_DAYS", "30")?,
            google_client_ids: env::var("GOOGLE_CLIENT_IDS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
            otp_provider: parse_or("OTP_PROVIDER", "log")?,
            voice: VoiceConfig::from_env()?,
            grader: GraderConfig::from_env()?,
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:8081,http://127.0.0.1:8081".to_owned())
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.jwt_secret.len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 bytes");
        }
        if self.app_env == AppEnv::Production {
            if self.jwt_secret.starts_with(DEV_SECRET_PREFIX) {
                anyhow::bail!("JWT_SECRET is the development placeholder; set a real secret");
            }
            if self.otp_provider == OtpProvider::Log {
                anyhow::bail!("OTP_PROVIDER=log only prints codes; configure a real provider");
            }
        }
        self.voice.validate(self.app_env)?;
        self.grader.validate(self.app_env, &self.voice)?;
        for origin in &self.cors_origins {
            if origin == "*" {
                anyhow::bail!("CORS_ORIGINS must list origins explicitly, not '*'");
            }
            if axum::http::HeaderValue::from_str(origin).is_err() || !origin.starts_with("http") {
                anyhow::bail!("invalid CORS origin '{origin}'");
            }
        }
        Ok(())
    }
}

fn required(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required env var {key}"))
}

/// An env var that may be absent; empty counts as absent.
fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn parse_or<T>(key: &str, default: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let raw = env::var(key).unwrap_or_else(|_| default.to_owned());
    raw.parse::<T>()
        .map_err(|e| anyhow::anyhow!("invalid value for {key} ('{raw}'): {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Config {
        Config {
            app_env: AppEnv::Development,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            database_url: String::new(),
            db_max_connections: 1,
            run_migrations: false,
            log_format: LogFormat::Pretty,
            jwt_secret: "dev-only-0123456789abcdef0123456789".into(),
            access_ttl_secs: 900,
            refresh_ttl_days: 30,
            google_client_ids: vec![],
            otp_provider: OtpProvider::Log,
            voice: VoiceConfig::fake(),
            grader: GraderConfig::fake(),
            cors_origins: vec![],
        }
    }

    #[test]
    fn voice_providers_need_keys_and_models() {
        let mut c = base();
        c.voice.primary = VoiceProviderKind::Gemini;
        assert!(c.validate().is_err(), "gemini without key/model");
        c.voice.gemini_api_key = Some("k".into());
        c.voice.gemini_live_model = Some("m".into());
        assert!(c.validate().is_ok());

        c.voice.fallback = Some(VoiceProviderKind::OpenAi);
        assert!(c.validate().is_err(), "openai fallback without key/model");
        c.voice.openai_api_key = Some("k".into());
        c.voice.openai_realtime_model = Some("m".into());
        assert!(c.validate().is_ok());

        c.voice.fallback = Some(VoiceProviderKind::Gemini);
        assert!(c.validate().is_err(), "fallback equal to primary");
    }

    #[test]
    fn fake_voice_is_refused_in_production() {
        let voice = VoiceConfig::fake();
        assert!(voice.validate(AppEnv::Production).is_err());
        assert!(voice.validate(AppEnv::Development).is_ok());
    }

    #[test]
    fn log_format_parses_case_insensitively() {
        assert_eq!("JSON".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!(" pretty ".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert!("xml".parse::<LogFormat>().is_err());
    }

    #[test]
    fn short_secret_is_rejected() {
        let mut c = base();
        c.jwt_secret = "short".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn production_refuses_dev_secret_and_log_otp() {
        let mut c = base();
        c.app_env = AppEnv::Production;
        assert!(c.validate().is_err(), "dev secret must fail in production");
        c.jwt_secret = "a-real-production-secret-of-enough-length".into();
        assert!(
            c.validate().is_err(),
            "log OTP provider must fail in production"
        );
        c.app_env = AppEnv::Development;
        assert!(c.validate().is_ok());
    }
}
