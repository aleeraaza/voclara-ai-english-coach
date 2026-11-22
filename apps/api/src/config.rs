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
        Ok(())
    }
}

fn required(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required env var {key}"))
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
        }
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
