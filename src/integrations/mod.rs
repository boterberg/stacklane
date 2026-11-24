pub mod proxmox;
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
    Proxmox(proxmox::Proxmox),
}

fn wrong_kind() -> AppError {
    AppError::Internal(anyhow::anyhow!(
        "deze dienst hoort bij een ander soort koppeling"
    ))
}

fn vmid(external_id: &str) -> anyhow::Result<u32> {
    external_id.parse().context("ongeldig VM-nummer")
}

impl Driver {
    pub fn pterodactyl(&self) -> AppResult<&pterodactyl::Pterodactyl> {
        match self {
            Self::Pterodactyl(client) => Ok(client),
            _ => Err(wrong_kind()),
        }
    }

    pub fn into_pterodactyl(self) -> AppResult<pterodactyl::Pterodactyl> {
        match self {
            Self::Pterodactyl(client) => Ok(client),
            _ => Err(wrong_kind()),
        }
    }

    pub fn proxmox(&self) -> AppResult<&proxmox::Proxmox> {
        match self {
            Self::Proxmox(client) => Ok(client),
            _ => Err(wrong_kind()),
        }
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        match self {
            Self::Pterodactyl(client) => client.test().await,
            Self::Proxmox(client) => client.test().await,
        }
    }

    pub async fn suspend(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.suspend(external_id).await,
            Self::Proxmox(client) => client.power(vmid(external_id)?, "stop").await,
        }
    }

    pub async fn unsuspend(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.unsuspend(external_id).await,
            Self::Proxmox(client) => client.power(vmid(external_id)?, "start").await,
        }
    }

    pub async fn terminate(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.terminate(external_id).await,
            Self::Proxmox(client) => client.destroy(vmid(external_id)?).await,
        }
    }
}

pub fn seal_credentials(state: &AppState, credentials: &impl Serialize) -> AppResult<Vec<u8>> {
    let secrets = state.config.secrets.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("SECRET_KEY ontbreekt in de configuratie"))
    })?;
    Ok(secrets.seal(&serde_json::to_vec(credentials).map_err(anyhow::Error::from)?)?)
}

pub fn build(kind: &str, base_url: &str, credentials: &[u8], settings: Value) -> AppResult<Driver> {
    match kind {
        "pterodactyl" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige Pterodactyl-sleutels")?;
            Ok(Driver::Pterodactyl(pterodactyl::Pterodactyl::new(
                base_url,
                credentials,
            )?))
        }
        "proxmox" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige Proxmox-sleutels")?;
            let settings =
                serde_json::from_value(settings).context("ongeldige Proxmox-instellingen")?;
            Ok(Driver::Proxmox(proxmox::Proxmox::new(
                base_url,
                credentials,
                settings,
            )?))
        }
        other => Err(AppError::Internal(anyhow::anyhow!(
            "koppeling {other} wordt nog niet ondersteund"
        ))),
    }
}

pub async fn driver(state: &AppState, integration_id: Uuid) -> AppResult<Driver> {
    let (kind, base_url, sealed, settings): (String, String, Vec<u8>, Value) = sqlx::query_as(
        "SELECT kind, base_url, credentials, settings FROM integrations WHERE id = $1",
    )
    .bind(integration_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let secrets = state.config.secrets.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("SECRET_KEY ontbreekt in de configuratie"))
    })?;
    build(&kind, &base_url, &secrets.open(&sealed)?, settings)
}
