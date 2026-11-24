use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::{Context, bail};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::vps::{FirewallRule, Snapshot, Status};

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub token_id: String,
    pub token_secret: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Image {
    pub template: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub node: String,
    #[serde(default = "default_bridge")]
    pub bridge: String,
    #[serde(default = "default_storage")]
    pub storage: String,
    pub gateway: Ipv4Addr,
    #[serde(default = "default_prefix")]
    pub prefix: u8,
    pub ip_from: Ipv4Addr,
    pub ip_to: Ipv4Addr,
    #[serde(default = "default_dns")]
    pub dns: String,
    #[serde(default)]
    pub verify_tls: bool,
    #[serde(default)]
    pub images: Vec<Image>,
}

fn default_bridge() -> String {
    "vmbr0".to_owned()
}

fn default_storage() -> String {
    "local".to_owned()
}

fn default_prefix() -> u8 {
    24
}

fn default_dns() -> String {
    "1.1.1.1".to_owned()
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProductConfig {
    pub cores: u32,
    pub memory_mb: u32,
    pub disk_gb: u32,
}

pub struct NewServer<'a> {
    pub vmid: u32,
    pub hostname: &'a str,
    pub template: u32,
    pub ipv4: Ipv4Addr,
    pub ssh_keys: &'a [String],
    pub password: &'a str,
    pub description: &'a str,
}

pub struct Proxmox {
    base_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
}

