//! Shared helpers for integration tests: fake providers, a test app and a
//! database handle. Each test binary uses a different subset of these.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rand::Rng;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use voclara_api::{
    AppState,
    config::{AppEnv, Config, LogFormat, OtpProvider, VoiceConfig},
    error::{AppError, AppResult},
    providers::{
        google::{GoogleIdentity, GoogleVerifier},
        otp::OtpSender,
        voice::{RelayTarget, SessionConfig, Transport, VoiceProvider, VoiceProviders, VoiceToken},
    },
    state::Providers,
};

pub const TEST_SECRET: &str = "dev-only-test-secret-0123456789abcdef";

pub fn test_config(database_url: &str) -> Config {
    Config {
        app_env: AppEnv::Development,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        database_url: database_url.into(),
        db_max_connections: 5,
        run_migrations: false,
        log_format: LogFormat::Pretty,
        jwt_secret: TEST_SECRET.into(),
        access_ttl_secs: 900,
        refresh_ttl_days: 30,
        google_client_ids: vec!["test-client".into()],
        otp_provider: OtpProvider::Log,
        voice: VoiceConfig::fake(),
    }
}

/// Voice provider for tests: records what it was asked for and can be told to fail.
pub struct TestVoice {
    pub name: &'static str,
    pub fail: std::sync::atomic::AtomicBool,
    pub last: Mutex<Option<(SessionConfig, u32)>>,
}

impl TestVoice {
    pub fn new(name: &'static str) -> Arc<Self> {
        Arc::new(Self {
            name,
            fail: false.into(),
            last: Mutex::new(None),
        })
    }

    pub fn set_failing(&self, fail: bool) {
        self.fail.store(fail, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn last_prompt(&self) -> String {
        let last = self.last.lock().unwrap();
        last.as_ref()
            .expect("no token minted")
            .0
            .system_prompt
            .clone()
    }

    pub fn last_ttl(&self) -> u32 {
        self.last
            .lock()
            .unwrap()
            .as_ref()
            .expect("no token minted")
            .1
    }
}

#[async_trait]
impl VoiceProvider for TestVoice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn model(&self) -> &str {
        "test-model"
    }

    async fn mint_ephemeral_token(
        &self,
        cfg: &SessionConfig,
        ttl_secs: u32,
    ) -> AppResult<VoiceToken> {
        if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AppError::Unavailable);
        }
        *self.last.lock().unwrap() = Some((cfg.clone(), ttl_secs));
        Ok(VoiceToken {
            token: format!("{}-token", self.name),
            token_expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            connect_url: format!("wss://{}.test/ws", self.name),
            transport: Transport::Websocket,
            hard_cutoff: true,
        })
    }

    fn relay_target(&self) -> AppResult<RelayTarget> {
        Ok(RelayTarget {
            url: "ws://test".into(),
            headers: vec![],
        })
    }
}

/// Records codes instead of sending them, so tests can read them back.
#[derive(Default)]
pub struct FakeOtp {
    sent: Mutex<Vec<(String, String)>>,
}

impl FakeOtp {
    pub fn last_code(&self, phone: &str) -> String {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(p, _)| p == phone)
            .map(|(_, c)| c.clone())
            .expect("no code sent to this phone")
    }
}

#[async_trait]
impl OtpSender for FakeOtp {
    async fn send(&self, phone: &str, code: &str) -> anyhow::Result<()> {
        self.sent.lock().unwrap().push((phone.into(), code.into()));
        Ok(())
    }
}

/// Accepts tokens shaped `google:<sub>:<email>:<verified>`; anything else is 401.
pub struct FakeGoogle;

#[async_trait]
impl GoogleVerifier for FakeGoogle {
    async fn verify(&self, id_token: &str) -> AppResult<GoogleIdentity> {
        let parts: Vec<&str> = id_token.split(':').collect();
        match parts.as_slice() {
            ["google", sub, email, verified] => Ok(GoogleIdentity {
                sub: (*sub).into(),
                email: Some(email.to_lowercase()).filter(|e| !e.is_empty()),
                email_verified: *verified == "true",
                name: Some("Test User".into()),
            }),
            _ => Err(AppError::Unauthorized),
        }
    }
}

pub struct TestApp {
    pub router: Router,
    pub otp: Arc<FakeOtp>,
    pub voice: Arc<TestVoice>,
    pub fallback: Option<Arc<TestVoice>>,
    pub db: PgPool,
}

pub fn build(db: PgPool) -> TestApp {
    build_with(db, false)
}

/// `with_fallback` adds a second test provider named "backup".
pub fn build_with(db: PgPool, with_fallback: bool) -> TestApp {
    let otp = Arc::new(FakeOtp::default());
    let voice = TestVoice::new("primary");
    let fallback = with_fallback.then(|| TestVoice::new("backup"));
    let providers = VoiceProviders::new(
        voice.clone(),
        fallback.clone().map(|f| f as Arc<dyn VoiceProvider>),
    );
    let state = AppState::new(
        db.clone(),
        test_config("postgres://unused"),
        Providers {
            otp: otp.clone(),
            google: Arc::new(FakeGoogle),
            voice: Arc::new(providers),
        },
    )
    .unwrap();
    TestApp {
        router: voclara_api::app(state),
        otp,
        voice,
        fallback,
        db,
    }
}

/// App whose pool points at a closed port: fine for routes that skip the DB.
pub fn build_without_db() -> TestApp {
    let db = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://voclara:voclara@127.0.0.1:1/voclara")
        .unwrap();
    build(db)
}

/// Real database from `DATABASE_URL`, migrated. `None` (test skips) if unset.
pub async fn test_db() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping database test");
        return None;
    };
    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&url)
        .await
        .unwrap();
    voclara_api::MIGRATOR.run(&db).await.unwrap();
    Some(db)
}

impl TestApp {
    pub async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        bearer: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(token) = bearer {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let req = match body {
            Some(json) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    pub async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(body), None).await
    }

    /// Full phone sign-in; returns the token response.
    pub async fn login_by_phone(&self, phone: &str) -> Value {
        let (s, _) = self
            .post(
                "/api/v1/auth/otp/send",
                serde_json::json!({ "phone": phone }),
            )
            .await;
        assert_eq!(s, StatusCode::ACCEPTED);
        let code = self.otp.last_code(phone);
        let (s, body) = self
            .post(
                "/api/v1/auth/otp/verify",
                serde_json::json!({ "phone": phone, "code": code }),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "verify failed: {body}");
        body
    }
}

/// A random Pakistani mobile number, so tests never collide.
pub fn random_phone() -> String {
    format!("+923{:09}", rand::rng().random_range(0..1_000_000_000u32))
}
