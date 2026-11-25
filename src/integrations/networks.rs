use anyhow::Context;

use super::Driver;
use super::openstack::OpenStack;
use super::proxmox::Proxmox;

pub struct NetworkRef {
    pub vlan: Option<u16>,
    pub external_id: Option<String>,
}

pub struct Attached {
    pub port: String,
    pub address: String,
}

pub enum Networks {
    Proxmox(Proxmox),
    OpenStack(OpenStack),
}

fn vmid(value: &str) -> anyhow::Result<u32> {
    value.parse().context("ongeldig VM-nummer")
}

impl Networks {
    pub fn from_driver(driver: Driver) -> Option<Self> {
        match driver {
            Driver::Proxmox(client) if client.private_bridge().is_some() => {
                Some(Self::Proxmox(client))
            }
            Driver::OpenStack(client) => Some(Self::OpenStack(client)),
            _ => None,
        }
    }

    pub fn vlans(&self) -> Option<std::ops::RangeInclusive<u16>> {
        match self {
            Self::Proxmox(client) => Some(client.settings().vlan_from..=client.settings().vlan_to),
            Self::OpenStack(_) => None,
        }
    }

    pub async fn create(&self, name: &str, cidr: &str) -> anyhow::Result<Option<String>> {
        match self {
            Self::Proxmox(_) => Ok(None),
            Self::OpenStack(client) => client.create_network(name, cidr).await.map(Some),
        }
    }

    pub async fn delete(&self, network: &NetworkRef) -> anyhow::Result<()> {
        match (self, network.external_id.as_deref()) {
            (Self::OpenStack(client), Some(id)) => client.delete_network(id).await,
            _ => Ok(()),
        }
    }

    pub async fn attach(
        &self,
        network: &NetworkRef,
        server: &str,
        address: Option<&str>,
    ) -> anyhow::Result<Attached> {
        match self {
            Self::Proxmox(client) => {
                let vlan = network.vlan.context("dit netwerk heeft geen VLAN")?;
                let address = address.context("er is geen adres gekozen")?;
                let port = client.attach_network(vmid(server)?, vlan, address).await?;
                Ok(Attached {
                    port,
                    address: address.to_owned(),
                })
            }
            Self::OpenStack(client) => {
                let id = network
                    .external_id
                    .as_deref()
                    .context("dit netwerk bestaat niet in OpenStack")?;
                let (port, address) = client.attach_network(server, id).await?;
                Ok(Attached { port, address })
            }
        }
    }

    pub async fn detach(&self, server: &str, port: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.detach_network(vmid(server)?, port).await,
            Self::OpenStack(client) => client.detach_network(server, port).await,
        }
    }
}
