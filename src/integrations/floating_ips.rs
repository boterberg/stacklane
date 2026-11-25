use std::net::Ipv4Addr;

use anyhow::Context;

use super::Driver;
use super::openstack::OpenStack;
use super::proxmox::Proxmox;

pub struct FloatingIp {
    pub id: String,
    pub address: String,
}

pub enum Addresses {
    Proxmox(Proxmox),
    OpenStack(OpenStack),
}

fn vmid(value: &str) -> anyhow::Result<u32> {
    value.parse().context("ongeldig VM-nummer")
}

fn address(ip: &FloatingIp) -> anyhow::Result<Ipv4Addr> {
    ip.address.parse().context("ongeldig IP-adres")
}

impl Addresses {
    pub fn from_driver(driver: Driver) -> Option<Self> {
        match driver {
            Driver::Proxmox(client) => Some(Self::Proxmox(client)),
            Driver::OpenStack(client) if client.has_floating_ips() => Some(Self::OpenStack(client)),
            _ => None,
        }
    }

    pub async fn attach(&self, ip: &FloatingIp, server: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.bind_address(vmid(server)?, address(ip)?).await,
            Self::OpenStack(client) => client.attach_floating_ip(&ip.id, server).await,
        }
    }

    pub async fn detach(&self, ip: &FloatingIp, server: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.unbind_address(vmid(server)?, address(ip)?).await,
            Self::OpenStack(client) => client.detach_floating_ip(&ip.id).await,
        }
    }

    pub async fn release(&self, ip: &FloatingIp, server: Option<&str>) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => match server {
                Some(server) => client.unbind_address(vmid(server)?, address(ip)?).await,
                None => Ok(()),
            },
            Self::OpenStack(client) => client.delete_floating_ip(&ip.id).await,
        }
    }
}
