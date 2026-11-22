//! `AuthUser` extractor: add it as a handler argument and the route requires a
//! valid bearer access token. Missing or bad tokens become a 401 `AppError`.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or(AppError::Unauthorized)?;
        let id = state.jwt.verify_access(token.trim())?;
        Ok(Self { id })
    }
}
