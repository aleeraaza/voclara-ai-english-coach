pub mod auth;
pub mod feedback;
pub mod health;
pub mod me;
pub mod sessions;

use axum::Router;

use crate::state::AppState;

/// Everything under `/api/v1`. Routes that need a signed-in user take an
/// `AuthUser` argument, which enforces the bearer token.
pub fn v1() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(me::router())
        .merge(sessions::router())
        .merge(feedback::router())
}
