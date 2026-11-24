use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::limits::Limits;
use crate::mail::Mailer;
use crate::updates::Updates;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub mailer: Arc<Mailer>,
    pub updates: Arc<Updates>,
    pub limits: Arc<Limits>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            db,
            updates: Arc::new(Updates::new(
                config.update_dir.clone(),
                crate::updates::default_check_url(),
            )),
            config: Arc::new(config),
            mailer: Arc::new(Mailer::from_env()?),
            limits: Arc::new(Limits::default()),
        })
    }
}
