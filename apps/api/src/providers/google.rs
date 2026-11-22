//! Google Sign-In: verify an ID token from the app or web against Google's
//! published signing keys (JWKS), our client ids and Google's issuer.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Deserializer};
use tokio::sync::RwLock;

use crate::error::{AppError, AppResult};

const JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];
/// Google rotates keys every few days; an hour of caching is safe.
const KEYS_TTL: Duration = Duration::from_secs(3600);
/// Unknown `kid`s trigger a refetch, but at most this often (stops abuse).
const MIN_REFETCH: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleIdentity {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
}

#[async_trait]
pub trait GoogleVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> AppResult<GoogleIdentity>;
}

#[derive(Deserialize)]
struct GoogleClaims {
    sub: String,
    email: Option<String>,
    #[serde(default, deserialize_with = "bool_or_string")]
    email_verified: bool,
    name: Option<String>,
}

/// Google has sent `email_verified` both as `true` and as `"true"`.
fn bool_or_string<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum B {
        Bool(bool),
        Str(String),
    }
    Ok(match B::deserialize(d)? {
        B::Bool(b) => b,
        B::Str(s) => s.eq_ignore_ascii_case("true"),
    })
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

struct KeyCache {
    keys: HashMap<String, DecodingKey>,
    fetched_at: Instant,
}

pub struct GoogleJwksVerifier {
    http: reqwest::Client,
    client_ids: Vec<String>,
    cache: RwLock<Option<KeyCache>>,
}

impl GoogleJwksVerifier {
    pub fn new(client_ids: Vec<String>) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            http,
            client_ids,
            cache: RwLock::new(None),
        })
    }

    async fn fetch_keys(&self) -> AppResult<HashMap<String, DecodingKey>> {
        let jwks: Jwks = self
            .http
            .get(JWKS_URL)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| {
                tracing::error!(error = %e, "fetching Google JWKS failed");
                AppError::Unavailable
            })?
            .json()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "parsing Google JWKS failed");
                AppError::Unavailable
            })?;
        let mut keys = HashMap::new();
        for k in jwks.keys {
            match DecodingKey::from_rsa_components(&k.n, &k.e) {
                Ok(key) => {
                    keys.insert(k.kid, key);
                }
                Err(e) => tracing::warn!(kid = %k.kid, error = %e, "skipping bad Google key"),
            }
        }
        Ok(keys)
    }

    async fn key_for(&self, kid: &str) -> AppResult<DecodingKey> {
        // Fast path under a read lock. The guard is dropped at the end of this
        // block, before we await the network, so other requests are not blocked.
        {
            let guard = self.cache.read().await;
            if let Some(cache) = guard.as_ref() {
                let age = cache.fetched_at.elapsed();
                if let Some(key) = cache.keys.get(kid).filter(|_| age < KEYS_TTL) {
                    return Ok(key.clone());
                }
                if age < MIN_REFETCH {
                    return Err(AppError::Unauthorized);
                }
            }
        }
        let keys = self.fetch_keys().await?;
        let key = keys.get(kid).cloned();
        *self.cache.write().await = Some(KeyCache {
            keys,
            fetched_at: Instant::now(),
        });
        key.ok_or(AppError::Unauthorized)
    }
}

#[async_trait]
impl GoogleVerifier for GoogleJwksVerifier {
    async fn verify(&self, id_token: &str) -> AppResult<GoogleIdentity> {
        if self.client_ids.is_empty() {
            tracing::error!("Google sign-in called but GOOGLE_CLIENT_IDS is not set");
            return Err(AppError::Unavailable);
        }
        let header = decode_header(id_token).map_err(|_| AppError::Unauthorized)?;
        if header.alg != Algorithm::RS256 {
            return Err(AppError::Unauthorized);
        }
        let kid = header.kid.ok_or(AppError::Unauthorized)?;
        let key = self.key_for(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&self.client_ids);
        validation.set_issuer(&ISSUERS);
        let claims = decode::<GoogleClaims>(id_token, &key, &validation)
            .map_err(|_| AppError::Unauthorized)?
            .claims;

        Ok(GoogleIdentity {
            sub: claims.sub,
            email: claims.email.map(|e| e.trim().to_lowercase()),
            email_verified: claims.email_verified,
            name: claims.name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_verified_accepts_bool_and_string() {
        let a: GoogleClaims = serde_json::from_str(r#"{"sub":"1","email_verified":true}"#).unwrap();
        let b: GoogleClaims =
            serde_json::from_str(r#"{"sub":"1","email_verified":"true"}"#).unwrap();
        let c: GoogleClaims = serde_json::from_str(r#"{"sub":"1"}"#).unwrap();
        assert!(a.email_verified && b.email_verified && !c.email_verified);
    }

    #[tokio::test]
    async fn unconfigured_verifier_is_unavailable() {
        let v = GoogleJwksVerifier::new(vec![]).unwrap();
        assert!(matches!(v.verify("x").await, Err(AppError::Unavailable)));
    }

    #[tokio::test]
    async fn malformed_token_is_unauthorized() {
        let v = GoogleJwksVerifier::new(vec!["client".into()]).unwrap();
        assert!(matches!(
            v.verify("not-a-jwt").await,
            Err(AppError::Unauthorized)
        ));
    }
}
