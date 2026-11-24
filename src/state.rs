use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::mail::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub mailer: Arc<Mailer>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            db,
            config: Arc::new(config),
            mailer: Arc::new(Mailer::from_env()?),
        })
    }
}
