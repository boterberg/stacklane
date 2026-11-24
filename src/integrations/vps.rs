use serde::{Deserialize, Serialize};

use super::openstack::OpenStack;
use super::proxmox::Proxmox;

#[derive(Serialize)]
pub struct Status {
    pub state: String,
    pub cpu: Option<f64>,
    pub cpus: u64,
    pub memory: Option<u64>,
    pub max_memory: u64,
    pub uptime: Option<u64>,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub name: String,
    pub description: String,
    pub created_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FirewallRule {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type")]
    pub direction: String,
    pub action: String,
    #[serde(default)]
    pub proto: Option<String>,
    #[serde(default)]
    pub dport: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
}

pub enum Machine {
    Proxmox(Proxmox, u32),
    OpenStack(OpenStack, String),
}

impl Machine {
    pub fn can_block(&self) -> bool {
        matches!(self, Self::Proxmox(..))
    }

    pub async fn status(&self) -> anyhow::Result<Status> {
        match self {
            Self::Proxmox(client, vmid) => client.status(*vmid).await,
            Self::OpenStack(client, id) => client.status(id).await,
        }
    }

    pub async fn power(&self, action: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.power(*vmid, action).await,
            Self::OpenStack(client, id) => client.power(id, action).await,
        }
    }

    pub async fn snapshots(&self) -> anyhow::Result<Vec<Snapshot>> {
        match self {
            Self::Proxmox(client, vmid) => client.snapshots(*vmid).await,
            Self::OpenStack(client, id) => client.snapshots(id).await,
        }
    }

    pub async fn create_snapshot(&self, name: &str, description: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.create_snapshot(*vmid, name, description).await,
            Self::OpenStack(client, id) => client.create_snapshot(id, name, description).await,
        }
    }

    pub async fn rollback_snapshot(&self, name: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.rollback_snapshot(*vmid, name).await,
            Self::OpenStack(client, id) => client.rollback_snapshot(id, name).await,
        }
    }

    pub async fn delete_snapshot(&self, name: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.delete_snapshot(*vmid, name).await,
            Self::OpenStack(client, id) => client.delete_snapshot(id, name).await,
        }
    }

    pub async fn firewall_rules(&self) -> anyhow::Result<Vec<FirewallRule>> {
        match self {
            Self::Proxmox(client, vmid) => client.firewall_rules(*vmid).await,
            Self::OpenStack(client, id) => client.firewall_rules(id).await,
        }
    }

    pub async fn add_firewall_rule(&self, rule: &FirewallRule) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.add_firewall_rule(*vmid, rule).await,
            Self::OpenStack(client, id) => client.add_firewall_rule(id, rule).await,
        }
    }

    pub async fn delete_firewall_rule(&self, rule: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.delete_firewall_rule(*vmid, rule.parse()?).await,
            Self::OpenStack(client, id) => client.delete_firewall_rule(id, rule).await,
        }
    }
}
