use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

/// Shared state handed to every handler. Cheap to clone: `PgPool` is already a
/// handle around an `Arc`, and the config sits behind one.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }
}
