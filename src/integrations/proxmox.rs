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
    #[serde(default = "default_storage")]
    pub iso_storage: String,
    pub gateway: Ipv4Addr,
    #[serde(default = "default_prefix")]
    pub prefix: u8,
    pub ip_from: Ipv4Addr,
    pub ip_to: Ipv4Addr,
    #[serde(default = "default_dns")]
    pub dns: String,
    #[serde(default = "default_verify_tls")]
    pub verify_tls: bool,
    #[serde(default)]
    pub images: Vec<Image>,
}

fn default_verify_tls() -> bool {
    true
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

const DISK_PREFIXES: [&str; 5] = ["scsi", "virtio", "sata", "ide", "unused"];

pub fn disk_key(config: &Value, volid: &str) -> Option<String> {
    config.as_object()?.iter().find_map(|(key, value)| {
        let is_disk = DISK_PREFIXES.iter().any(|prefix| {
            key.strip_prefix(prefix)
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
        });
        (is_disk && value.as_str()?.split(',').next()? == volid).then(|| key.clone())
    })
}

pub fn free_disk_key(config: &Value) -> anyhow::Result<String> {
    (1..=30)
        .map(|index| format!("scsi{index}"))
        .find(|key| config.get(key).is_none())
        .context("deze server heeft geen vrije schijfplaats meer")
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
        self.wait_for_task_up_to(upid, 600).await
    }

    async fn wait_for_task_up_to(&self, upid: &str, seconds: u32) -> anyhow::Result<()> {
        let path = format!(
            "/nodes/{}/tasks/{}/status",
            self.settings.node,
            upid.replace('!', "%21")
        );
        for _ in 0..seconds {
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

    pub async fn bind_address(&self, vmid: u32, ipv4: Ipv4Addr) -> anyhow::Result<()> {
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
            cpu: current["cpu"].as_f64(),
            cpus: current["cpus"].as_u64().unwrap_or(0),
            memory: current["mem"].as_u64(),
            max_memory: current["maxmem"].as_u64().unwrap_or(0),
            uptime: current["uptime"].as_u64(),
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

    async fn config(&self, vmid: u32) -> anyhow::Result<Value> {
        self.request(Method::GET, &format!("{}/config", self.vm_path(vmid)), None)
            .await
    }

    pub async fn create_volume(&self, vmid: u32, size_gb: u32) -> anyhow::Result<String> {
        let key = free_disk_key(&self.config(vmid).await?)?;
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &[(&key, format!("{}:{size_gb}", self.settings.storage))],
        )
        .await?;
        let config = self.config(vmid).await?;
        config[&key]
            .as_str()
            .and_then(|value| value.split(',').next())
            .map(str::to_owned)
            .context("Proxmox maakte geen schijf aan")
    }

    async fn unused_key(&self, vmid: u32, volid: &str) -> anyhow::Result<String> {
        let key = disk_key(&self.config(vmid).await?, volid)
            .context("de schijf staat niet meer bij deze server")?;
        if key.starts_with("unused") {
            return Ok(key);
        }
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &[("delete", key)],
        )
        .await?;
        disk_key(&self.config(vmid).await?, volid)
            .context("de losgekoppelde schijf is niet gevonden")
    }

    pub async fn detach_volume(&self, vmid: u32, volid: &str) -> anyhow::Result<()> {
        self.unused_key(vmid, volid).await.map(|_| ())
    }

    pub async fn attach_volume(&self, from: u32, volid: &str, to: u32) -> anyhow::Result<String> {
        let unused = self.unused_key(from, volid).await?;
        let target = free_disk_key(&self.config(to).await?)?;
        if from == to {
            self.run(
                Method::POST,
                &format!("{}/config", self.vm_path(to)),
                &[(&target, volid.to_owned())],
            )
            .await?;
        } else {
            self.run(
                Method::POST,
                &format!("{}/move_disk", self.vm_path(from)),
                &[
                    ("disk", unused),
                    ("target-vmid", to.to_string()),
                    ("target-disk", target.clone()),
                ],
            )
            .await?;
        }
        let config = self.config(to).await?;
        config[&target]
            .as_str()
            .and_then(|value| value.split(',').next())
            .map(str::to_owned)
            .context("de schijf staat niet bij de nieuwe server")
    }

    pub async fn delete_volume(&self, vmid: u32, volid: &str) -> anyhow::Result<()> {
        if disk_key(&self.config(vmid).await?, volid).is_none() {
            return Ok(());
        }
        let unused = self.unused_key(vmid, volid).await?;
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &[("delete", unused)],
        )
        .await
    }

    pub async fn stacklane_isos(&self) -> anyhow::Result<Vec<String>> {
        let content = self
            .request(
                Method::GET,
                &format!(
                    "/nodes/{}/storage/{}/content?content=iso",
                    self.settings.node, self.settings.iso_storage
                ),
                None,
            )
            .await?;
        let prefix = format!("{}:iso/", self.settings.iso_storage);
        Ok(content
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item["volid"].as_str())
            .filter_map(|volid| volid.strip_prefix(&prefix))
            .filter(|file| file.starts_with("stacklane-") && file.ends_with(".iso"))
            .map(str::to_owned)
            .collect())
    }

    fn iso_volume(&self, file: &str) -> String {
        format!("{}:iso/{file}", self.settings.iso_storage)
    }

    pub async fn import_iso(&self, file: &str, url: &str) -> anyhow::Result<()> {
        let upid = self
            .request(
                Method::POST,
                &format!(
                    "/nodes/{}/storage/{}/download-url",
                    self.settings.node, self.settings.iso_storage
                ),
                Some(&[
                    ("content", "iso".to_owned()),
                    ("filename", file.to_owned()),
                    ("url", url.to_owned()),
                    ("verify-certificates", "1".to_owned()),
                ]),
            )
            .await?;
        let Some(upid) = upid.as_str() else {
            return Ok(());
        };
        if let Err(error) = self.wait_for_task_up_to(upid, 3600).await {
            self.stop_task(upid).await.ok();
            self.delete_iso(file).await.ok();
            return Err(error);
        }
        Ok(())
    }

    async fn stop_task(&self, upid: &str) -> anyhow::Result<()> {
        self.request(
            Method::DELETE,
            &format!(
                "/nodes/{}/tasks/{}",
                self.settings.node,
                upid.replace('!', "%21")
            ),
            None,
        )
        .await
        .map(|_| ())
    }

    pub async fn delete_iso(&self, file: &str) -> anyhow::Result<()> {
        let volume = self.iso_volume(file);
        self.request(
            Method::DELETE,
            &format!(
                "/nodes/{}/storage/{}/content/{}",
                self.settings.node,
                self.settings.iso_storage,
                volume.replace('/', "%2F")
            ),
            None,
        )
        .await
        .map(|_| ())
    }

    pub async fn mount_iso(&self, vmid: u32, file: &str, uefi: bool) -> anyhow::Result<()> {
        let mut options = vec![
            ("ide2", format!("{},media=cdrom", self.iso_volume(file))),
            ("boot", "order=ide2;scsi0".to_owned()),
        ];
        let config = self.config(vmid).await?;
        if uefi {
            options.push(("bios", "ovmf".to_owned()));
            if config.get("efidisk0").is_none() {
                options.push((
                    "efidisk0",
                    format!("{}:1,efitype=4m,pre-enrolled-keys=0", self.settings.storage),
                ));
            }
        } else if config.get("bios").is_some() {
            options.push(("delete", "bios".to_owned()));
        }
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &options,
        )
        .await?;
        self.restart_if_running(vmid).await
    }

    pub async fn unmount_iso(&self, vmid: u32) -> anyhow::Result<()> {
        self.run(
            Method::POST,
            &format!("{}/config", self.vm_path(vmid)),
            &[
                ("ide2", "none,media=cdrom".to_owned()),
                ("boot", "order=scsi0".to_owned()),
            ],
        )
        .await?;
        self.restart_if_running(vmid).await
    }

    async fn restart_if_running(&self, vmid: u32) -> anyhow::Result<()> {
        let status = self
            .request(
                Method::GET,
                &format!("{}/status/current", self.vm_path(vmid)),
                None,
            )
            .await?;
        if status["status"] == "running" {
            self.power(vmid, "reboot").await?;
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

    fn stuck_download(method: &str, path: &str, _: &str) -> (u16, serde_json::Value) {
        match (method, path) {
            ("POST", path) if path.ends_with("/download-url") => (
                200,
                json!({ "data": "UPID:pve:1:2:3:download:iso:root@pam:" }),
            ),
            ("GET", path) if path.contains("/tasks/") => (503, json!({ "message": "busy" })),
            _ => (200, json!({ "data": null })),
        }
    }

    #[tokio::test]
    async fn an_abandoned_iso_download_is_stopped_and_removed() {
        let (base, calls) = crate::testing::fake_api(stuck_download).await;
        let credentials = Credentials {
            token_id: "t".into(),
            token_secret: "s".into(),
        };
        let client = Proxmox::new(&base, credentials, settings()).unwrap();
        assert!(
            client
                .import_iso("stacklane-x.iso", "https://example.test/x.iso")
                .await
                .is_err()
        );
        let calls = calls.lock().unwrap().clone();
        assert!(
            calls
                .iter()
                .any(|call| call.starts_with("DELETE /api2/json/nodes/pve/tasks/UPID"))
        );
        assert!(calls.iter().any(
            |call| call.starts_with("DELETE /api2/json/nodes/pve/storage/")
                && call.contains("stacklane-x.iso")
        ));
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
    fn finds_disks_by_volume() {
        let config = serde_json::json!({
            "scsi0": "local-lvm:vm-101-disk-0,size=20G",
            "scsi1": "local-lvm:vm-101-disk-1,size=10G",
            "unused0": "local-lvm:vm-101-disk-2",
            "scsihw": "virtio-scsi-pci",
        });
        assert_eq!(
            super::disk_key(&config, "local-lvm:vm-101-disk-1").as_deref(),
            Some("scsi1")
        );
        assert_eq!(
            super::disk_key(&config, "local-lvm:vm-101-disk-2").as_deref(),
            Some("unused0")
        );
        assert_eq!(super::disk_key(&config, "local-lvm:vm-101-disk-9"), None);
        assert_eq!(super::free_disk_key(&config).unwrap(), "scsi2");
    }

    #[test]
    fn fills_in_defaults() {
        let settings = settings();
        assert_eq!(settings.bridge, "vmbr0");
        assert_eq!(settings.storage, "local");
        assert_eq!(settings.prefix, 24);
        assert!(settings.verify_tls);
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
