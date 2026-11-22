//! Tracing setup: JSON logs in production, readable logs in development.

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config::LogFormat;

pub fn init(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("voclara_api=info,tower_http=info,sqlx=warn"));

    let registry = tracing_subscriber::registry().with(filter);
    match format {
        LogFormat::Json => registry
            .with(
                fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true),
            )
            .init(),
        LogFormat::Pretty => registry.with(fmt::layer()).init(),
    }
}
