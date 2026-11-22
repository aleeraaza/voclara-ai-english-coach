use std::{sync::Arc, time::Duration};

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use voclara_api::{
    AppState, MIGRATOR,
    config::{Config, GraderKind, OtpProvider, VoiceProviderKind},
    providers::{
        google::{GoogleJwksVerifier, GoogleVerifier},
        grader,
        otp::{LogOtpSender, OtpSender},
        voice,
    },
    state::Providers,
    telemetry, worker,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A missing .env is fine (production uses real env vars).
    let _ = dotenvy::dotenv();
    let config = Config::from_env()?;
    telemetry::init(config.log_format);

    let db = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .context("connecting to Postgres")?;

    if config.run_migrations {
        MIGRATOR.run(&db).await.context("running migrations")?;
        tracing::info!("migrations up to date");
    }

    let otp: Arc<dyn OtpSender> = match config.otp_provider {
        OtpProvider::Log => {
            tracing::warn!("OTP_PROVIDER=log: sign-in codes are printed to the log, not sent");
            Arc::new(LogOtpSender)
        }
    };
    if config.google_client_ids.is_empty() {
        tracing::warn!("GOOGLE_CLIENT_IDS not set: Google sign-in will return 503");
    }
    let google: Arc<dyn GoogleVerifier> =
        Arc::new(GoogleJwksVerifier::new(config.google_client_ids.clone())?);

    if config.voice.primary == VoiceProviderKind::Fake {
        tracing::warn!("VOICE_PROVIDER=fake: sessions get dummy tokens, no real voice");
    }
    let voice = Arc::new(voice::from_config(&config.voice)?);
    tracing::info!(
        provider = voice.primary().name(),
        model = voice.primary().model(),
        fallback = ?config.voice.fallback,
        "voice provider ready"
    );

    if config.grader.provider == GraderKind::Fake {
        tracing::warn!("GRADER_PROVIDER=fake: feedback is demo data, not a real grade");
    }
    let grader = grader::from_config(&config)?;
    tracing::info!(
        grader = grader.name(),
        model = grader.model(),
        "grader ready"
    );

    let addr = config.bind_addr;
    let workers = config.grader.worker_concurrency;
    let state = AppState::new(
        db,
        config,
        Providers {
            otp,
            google,
            voice,
            grader,
        },
    )?;
    // Background jobs (grading) run in this process; they stop when it exits
    // and an interrupted job is picked up again on the next start.
    let _workers = worker::spawn(state.clone(), workers);
    tracing::info!(workers, "job workers started");
    let app = voclara_api::app(state);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "voclara-api listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    tracing::info!("shut down cleanly");
    Ok(())
}

/// Resolve on Ctrl+C, or on SIGTERM (what Docker sends) on Unix.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
