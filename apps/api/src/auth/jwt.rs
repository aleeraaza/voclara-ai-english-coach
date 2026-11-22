//! Short-lived access tokens (HS256 JWT). The app sends them as
//! `Authorization: Bearer <token>`; they are never stored server-side.

use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const TOKEN_TYPE: &str = "access";
const ISSUER: &str = "voclara-api";

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
    /// Guards against a refresh token or another JWT being used as an access token.
    pub typ: String,
}

pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    access_ttl_secs: i64,
}

impl JwtKeys {
    pub fn new(secret: &[u8], access_ttl_secs: i64) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.leeway = 30;
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            validation,
            access_ttl_secs,
        }
    }

    pub fn access_ttl_secs(&self) -> i64 {
        self.access_ttl_secs
    }

    /// Issue an access token for `user_id`.
    pub fn issue_access(&self, user_id: Uuid) -> AppResult<String> {
        let now = Utc::now().timestamp();
        let claims = AccessClaims {
            sub: user_id,
            iat: now,
            exp: now + self.access_ttl_secs,
            iss: ISSUER.to_owned(),
            typ: TOKEN_TYPE.to_owned(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("signing access token: {e}")))
    }

    /// Verify signature, issuer, expiry and type; return the user id.
    pub fn verify_access(&self, token: &str) -> AppResult<Uuid> {
        let data = decode::<AccessClaims>(token, &self.decoding, &self.validation)
            .map_err(|_| AppError::Unauthorized)?;
        if data.claims.typ != TOKEN_TYPE {
            return Err(AppError::Unauthorized);
        }
        Ok(data.claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"dev-only-0123456789abcdef0123456789";

    #[test]
    fn round_trip() {
        let keys = JwtKeys::new(SECRET, 900);
        let user = Uuid::new_v4();
        let token = keys.issue_access(user).unwrap();
        assert_eq!(keys.verify_access(&token).unwrap(), user);
    }

    #[test]
    fn wrong_secret_is_rejected() {
        let token = JwtKeys::new(SECRET, 900)
            .issue_access(Uuid::new_v4())
            .unwrap();
        let other = JwtKeys::new(b"another-secret-that-is-long-enough!!", 900);
        assert!(matches!(
            other.verify_access(&token),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn expired_token_is_rejected() {
        // TTL far in the past, beyond the 30 s leeway.
        let keys = JwtKeys::new(SECRET, -120);
        let token = keys.issue_access(Uuid::new_v4()).unwrap();
        assert!(matches!(
            keys.verify_access(&token),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn garbage_is_rejected() {
        let keys = JwtKeys::new(SECRET, 900);
        assert!(keys.verify_access("not.a.jwt").is_err());
    }
}
