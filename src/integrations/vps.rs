use serde::{Deserialize, Serialize};

use super::openstack::OpenStack;
use super::proxmox::Proxmox;
use super::virtfusion::VirtFusion;

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
pub struct Metric {
    pub time: i64,
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub max_memory: Option<f64>,
    pub net_in: Option<f64>,
    pub net_out: Option<f64>,
    pub disk_read: Option<f64>,
    pub disk_write: Option<f64>,
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

#[derive(Serialize)]
pub struct Features {
    pub snapshots: bool,
    pub firewall: bool,
    pub volumes: bool,
    pub floating_ips: bool,
    pub networks: bool,
    pub rebuild: bool,
    pub graphs: bool,
    pub rescue: bool,
    pub panel: bool,
}

pub const DEMO_ONLY: &str = "In de demo voer je dit niet echt uit.";

fn demo<T>() -> anyhow::Result<T> {
    anyhow::bail!(DEMO_ONLY)
}

fn demo_metrics(seed: u64, timeframe: &str) -> Vec<Metric> {
    let (points, step) = match timeframe {
        "day" => (72, 1200),
        "week" => (84, 7200),
        "month" => (90, 28800),
        _ => (60, 60),
    };
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let base = (seed % 7) as f64;
    (0..points)
        .map(|index| {
            let wave = ((index as f64 + base) / 6.0).sin();
            let busy = 0.18 + 0.1 * wave + 0.04 * ((index as f64 * 1.7 + base).cos());
            Metric {
                time: now - (points - 1 - index) * step,
                cpu: Some(busy.clamp(0.02, 0.95)),
                memory: Some((0.55 + 0.08 * wave) * 2_147_483_648.0),
                max_memory: Some(2_147_483_648.0),
                net_in: Some(180_000.0 + 90_000.0 * wave.abs()),
                net_out: Some(420_000.0 + 210_000.0 * wave.abs()),
                disk_read: Some(35_000.0 + 20_000.0 * (wave + 1.0)),
                disk_write: Some(60_000.0 + 25_000.0 * (wave + 1.0)),
            }
        })
        .collect()
}

fn demo_rule(id: &str, port: &str, source: Option<&str>, comment: &str) -> FirewallRule {
    FirewallRule {
        id: id.to_owned(),
        direction: "in".to_owned(),
        action: "ACCEPT".to_owned(),
        proto: Some("tcp".to_owned()),
        dport: Some(port.to_owned()),
        source: source.map(str::to_owned),
        comment: Some(comment.to_owned()),
    }
}

fn unsupported<T>() -> anyhow::Result<T> {
    anyhow::bail!("dit kan alleen in het panel van VirtFusion")
}

pub enum Machine {
    Proxmox(Proxmox, u32),
    OpenStack(OpenStack, String),
    VirtFusion(VirtFusion, String),
    Demo(u64),
}

impl Machine {
    pub fn features(&self) -> Features {
        if matches!(self, Self::Demo(..)) {
            return Features {
                snapshots: true,
                firewall: true,
                volumes: true,
                floating_ips: true,
                networks: true,
                rebuild: true,
                graphs: true,
                rescue: true,
                panel: false,
            };
        }
        let virtfusion = matches!(self, Self::VirtFusion(..));
        Features {
            snapshots: !virtfusion,
            firewall: !virtfusion,
            volumes: !virtfusion,
            floating_ips: match self {
                Self::Proxmox(..) => true,
                Self::OpenStack(client, _) => client.has_floating_ips(),
                Self::VirtFusion(..) | Self::Demo(..) => false,
            },
            networks: match self {
                Self::Proxmox(client, _) => client.private_bridge().is_some(),
                Self::OpenStack(..) => true,
                Self::VirtFusion(..) | Self::Demo(..) => false,
            },
            rebuild: !virtfusion,
            graphs: matches!(self, Self::Proxmox(..)),
            rescue: matches!(self, Self::OpenStack(..)),
            panel: virtfusion,
        }
    }

    pub fn can_block(&self) -> bool {
        matches!(self, Self::Proxmox(..) | Self::Demo(..))
    }

