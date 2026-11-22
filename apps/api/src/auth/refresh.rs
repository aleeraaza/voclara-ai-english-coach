//! Rotating refresh tokens.
//!
//! - The client gets a random 256-bit token; the database keeps only its SHA-256.
//! - Every refresh replaces the token with a new one in the same *family*.
//! - Presenting a token that was already replaced means it leaked (or the
//!   client raced itself): the whole family is revoked and the user must sign in
//!   again. Clients must therefore never refresh twice in parallel.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub struct IssuedRefresh {
    pub id: Uuid,
    /// The raw token. Shown to the client once, never stored.
    pub token: String,
}

/// Result of a successful rotation.
pub struct Rotated {
    pub user_id: Uuid,
    pub refresh_token: String,
}

fn new_raw_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(raw: &str) -> Vec<u8> {
    Sha256::digest(raw.as_bytes()).to_vec()
}

/// Insert a new refresh token. `family_id = None` starts a new family (a fresh sign-in).
///
/// Takes `&mut PgConnection` so callers can pass a transaction (`&mut *tx`):
/// `Transaction` derefs to the connection it wraps, so the insert joins it.
pub async fn issue(
    conn: &mut PgConnection,
    user_id: Uuid,
    family_id: Option<Uuid>,
    ttl_days: i64,
) -> AppResult<IssuedRefresh> {
    let token = new_raw_token();
    let expires_at = Utc::now() + Duration::days(ttl_days);
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO refresh_tokens (user_id, family_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(user_id)
    .bind(family_id.unwrap_or_else(Uuid::new_v4))
    .bind(hash_token(&token))
    .bind(expires_at)
    .fetch_one(conn)
    .await?;
    Ok(IssuedRefresh { id, token })
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

/// Swap a valid refresh token for a new one. Reuse of a replaced token revokes
/// the family and returns 401.
pub async fn rotate(db: &PgPool, presented: &str, ttl_days: i64) -> AppResult<Rotated> {
    let mut tx = db.begin().await?;
    // FOR UPDATE: two refreshes with the same token are serialised, so exactly
    // one rotates and the other sees `revoked_at` set.
    let row: Option<TokenRow> = sqlx::query_as(
        "SELECT id, user_id, family_id, expires_at, revoked_at
         FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE",
    )
    .bind(hash_token(presented))
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Err(AppError::Unauthorized);
    };

    if row.revoked_at.is_some() {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now()
             WHERE family_id = $1 AND revoked_at IS NULL",
        )
        .bind(row.family_id)
        .execute(&mut *tx)
        .await?;
        // Commit before returning the error, or the revocation would roll back.
        tx.commit().await?;
        tracing::warn!(user_id = %row.user_id, family_id = %row.family_id,
            "refresh token reuse detected; family revoked");
        return Err(AppError::Unauthorized);
    }

    if row.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }

    let next = issue(&mut tx, row.user_id, Some(row.family_id), ttl_days).await?;
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $2 WHERE id = $1")
        .bind(row.id)
        .bind(next.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Rotated {
        user_id: row.user_id,
        refresh_token: next.token,
    })
}

/// Sign out: revoke the family of the presented token. Unknown tokens are ignored.
pub async fn revoke_family(db: &PgPool, presented: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now()
         WHERE revoked_at IS NULL
           AND family_id = (SELECT family_id FROM refresh_tokens WHERE token_hash = $1)",
    )
    .bind(hash_token(presented))
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_tokens_are_long_and_unique() {
        let a = new_raw_token();
        let b = new_raw_token();
        assert_eq!(a.len(), 43); // 32 bytes, base64url without padding
        assert_ne!(a, b);
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }
}
