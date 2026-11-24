use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::Driver;
use super::openstack::OpenStack;
use super::proxmox::Proxmox;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeRef {
    #[serde(default)]
    pub host: String,
    pub id: String,
}

pub enum Storage {
    Proxmox(Proxmox),
    OpenStack(OpenStack),
}

fn vmid(value: &str) -> anyhow::Result<u32> {
    value.parse().context("ongeldig VM-nummer")
}

impl Storage {
    pub fn from_driver(driver: Driver) -> Option<Self> {
        match driver {
            Driver::Proxmox(client) => Some(Self::Proxmox(client)),
            Driver::OpenStack(client) => Some(Self::OpenStack(client)),
            _ => None,
        }
    }

    pub async fn create(
        &self,
        server: &str,
        name: &str,
        size_gb: u32,
    ) -> anyhow::Result<VolumeRef> {
        match self {
            Self::Proxmox(client) => Ok(VolumeRef {
                host: server.to_owned(),
                id: client.create_volume(vmid(server)?, size_gb).await?,
            }),
            Self::OpenStack(client) => Ok(VolumeRef {
                host: String::new(),
                id: client.create_volume(server, name, size_gb).await?,
            }),
        }
    }

    pub async fn attach(&self, volume: &VolumeRef, server: &str) -> anyhow::Result<VolumeRef> {
        match self {
            Self::Proxmox(client) => Ok(VolumeRef {
                host: server.to_owned(),
                id: client
                    .attach_volume(vmid(&volume.host)?, &volume.id, vmid(server)?)
                    .await?,
            }),
            Self::OpenStack(client) => {
                client.attach_volume(server, &volume.id).await?;
                Ok(volume.clone())
            }
        }
    }

    pub async fn detach(&self, volume: &VolumeRef, server: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.detach_volume(vmid(&volume.host)?, &volume.id).await,
            Self::OpenStack(client) => client.detach_volume(server, &volume.id).await,
        }
    }

    pub async fn delete(&self, volume: &VolumeRef, server: Option<&str>) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.delete_volume(vmid(&volume.host)?, &volume.id).await,
            Self::OpenStack(client) => client.delete_volume(server, &volume.id).await,
        }
    }
}