    pub async fn status(&self) -> anyhow::Result<Status> {
        match self {
            Self::Proxmox(client, vmid) => client.status(*vmid).await,
            Self::OpenStack(client, id) => client.status(id).await,
            Self::VirtFusion(client, id) => client.status(id).await,
            Self::Demo(seed) => Ok(Status {
                state: "running".to_owned(),
                cpu: Some(0.12 + (*seed % 5) as f64 / 100.0),
                cpus: 2,
                memory: Some(1_288_490_189),
                max_memory: 2_147_483_648,
                uptime: Some(1_296_000 + *seed % 86_400),
            }),
        }
    }

    pub async fn metrics(&self, timeframe: &str) -> anyhow::Result<Vec<Metric>> {
        match self {
            Self::Proxmox(client, vmid) => client.metrics(*vmid, timeframe).await,
            Self::Demo(seed) => Ok(demo_metrics(*seed, timeframe)),
            _ => anyhow::bail!("dit platform geeft geen grafieken door"),
        }
    }

    pub async fn rescue(&self, password: &str) -> anyhow::Result<()> {
        match self {
            Self::OpenStack(client, id) => client.rescue(id, password).await,
            Self::Demo(..) => demo(),
            _ => anyhow::bail!("dit platform heeft geen rescue-modus"),
        }
    }

    pub async fn unrescue(&self) -> anyhow::Result<()> {
        match self {
            Self::OpenStack(client, id) => client.unrescue(id).await,
            Self::Demo(..) => demo(),
            _ => anyhow::bail!("dit platform heeft geen rescue-modus"),
        }
    }

    pub async fn power(&self, action: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.power(*vmid, action).await,
            Self::OpenStack(client, id) => client.power(id, action).await,
            Self::VirtFusion(client, id) => client.power(id, action).await,
            Self::Demo(..) => demo(),
        }
    }

    pub async fn snapshots(&self) -> anyhow::Result<Vec<Snapshot>> {
        match self {
            Self::Proxmox(client, vmid) => client.snapshots(*vmid).await,
            Self::OpenStack(client, id) => client.snapshots(id).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => {
                let now = time::OffsetDateTime::now_utc().unix_timestamp();
                Ok(vec![
                    Snapshot {
                        name: "voor_update".to_owned(),
                        description: "Voor de upgrade naar PHP 8.3".to_owned(),
                        created_at: Some(now - 3 * 86_400),
                    },
                    Snapshot {
                        name: "schone_installatie".to_owned(),
                        description: "Net na de installatie".to_owned(),
                        created_at: Some(now - 40 * 86_400),
                    },
                ])
            }
        }
    }

    pub async fn create_snapshot(&self, name: &str, description: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.create_snapshot(*vmid, name, description).await,
            Self::OpenStack(client, id) => client.create_snapshot(id, name, description).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => demo(),
        }
    }

    pub async fn rollback_snapshot(&self, name: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.rollback_snapshot(*vmid, name).await,
            Self::OpenStack(client, id) => client.rollback_snapshot(id, name).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => demo(),
        }
    }

    pub async fn delete_snapshot(&self, name: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.delete_snapshot(*vmid, name).await,
            Self::OpenStack(client, id) => client.delete_snapshot(id, name).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => demo(),
        }
    }

    pub async fn firewall_rules(&self) -> anyhow::Result<Vec<FirewallRule>> {
        match self {
            Self::Proxmox(client, vmid) => client.firewall_rules(*vmid).await,
            Self::OpenStack(client, id) => client.firewall_rules(id).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => Ok(vec![
                demo_rule(
                    "1",
                    "22",
                    Some("198.51.100.0/24"),
                    "[fw:demo] SSH vanaf kantoor",
                ),
                demo_rule("2", "80", None, "[fw:demo] Website"),
                demo_rule("3", "443", None, "[fw:demo] Website"),
            ]),
        }
    }

    pub async fn add_firewall_rule(&self, rule: &FirewallRule) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.add_firewall_rule(*vmid, rule).await,
            Self::OpenStack(client, id) => client.add_firewall_rule(id, rule).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => demo(),
        }
    }

    pub async fn delete_firewall_rule(&self, rule: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client, vmid) => client.delete_firewall_rule(*vmid, rule.parse()?).await,
            Self::OpenStack(client, id) => client.delete_firewall_rule(id, rule).await,
            Self::VirtFusion(..) => unsupported(),
            Self::Demo(..) => demo(),
        }
    }
}