fn encode_ssh_keys(keys: &[String]) -> String {
    keys.join("\n")
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

pub fn ip_range(settings: &Settings) -> impl Iterator<Item = Ipv4Addr> {
    (u32::from(settings.ip_from)..=u32::from(settings.ip_to)).map(Ipv4Addr::from)
}

impl Proxmox {
    pub fn new(
        base_url: &str,
        credentials: Credentials,
        settings: Settings,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .danger_accept_invalid_certs(!settings.verify_tls)
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            credentials,
            settings,
            http,
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    fn vm_path(&self, vmid: u32) -> String {
        format!("/nodes/{}/qemu/{vmid}", self.settings.node)
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        form: Option<&[(&str, String)]>,
    ) -> anyhow::Result<Value> {
        let mut request = self
            .http
            .request(method, format!("{}/api2/json{path}", self.base_url))
            .header(
                "Authorization",
                format!(
                    "PVEAPIToken={}={}",
                    self.credentials.token_id, self.credentials.token_secret
                ),
            );
        if let Some(form) = form {
            request = request.form(form);
        }
        let response = request.send().await.context("Proxmox is niet bereikbaar")?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            let detail = body["errors"]
                .as_object()
                .and_then(|errors| errors.values().next())
                .and_then(Value::as_str)
                .or_else(|| body["message"].as_str())
                .unwrap_or("onbekende fout");
            bail!("Proxmox antwoordde {status}: {}", detail.trim());
        }
        Ok(body["data"].clone())
    }

    async fn wait_for_task(&self, upid: &str) -> anyhow::Result<()> {
        let path = format!(
            "/nodes/{}/tasks/{}/status",
            self.settings.node,
            upid.replace('!', "%21")
        );
        for _ in 0..600 {
            let task = self.request(Method::GET, &path, None).await?;
            if task["status"] == "stopped" {
                return match task["exitstatus"].as_str() {
                    Some("OK") => Ok(()),
                    other => bail!("Proxmox-taak mislukt: {}", other.unwrap_or("onbekend")),
                };
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        bail!("Proxmox-taak duurde te lang")
    }

    async fn run(&self, method: Method, path: &str, form: &[(&str, String)]) -> anyhow::Result<()> {
        let upid = self.request(method, path, Some(form)).await?;
        if let Some(upid) = upid.as_str() {
            self.wait_for_task(upid).await?;
        }
        Ok(())
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        let version = self.request(Method::GET, "/version", None).await?;
        self.request(
            Method::GET,
            &format!("/nodes/{}/status", self.settings.node),
            None,
        )
        .await
        .with_context(|| format!("node {} niet gevonden", self.settings.node))?;
        let firewall = self
            .request(Method::GET, "/cluster/firewall/options", None)
            .await
            .unwrap_or_default();
        let warning = if firewall["enable"].as_i64() == Some(1) {
            ""
        } else {
            ", let op: de firewall van het datacenter staat uit, zet die aan zodat servers elkaars IP-adres niet kunnen overnemen"
        };
        Ok(format!(
            "Proxmox VE {}{warning}",
            version["version"].as_str().unwrap_or("?")
        ))
    }

    pub async fn next_vmid(&self) -> anyhow::Result<u32> {
        let id = self.request(Method::GET, "/cluster/nextid", None).await?;
        id.as_str()
            .and_then(|id| id.parse().ok())
            .or_else(|| id.as_u64().and_then(|id| u32::try_from(id).ok()))
            .context("Proxmox gaf geen vrij VM-nummer")
    }

    pub async fn free_vmid(&self, taken: &[u32]) -> anyhow::Result<u32> {
        let mut vmid = self.next_vmid().await?;
        for _ in 0..1000 {
            if !taken.contains(&vmid)
                && self
                    .request(Method::GET, &format!("/cluster/nextid?vmid={vmid}"), None)
                    .await
                    .is_ok()
            {
                return Ok(vmid);
            }
            vmid += 1;
        }
        bail!("Proxmox gaf geen vrij VM-nummer")
    }

    pub async fn description(&self, vmid: u32) -> Option<String> {
        let config = self
            .request(Method::GET, &format!("{}/config", self.vm_path(vmid)), None)
            .await
            .ok()?;
        config["description"].as_str().map(str::to_owned)
    }

    pub async fn create(
        &self,
        server: &NewServer<'_>,
        config: &ProductConfig,
    ) -> anyhow::Result<()> {
        self.run(
            Method::POST,
            &format!("{}/clone", self.vm_path(server.template)),
            &[
                ("newid", server.vmid.to_string()),
                ("name", server.hostname.to_owned()),
                ("full", "1".to_owned()),
                ("storage", self.settings.storage.clone()),
                ("description", server.description.to_owned()),
            ],
        )
        .await?;
        let mut options = vec![
            ("cores", config.cores.to_string()),
            ("memory", config.memory_mb.to_string()),
            ("ciuser", "root".to_owned()),
            ("cipassword", server.password.to_owned()),
            (
                "ipconfig0",
                format!(
                    "ip={}/{},gw={}",
                    server.ipv4, self.settings.prefix, self.settings.gateway
                ),
            ),
            ("nameserver", self.settings.dns.clone()),
            (
                "net0",
                format!("virtio,bridge={},firewall=1", self.settings.bridge),
            ),
            ("onboot", "1".to_owned()),
        ];
        if !server.ssh_keys.is_empty() {
            options.push(("sshkeys", encode_ssh_keys(server.ssh_keys)));
        }
        self.request(
            Method::POST,
            &format!("{}/config", self.vm_path(server.vmid)),
            Some(&options),
        )
        .await?;
        self.request(
            Method::PUT,
            &format!("{}/resize", self.vm_path(server.vmid)),
            Some(&[
                ("disk", "scsi0".to_owned()),
                ("size", format!("{}G", config.disk_gb)),
            ]),
        )
        .await?;
        self.bind_address(server.vmid, server.ipv4).await?;
        self.power(server.vmid, "start").await
    }

    async fn bind_address(&self, vmid: u32, ipv4: Ipv4Addr) -> anyhow::Result<()> {
        let firewall = format!("{}/firewall", self.vm_path(vmid));
        self.request(
            Method::POST,
            &format!("{firewall}/ipset"),
            Some(&[("name", "ipfilter-net0".to_owned())]),
        )
        .await
        .ok();
        self.request(
            Method::POST,
            &format!("{firewall}/ipset/ipfilter-net0"),
            Some(&[("cidr", ipv4.to_string())]),
        )
        .await
        .ok();
        let listed = self
            .request(
                Method::GET,
                &format!("{firewall}/ipset/ipfilter-net0"),
                None,
            )
            .await?;
        anyhow::ensure!(
            listed
                .as_array()
                .into_iter()
                .flatten()
                .any(|entry| entry["cidr"]
                    .as_str()
                    .is_some_and(|cidr| cidr.trim_end_matches("/32") == ipv4.to_string())),
            "het IP-adres staat niet in het IP-filter van de server"
        );
        self.request(
            Method::PUT,
            &format!("{firewall}/options"),
            Some(&[
                ("enable", "1".to_owned()),
                ("ipfilter", "1".to_owned()),
                ("macfilter", "1".to_owned()),
                ("policy_in", "ACCEPT".to_owned()),
            ]),
        )
        .await?;
        Ok(())
    }

    pub async fn status(&self, vmid: u32) -> anyhow::Result<Status> {
        let current = self
            .request(
                Method::GET,
                &format!("{}/status/current", self.vm_path(vmid)),
                None,
            )
            .await?;
        Ok(Status {
            state: current["status"].as_str().unwrap_or("unknown").to_owned(),
            cpu: current["cpu"].as_f64().unwrap_or(0.0),
            cpus: current["cpus"].as_u64().unwrap_or(0),
            memory: current["mem"].as_u64().unwrap_or(0),
            max_memory: current["maxmem"].as_u64().unwrap_or(0),
            uptime: current["uptime"].as_u64().unwrap_or(0),
        })
    }

    pub async fn suspend(&self, vmid: u32) -> anyhow::Result<()> {
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &[("onboot", "0".to_owned())],
        )
        .await?;
        if self.status(vmid).await?.state == "running" {
            self.power(vmid, "stop").await?;
        }
        Ok(())
    }

    pub async fn unsuspend(&self, vmid: u32) -> anyhow::Result<()> {
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &[("onboot", "1".to_owned())],
        )
        .await?;
        if self.status(vmid).await?.state != "running" {
            self.power(vmid, "start").await?;
        }
        Ok(())
    }

    pub async fn power(&self, vmid: u32, action: &str) -> anyhow::Result<()> {
        if !matches!(action, "start" | "stop" | "shutdown" | "reboot") {
            bail!("onbekende actie {action}");
        }
        self.run(
            Method::POST,
            &format!("{}/status/{action}", self.vm_path(vmid)),
            &[],
        )
        .await
    }

    pub async fn destroy(&self, vmid: u32) -> anyhow::Result<()> {
        if self
            .status(vmid)
            .await
            .map(|status| status.state == "running")
            .unwrap_or(false)
        {
            self.power(vmid, "stop").await?;
        }
        let upid = self
            .request(
                Method::DELETE,
                &format!(
                    "{}?purge=1&destroy-unreferenced-disks=1",
                    self.vm_path(vmid)
                ),
                None,
            )
            .await?;
        if let Some(upid) = upid.as_str() {
            self.wait_for_task(upid).await?;
        }
        Ok(())
    }

    pub async fn snapshots(&self, vmid: u32) -> anyhow::Result<Vec<Snapshot>> {
        let list = self
            .request(
                Method::GET,
                &format!("{}/snapshot", self.vm_path(vmid)),
                None,
            )
            .await?;
        let mut snapshots: Vec<Snapshot> = list
            .as_array()
            .into_iter()
            .flatten()
            .filter(|snapshot| snapshot["name"] != "current")
            .map(|snapshot| Snapshot {
                name: snapshot["name"].as_str().unwrap_or_default().to_owned(),
                description: snapshot["description"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
                created_at: snapshot["snaptime"].as_i64(),
            })
            .collect();
        snapshots.sort_by_key(|snapshot| std::cmp::Reverse(snapshot.created_at));
        Ok(snapshots)
    }

    pub async fn create_snapshot(
        &self,
        vmid: u32,
        name: &str,
        description: &str,
    ) -> anyhow::Result<()> {
        self.run(
            Method::POST,
            &format!("{}/snapshot", self.vm_path(vmid)),
            &[
                ("snapname", name.to_owned()),
                ("description", description.to_owned()),
            ],
        )
        .await
    }

    pub async fn rollback_snapshot(&self, vmid: u32, name: &str) -> anyhow::Result<()> {
        self.run(
            Method::POST,
            &format!("{}/snapshot/{name}/rollback", self.vm_path(vmid)),
            &[],
        )
        .await
    }

    pub async fn delete_snapshot(&self, vmid: u32, name: &str) -> anyhow::Result<()> {
        let upid = self
            .request(
                Method::DELETE,
                &format!("{}/snapshot/{name}", self.vm_path(vmid)),
                None,
            )
            .await?;
        if let Some(upid) = upid.as_str() {
            self.wait_for_task(upid).await?;
        }
        Ok(())
    }

    pub async fn firewall_rules(&self, vmid: u32) -> anyhow::Result<Vec<FirewallRule>> {
        let rules = self
            .request(
                Method::GET,
                &format!("{}/firewall/rules", self.vm_path(vmid)),
                None,
            )
            .await?;
        Ok(rules
            .as_array()
            .into_iter()
            .flatten()
            .map(|rule| {
                let text = |key: &str| rule[key].as_str().map(str::to_owned);
                FirewallRule {
                    id: rule["pos"].as_u64().unwrap_or_default().to_string(),
                    direction: text("type").unwrap_or_default(),
                    action: text("action").unwrap_or_default(),
                    proto: text("proto"),
                    dport: text("dport"),
                    source: text("source"),
                    comment: text("comment"),
                }
            })
            .collect())
    }

    pub async fn add_firewall_rule(&self, vmid: u32, rule: &FirewallRule) -> anyhow::Result<()> {
        let mut form = vec![
            ("type", rule.direction.clone()),
            ("action", rule.action.clone()),
            ("enable", "1".to_owned()),
        ];
        for (key, value) in [
            ("proto", &rule.proto),
            ("dport", &rule.dport),
            ("source", &rule.source),
            ("comment", &rule.comment),
        ] {
            if let Some(value) = value.as_ref().filter(|value| !value.is_empty()) {
                form.push((key, value.clone()));
            }
        }
        self.request(
            Method::POST,
            &format!("{}/firewall/rules", self.vm_path(vmid)),
            Some(&form),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_firewall_rule(&self, vmid: u32, pos: u32) -> anyhow::Result<()> {
        self.request(
            Method::DELETE,
            &format!("{}/firewall/rules/{pos}", self.vm_path(vmid)),
            None,
        )
        .await?;
        Ok(())
    }
}

pub fn product_details(config: &ProductConfig) -> Value {
    json!({ "cores": config.cores, "memory_mb": config.memory_mb, "disk_gb": config.disk_gb })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_node(method: &str, path: &str, _: &str) -> (u16, serde_json::Value) {
        match (method, path) {
            ("GET", path) if path.ends_with("/status/current") => {
                (200, json!({ "data": { "status": "running" } }))
            }
            _ => (200, json!({ "data": null })),
        }
    }

    async fn client() -> (Proxmox, crate::testing::Calls) {
        let (base, calls) = crate::testing::fake_api(fake_node).await;
        let credentials = Credentials {
            token_id: "t".into(),
            token_secret: "s".into(),
        };
        (Proxmox::new(&base, credentials, settings()).unwrap(), calls)
    }

    fn filtering_node(method: &str, path: &str, _: &str) -> (u16, serde_json::Value) {
        match (method, path) {
            ("GET", path) if path.ends_with("/ipset/ipfilter-net0") => {
                (200, json!({ "data": [{ "cidr": "192.0.2.10" }] }))
            }
            _ => (200, json!({ "data": null })),
        }
    }

    #[tokio::test]
    async fn binds_each_vm_to_its_own_address() {
        let (base, calls) = crate::testing::fake_api(filtering_node).await;
        let credentials = Credentials {
            token_id: "t".into(),
            token_secret: "s".into(),
        };
        let client = Proxmox::new(&base, credentials, settings()).unwrap();
        client
            .bind_address(101, "192.0.2.10".parse().unwrap())
            .await
            .unwrap();
        let calls = calls.lock().unwrap().clone();
        assert!(
            calls.contains(
                &"POST /api2/json/nodes/pve/qemu/101/firewall/ipset/ipfilter-net0 cidr=192.0.2.10"
                    .to_owned()
            )
        );
        assert!(calls.iter().any(|call| {
            call.starts_with("PUT /api2/json/nodes/pve/qemu/101/firewall/options")
                && call.contains("ipfilter=1")
        }));
        assert!(
            client
                .bind_address(102, "192.0.2.11".parse().unwrap())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn suspending_keeps_the_vm_off_after_a_reboot() {
        let (client, calls) = client().await;
        client.suspend(101).await.unwrap();
        let calls = calls.lock().unwrap().clone();
        assert_eq!(
            calls[0],
            "POST /api2/json/nodes/pve/qemu/101/config onboot=0"
        );
        assert!(calls.contains(&"POST /api2/json/nodes/pve/qemu/101/status/stop".to_owned()));
    }

    fn settings() -> Settings {
        serde_json::from_value(json!({
            "node": "pve",
            "gateway": "10.10.10.1",
            "ip_from": "10.10.10.100",
            "ip_to": "10.10.10.102",
        }))
        .unwrap()
    }

    #[test]
    fn fills_in_defaults() {
        let settings = settings();
        assert_eq!(settings.bridge, "vmbr0");
        assert_eq!(settings.storage, "local");
        assert_eq!(settings.prefix, 24);
        assert!(!settings.verify_tls);
    }

    #[test]
    fn walks_the_ip_range() {
        let ips: Vec<String> = ip_range(&settings()).map(|ip| ip.to_string()).collect();
        assert_eq!(ips, ["10.10.10.100", "10.10.10.101", "10.10.10.102"]);
    }

    #[test]
    fn encodes_ssh_keys_like_proxmox_expects() {
        let encoded = encode_ssh_keys(&[
            "ssh-ed25519 AAAA+/= jan@laptop".to_owned(),
            "ssh-rsa BBB".to_owned(),
        ]);
        assert_eq!(
            encoded,
            "ssh-ed25519%20AAAA%2B%2F%3D%20jan%40laptop%0Assh-rsa%20BBB"
        );
    }
}
