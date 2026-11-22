//! Sign-in for app and web: phone OTP, Google, token refresh and sign-out.

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{otp, refresh},
    domain::users,
    error::{AppError, AppJson, AppResult, ErrorBody},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/otp/send", post(otp_send))
        .route("/auth/otp/verify", post(otp_verify))
        .route("/auth/google", post(google))
        .route("/auth/refresh", post(refresh_tokens))
        .route("/auth/logout", post(logout))
}

#[derive(Deserialize, ToSchema)]
pub struct OtpSendRequest {
    /// International format, e.g. "+923001234567".
    pub phone: String,
}

#[derive(Serialize, ToSchema)]
pub struct OtpSendResponse {
    /// Seconds until the code expires.
    pub expires_in: i64,
    /// Seconds before another code can be requested.
    pub resend_after: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct OtpVerifyRequest {
    pub phone: String,
    pub code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct GoogleLoginRequest {
    /// The ID token from Google Sign-In on the device or web.
    pub id_token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    /// Always "Bearer".
    pub token_type: String,
    /// Access token lifetime in seconds.
    pub expires_in: i64,
    /// Single-use: each refresh returns a new one. Store it securely.
    pub refresh_token: String,
    pub user_id: Uuid,
    /// True on the very first sign-in (show onboarding).
    pub new_user: bool,
}

fn token_response(
    state: &AppState,
    user_id: Uuid,
    refresh_token: String,
    new_user: bool,
) -> AppResult<TokenResponse> {
    Ok(TokenResponse {
        access_token: state.jwt.issue_access(user_id)?,
        token_type: "Bearer".into(),
        expires_in: state.jwt.access_ttl_secs(),
        refresh_token,
        user_id,
        new_user,
    })
}

/// Send a one-time sign-in code to a phone number.
#[utoipa::path(
    post, path = "/api/v1/auth/otp/send", tag = "auth",
    request_body = OtpSendRequest,
    responses(
        (status = 202, description = "Code sent", body = OtpSendResponse),
        (status = 400, description = "Invalid phone", body = ErrorBody),
        (status = 429, description = "Too many codes requested", body = ErrorBody)
    )
)]
pub async fn otp_send(
    State(state): State<AppState>,
    AppJson(req): AppJson<OtpSendRequest>,
) -> AppResult<(StatusCode, Json<OtpSendResponse>)> {
    let phone = otp::normalize_phone(&req.phone)?;

    let (sent_last_hour, seconds_since_last): (i64, Option<f64>) = sqlx::query_as(
        "SELECT count(*), EXTRACT(EPOCH FROM now() - max(created_at))::float8
         FROM otp_codes WHERE phone = $1 AND created_at > now() - interval '1 hour'",
    )
    .bind(&phone)
    .fetch_one(&state.db)
    .await?;
    if sent_last_hour >= otp::MAX_SENDS_PER_HOUR
        || seconds_since_last.is_some_and(|s| s < otp::RESEND_AFTER_SECS as f64)
    {
        return Err(AppError::TooManyRequests);
    }

    let code = otp::generate_code();
    sqlx::query(
        "INSERT INTO otp_codes (phone, code_hash, expires_at)
         VALUES ($1, $2, now() + make_interval(secs => $3))",
    )
    .bind(&phone)
    .bind(otp::hash_code(
        state.config.jwt_secret.as_bytes(),
        &phone,
        &code,
    ))
    .bind(otp::CODE_TTL_SECS as f64)
    .execute(&state.db)
    .await?;

    state
        .otp
        .send(&phone, &code)
        .await
        .map_err(|e| AppError::Internal(e.context("sending one-time code")))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(OtpSendResponse {
            expires_in: otp::CODE_TTL_SECS,
            resend_after: otp::RESEND_AFTER_SECS,
        }),
    ))
}

