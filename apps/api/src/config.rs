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

/// Runtime configuration. Deliberately not `Debug`: it will hold secrets.
#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub db_max_connections: u32,
    pub run_migrations: bool,
    pub log_format: LogFormat,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bind_addr: parse_or("BIND_ADDR", "127.0.0.1:8080")?,
            database_url: required("DATABASE_URL")?,
            db_max_connections: parse_or("DB_MAX_CONNECTIONS", "10")?,
            run_migrations: parse_or("RUN_MIGRATIONS", "true")?,
            log_format: parse_or("LOG_FORMAT", "pretty")?,
        })
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

    #[test]
    fn log_format_parses_case_insensitively() {
        assert_eq!("JSON".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!(" pretty ".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert!("xml".parse::<LogFormat>().is_err());
    }
}
