pub mod pterodactyl;

use anyhow::Context;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const KINDS: [&str; 4] = ["pterodactyl", "proxmox", "openstack", "virtfusion"];

#[derive(Serialize, sqlx::FromRow)]
pub struct IntegrationSummary {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub base_url: String,
    pub settings: Value,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_checked_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

pub enum Driver {
    Pterodactyl(pterodactyl::Pterodactyl),
}

impl Driver {
    pub fn pterodactyl(&self) -> AppResult<&pterodactyl::Pterodactyl> {
        match self {
            Self::Pterodactyl(client) => Ok(client),
        }
    }

    pub fn into_pterodactyl(self) -> AppResult<pterodactyl::Pterodactyl> {
        match self {
            Self::Pterodactyl(client) => Ok(client),
        }
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        match self {
            Self::Pterodactyl(client) => client.test().await,
        }
    }

    pub async fn suspend(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.suspend(external_id).await,
        }
    }

    pub async fn unsuspend(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.unsuspend(external_id).await,
        }
    }

    pub async fn terminate(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.terminate(external_id).await,
        }
    }
}

pub fn seal_credentials(state: &AppState, credentials: &impl Serialize) -> AppResult<Vec<u8>> {
    let secrets = state.config.secrets.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("SECRET_KEY ontbreekt in de configuratie"))
    })?;
    Ok(secrets.seal(&serde_json::to_vec(credentials).map_err(anyhow::Error::from)?)?)
}

pub async fn driver(state: &AppState, integration_id: Uuid) -> AppResult<Driver> {
    let (kind, base_url, sealed): (String, String, Vec<u8>) =
        sqlx::query_as("SELECT kind, base_url, credentials FROM integrations WHERE id = $1")
            .bind(integration_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    let secrets = state.config.secrets.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("SECRET_KEY ontbreekt in de configuratie"))
    })?;
    let credentials = secrets.open(&sealed)?;
    match kind.as_str() {
        "pterodactyl" => {
            let credentials =
                serde_json::from_slice(&credentials).context("ongeldige Pterodactyl-sleutels")?;
            Ok(Driver::Pterodactyl(pterodactyl::Pterodactyl::new(
                &base_url,
                credentials,
            )?))
        }
        other => Err(AppError::Internal(anyhow::anyhow!(
            "koppeling {other} wordt nog niet ondersteund"
        ))),
    }
}
