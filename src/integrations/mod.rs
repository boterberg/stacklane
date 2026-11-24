pub mod directadmin;
pub mod openprovider;
pub mod openstack;
pub mod plesk;
pub mod proxmox;
pub mod pterodactyl;
pub mod virtfusion;
pub mod vps;
pub mod webhosting;

use anyhow::Context;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const KINDS: [&str; 6] = [
    "pterodactyl",
    "proxmox",
    "openstack",
    "virtfusion",
    "directadmin",
    "plesk",
];

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
    DirectAdmin(directadmin::DirectAdmin),
    Plesk(plesk::Plesk),
    OpenStack(openstack::OpenStack),
    VirtFusion(virtfusion::VirtFusion),
    Openprovider(openprovider::Openprovider),
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

    pub async fn test(&self) -> anyhow::Result<String> {
        match self {
            Self::Pterodactyl(client) => client.test().await,
            Self::Proxmox(client) => client.test().await,
            Self::DirectAdmin(client) => client.test().await,
            Self::Plesk(client) => client.test().await,
            Self::OpenStack(client) => client.test().await,
            Self::VirtFusion(client) => client.test().await,
            Self::Openprovider(client) => client.test().await,
        }
    }

    pub async fn suspend(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.suspend(external_id).await,
            Self::Proxmox(client) => client.power(vmid(external_id)?, "stop").await,
            Self::DirectAdmin(client) => client.suspend(external_id).await,
            Self::Plesk(client) => client.suspend(webhosting::plesk_parts(external_id).1).await,
            Self::OpenStack(client) => client.power(external_id, "stop").await,
            Self::VirtFusion(client) => client.suspend(external_id).await,
            Self::Openprovider(_) => Ok(()),
        }
    }

    pub async fn unsuspend(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.unsuspend(external_id).await,
            Self::Proxmox(client) => client.power(vmid(external_id)?, "start").await,
            Self::DirectAdmin(client) => client.unsuspend(external_id).await,
            Self::Plesk(client) => {
                client
                    .unsuspend(webhosting::plesk_parts(external_id).1)
                    .await
            }
            Self::OpenStack(client) => client.power(external_id, "start").await,
            Self::VirtFusion(client) => client.unsuspend(external_id).await,
            Self::Openprovider(_) => Ok(()),
        }
    }

    pub async fn terminate(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Pterodactyl(client) => client.terminate(external_id).await,
            Self::Proxmox(client) => client.destroy(vmid(external_id)?).await,
            Self::DirectAdmin(client) => client.terminate(external_id).await,
            Self::Plesk(client) => {
                let (username, domain) = webhosting::plesk_parts(external_id);
                client.terminate(domain, username).await
            }
            Self::OpenStack(client) => client.destroy(external_id).await,
            Self::VirtFusion(client) => client.destroy(external_id).await,
            Self::Openprovider(_) => Ok(()),
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
        "directadmin" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige DirectAdmin-gegevens")?;
            let settings =
                serde_json::from_value(settings).context("ongeldige DirectAdmin-instellingen")?;
            Ok(Driver::DirectAdmin(directadmin::DirectAdmin::new(
                base_url,
                credentials,
                settings,
            )?))
        }
        "plesk" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige Plesk-gegevens")?;
            let settings =
                serde_json::from_value(settings).context("ongeldige Plesk-instellingen")?;
            Ok(Driver::Plesk(plesk::Plesk::new(
                base_url,
                credentials,
                settings,
            )?))
        }
        "openstack" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige OpenStack-sleutels")?;
            let settings =
                serde_json::from_value(settings).context("ongeldige OpenStack-instellingen")?;
            Ok(Driver::OpenStack(openstack::OpenStack::new(
                base_url,
                credentials,
                settings,
            )?))
        }
        "openprovider" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige Openprovider-gegevens")?;
            let settings =
                serde_json::from_value(settings).context("ongeldige Openprovider-instellingen")?;
            Ok(Driver::Openprovider(openprovider::Openprovider::new(
                base_url,
                credentials,
                settings,
            )?))
        }
        "virtfusion" => {
            let credentials =
                serde_json::from_slice(credentials).context("ongeldige VirtFusion-sleutel")?;
            let settings =
                serde_json::from_value(settings).context("ongeldige VirtFusion-instellingen")?;
            Ok(Driver::VirtFusion(virtfusion::VirtFusion::new(
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
