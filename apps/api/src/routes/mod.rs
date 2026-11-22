pub mod health;

use axum::Router;

use crate::state::AppState;

/// Everything under `/api/v1`. Auth, sessions, scores and billing routes are
/// added here in the next steps; all of them will sit behind bearer auth.
pub fn v1() -> Router<AppState> {
    Router::new()
}