/// Check a one-time code; signs in (or signs up) and returns tokens.
#[utoipa::path(
    post, path = "/api/v1/auth/otp/verify", tag = "auth",
    request_body = OtpVerifyRequest,
    responses(
        (status = 200, description = "Signed in", body = TokenResponse),
        (status = 400, description = "Wrong or expired code", body = ErrorBody),
        (status = 429, description = "Too many wrong attempts", body = ErrorBody)
    )
)]
pub async fn otp_verify(
    State(state): State<AppState>,
    AppJson(req): AppJson<OtpVerifyRequest>,
) -> AppResult<Json<TokenResponse>> {
    let phone = otp::normalize_phone(&req.phone)?;
    let mut tx = state.db.begin().await?;

    let row: Option<(Uuid, Vec<u8>, i32)> = sqlx::query_as(
        "SELECT id, code_hash, attempts FROM otp_codes
         WHERE phone = $1 AND consumed_at IS NULL AND expires_at > now()
         ORDER BY created_at DESC LIMIT 1
         FOR UPDATE",
    )
    .bind(&phone)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((code_id, code_hash, attempts)) = row else {
        return Err(AppError::InvalidCode);
    };
    if attempts >= otp::MAX_ATTEMPTS {
        return Err(AppError::TooManyRequests);
    }

    let secret = state.config.jwt_secret.as_bytes();
    if !otp::code_matches(&code_hash, secret, &phone, &req.code) {
        sqlx::query("UPDATE otp_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(code_id)
            .execute(&mut *tx)
            .await?;
        // Commit so the failed attempt counts even though we return an error.
        tx.commit().await?;
        return Err(AppError::InvalidCode);
    }

    sqlx::query("UPDATE otp_codes SET consumed_at = now() WHERE id = $1")
        .bind(code_id)
        .execute(&mut *tx)
        .await?;
    let (user_id, created) = users::upsert_by_phone(&mut tx, &phone).await?;
    let issued = refresh::issue(&mut tx, user_id, None, state.config.refresh_ttl_days).await?;
    tx.commit().await?;

    if created {
        tracing::info!(%user_id, method = "phone", "new user signed up");
    }
    Ok(Json(token_response(
        &state,
        user_id,
        issued.token,
        created,
    )?))
}

/// Sign in with a Google ID token.
#[utoipa::path(
    post, path = "/api/v1/auth/google", tag = "auth",
    request_body = GoogleLoginRequest,
    responses(
        (status = 200, description = "Signed in", body = TokenResponse),
        (status = 401, description = "Invalid Google token", body = ErrorBody),
        (status = 409, description = "Email belongs to another Google account", body = ErrorBody),
        (status = 503, description = "Google sign-in not configured or unreachable", body = ErrorBody)
    )
)]
pub async fn google(
    State(state): State<AppState>,
    AppJson(req): AppJson<GoogleLoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    let ident = state.google.verify(&req.id_token).await?;
    let mut tx = state.db.begin().await?;
    let (user_id, created) = users::find_or_create_google(&mut tx, &ident).await?;
    let issued = refresh::issue(&mut tx, user_id, None, state.config.refresh_ttl_days).await?;
    tx.commit().await?;

    if created {
        tracing::info!(%user_id, method = "google", "new user signed up");
    }
    Ok(Json(token_response(
        &state,
        user_id,
        issued.token,
        created,
    )?))
}

/// Exchange a refresh token for a new access + refresh token pair.
#[utoipa::path(
    post, path = "/api/v1/auth/refresh", tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "New tokens", body = TokenResponse),
        (status = 401, description = "Token invalid, expired or reused (sign in again)", body = ErrorBody)
    )
)]
pub async fn refresh_tokens(
    State(state): State<AppState>,
    AppJson(req): AppJson<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    let rotated =
        refresh::rotate(&state.db, &req.refresh_token, state.config.refresh_ttl_days).await?;
    Ok(Json(token_response(
        &state,
        rotated.user_id,
        rotated.refresh_token,
        false,
    )?))
}

/// Sign out this device: revokes its refresh token family.
#[utoipa::path(
    post, path = "/api/v1/auth/logout", tag = "auth",
    request_body = RefreshRequest,
    responses((status = 204, description = "Signed out"))
)]
pub async fn logout(
    State(state): State<AppState>,
    AppJson(req): AppJson<RefreshRequest>,
) -> AppResult<StatusCode> {
    refresh::revoke_family(&state.db, &req.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}
