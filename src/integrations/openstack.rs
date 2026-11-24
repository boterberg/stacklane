use std::time::Duration;

use anyhow::{Context, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::OnceCell;

use super::vps::{FirewallRule, Snapshot, Status};

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub application_credential_id: String,
    pub application_credential_secret: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Image {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub region: Option<String>,
    pub network_id: String,
    #[serde(default = "default_interface")]
    pub interface: String,
    #[serde(default)]
    pub images: Vec<Image>,
}

fn default_interface() -> String {
    "public".to_owned()
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProductConfig {
    pub flavor_id: String,
    #[serde(default)]
    pub cores: u32,
    #[serde(default)]
    pub memory_mb: u32,
    #[serde(default)]
    pub disk_gb: u32,
}

pub struct NewServer<'a> {
    pub hostname: &'a str,
    pub image_id: &'a str,
    pub flavor_id: &'a str,
    pub ssh_keys: &'a [String],
    pub password: &'a str,
    pub description: &'a str,
}

#[derive(Clone, Copy)]
enum Service {
    Compute,
    Image,
    Network,
    Volume,
}

struct Session {
    token: String,
    compute: String,
    image: String,
    network: String,
    volume: Option<String>,
}

pub struct OpenStack {
    auth_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
    session: OnceCell<Session>,
}

const COMPUTE_VERSION: &str = "compute 2.47";
const VOLUME_VERSION: &str = "volume 3.42";

fn error_message(body: &Value) -> Option<String> {
    let nested = body
        .as_object()
        .and_then(|object| object.values().find_map(|value| value["message"].as_str()));
    nested
        .or_else(|| body["message"].as_str())
        .or_else(|| body["error"]["message"].as_str())
        .map(|message| message.trim().to_owned())
}

pub fn cloud_config(hostname: &str, password: &str, ssh_keys: &[String]) -> String {
    let config = json!({
        "hostname": hostname,
        "fqdn": hostname,
        "disable_root": false,
        "ssh_pwauth": true,
        "chpasswd": {
            "expire": false,
            "users": [{ "name": "root", "password": password, "type": "text" }],
        },
        "users": [{ "name": "root", "ssh_authorized_keys": ssh_keys }],
    });
    format!("#cloud-config\n{config}\n")
}

fn port_range(ports: &str) -> anyhow::Result<(u16, u16)> {
    let (low, high) = ports.split_once(':').unwrap_or((ports, ports));
    Ok((low.trim().parse()?, high.trim().parse()?))
}

impl OpenStack {
    pub fn new(
        auth_url: &str,
        credentials: Credentials,
        settings: Settings,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            auth_url: auth_url
                .trim_end_matches('/')
                .trim_end_matches("/v3")
                .to_owned(),
            credentials,
            settings,
            http,
            session: OnceCell::new(),
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    async fn authenticate(&self) -> anyhow::Result<Session> {
        let response = self
            .http
            .post(format!("{}/v3/auth/tokens", self.auth_url))
            .json(&json!({
                "auth": {
                    "identity": {
                        "methods": ["application_credential"],
                        "application_credential": {
                            "id": self.credentials.application_credential_id,
                            "secret": self.credentials.application_credential_secret,
                        },
                    },
                },
            }))
            .send()
            .await
            .context("Keystone is niet bereikbaar")?;
        let status = response.status();
        let token = response
            .headers()
            .get("x-subject-token")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            bail!(
                "Keystone antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "inloggen mislukt".to_owned())
            );
        }
        let token = token.context("Keystone gaf geen token terug")?;
        let endpoint = |kind: &str| -> anyhow::Result<String> {
            body["token"]["catalog"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|service| service["type"] == kind)
                .flat_map(|service| service["endpoints"].as_array().into_iter().flatten())
                .find(|endpoint| {
                    endpoint["interface"] == self.settings.interface.as_str()
                        && self.settings.region.as_deref().is_none_or(|region| {
                            endpoint["region_id"] == region || endpoint["region"] == region
                        })
                })
                .and_then(|endpoint| endpoint["url"].as_str())
                .map(|url| url.trim_end_matches('/').to_owned())
                .with_context(|| format!("geen {kind}-endpoint in de catalogus van OpenStack"))
        };
        Ok(Session {
            compute: endpoint("compute")?,
            image: endpoint("image")?,
            network: endpoint("network")?,
            volume: endpoint("block-storage")
                .or_else(|_| endpoint("volumev3"))
                .ok(),
            token,
        })
    }

    async fn send(
        &self,
        service: Service,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<(StatusCode, Value)> {
        let session = self.session.get_or_try_init(|| self.authenticate()).await?;
        let base = match service {
            Service::Compute => &session.compute,
            Service::Image => &session.image,
            Service::Network => &session.network,
            Service::Volume => session
                .volume
                .as_ref()
                .context("deze OpenStack heeft geen blokopslag (Cinder)")?,
        };
        let version = match service {
            Service::Volume => VOLUME_VERSION,
            _ => COMPUTE_VERSION,
        };
        let mut request = self
            .http
            .request(method, format!("{base}{path}"))
            .header("X-Auth-Token", &session.token)
            .header("OpenStack-API-Version", version);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .context("OpenStack is niet bereikbaar")?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    async fn call(
        &self,
        service: Service,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        let (status, body) = self.send(service, method, path, body).await?;
        if !status.is_success() {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        Ok(body)
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        self.call(Service::Compute, Method::GET, "/flavors", None)
            .await?;
        self.call(
            Service::Network,
            Method::GET,
            &format!("/v2.0/networks/{}", self.settings.network_id),
            None,
        )
        .await
        .context("het netwerk is niet gevonden")?;
        Ok(match &self.settings.region {
            Some(region) => format!("OpenStack, regio {region}"),
            None => "OpenStack".to_owned(),
        })
    }

    pub async fn flavors(&self) -> anyhow::Result<Value> {
        let body = self
            .call(Service::Compute, Method::GET, "/flavors/detail", None)
            .await?;
        let mut flavors: Vec<Value> = body["flavors"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|flavor| {
                json!({
                    "id": flavor["id"],
                    "name": flavor["name"],
                    "cores": flavor["vcpus"],
                    "memory_mb": flavor["ram"],
                    "disk_gb": flavor["disk"],
                })
            })
            .collect();
        flavors.sort_by_key(|flavor| (flavor["cores"].as_u64(), flavor["memory_mb"].as_u64()));
        Ok(json!({ "flavors": flavors }))
    }

    async fn server(&self, id: &str) -> anyhow::Result<Value> {
        let body = self
            .call(
                Service::Compute,
                Method::GET,
                &format!("/servers/{id}"),
                None,
            )
            .await?;
        Ok(body["server"].clone())
    }

    async fn action(&self, id: &str, action: Value) -> anyhow::Result<Value> {
        self.call(
            Service::Compute,
            Method::POST,
            &format!("/servers/{id}/action"),
            Some(action),
        )
        .await
    }

    fn ipv4(server: &Value) -> Option<String> {
        server["addresses"]
            .as_object()?
            .values()
            .flat_map(|addresses| addresses.as_array().into_iter().flatten())
            .find(|address| address["version"] == 4)
            .and_then(|address| address["addr"].as_str())
            .map(str::to_owned)
    }

    pub async fn wait_until_active(&self, id: &str) -> anyhow::Result<Option<String>> {
        for _ in 0..300 {
            let server = self.server(id).await?;
            match server["status"].as_str() {
                Some("ACTIVE") => return Ok(Self::ipv4(&server)),
                Some("ERROR") => bail!(
                    "OpenStack kon de server niet maken: {}",
                    server["fault"]["message"]
                        .as_str()
                        .unwrap_or("onbekende fout")
                ),
                _ => tokio::time::sleep(Duration::from_secs(2)).await,
            }
        }
        bail!("de server werd niet op tijd actief")
    }

    async fn create_security_group(&self, name: &str, description: &str) -> anyhow::Result<String> {
        let group = self
            .call(
                Service::Network,
                Method::POST,
                "/v2.0/security-groups",
                Some(json!({ "security_group": { "name": name, "description": description } })),
            )
            .await?;
        let id = group["security_group"]["id"]
            .as_str()
            .context("OpenStack gaf geen security group terug")?
            .to_owned();
        for ethertype in ["IPv4", "IPv6"] {
            self.call(
                Service::Network,
                Method::POST,
                "/v2.0/security-group-rules",
                Some(json!({ "security_group_rule": {
                    "security_group_id": id,
                    "direction": "ingress",
                    "ethertype": ethertype,
                    "description": "Stacklane: alles toestaan",
                } })),
            )
            .await?;
        }
        Ok(id)
    }

    pub async fn create(&self, server: &NewServer<'_>) -> anyhow::Result<String> {
        let group = self
            .create_security_group(
                &format!("stacklane-{}", server.hostname),
                server.description,
            )
            .await?;
        let user_data = STANDARD.encode(cloud_config(
            server.hostname,
            server.password,
            server.ssh_keys,
        ));
        let description: String = server.description.chars().take(255).collect();
        let created = self
            .call(
                Service::Compute,
                Method::POST,
                "/servers",
                Some(json!({ "server": {
                    "name": server.hostname,
                    "imageRef": server.image_id,
                    "flavorRef": server.flavor_id,
                    "networks": [{ "uuid": self.settings.network_id }],
                    "security_groups": [{ "name": group }],
                    "user_data": user_data,
                    "metadata": { "stacklane": description },
                } })),
            )
            .await;
        let created = match created {
            Ok(created) => created,
            Err(error) => {
                let _ = self.delete_security_group(&group).await;
                return Err(error);
            }
        };
        Ok(created["server"]["id"]
            .as_str()
            .context("OpenStack gaf geen server-ID terug")?
            .to_owned())
    }

    pub async fn status(&self, id: &str) -> anyhow::Result<Status> {
        let server = self.server(id).await?;
        let running = server["OS-EXT-STS:power_state"] == 1;
        Ok(Status {
            state: if running { "running" } else { "stopped" }.to_owned(),
            cpu: None,
            cpus: server["flavor"]["vcpus"].as_u64().unwrap_or(0),
            memory: None,
            max_memory: server["flavor"]["ram"].as_u64().unwrap_or(0) * 1024 * 1024,
            uptime: None,
        })
    }

    pub async fn power(&self, id: &str, action: &str) -> anyhow::Result<()> {
        let body = match action {
            "start" => json!({ "os-start": null }),
            "stop" | "shutdown" => json!({ "os-stop": null }),
            "reboot" => json!({ "reboot": { "type": "SOFT" } }),
            _ => bail!("onbekende actie {action}"),
        };
        self.action(id, body).await?;
        Ok(())
    }

    async fn security_groups(&self, id: &str) -> anyhow::Result<Vec<String>> {
        let ports = self
            .call(
                Service::Network,
                Method::GET,
                &format!("/v2.0/ports?device_id={id}"),
                None,
            )
            .await?;
        Ok(ports["ports"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|port| port["security_groups"].as_array().into_iter().flatten())
            .filter_map(|group| group.as_str().map(str::to_owned))
            .collect())
    }

    async fn delete_security_group(&self, group: &str) -> anyhow::Result<()> {
        self.call(
            Service::Network,
            Method::DELETE,
            &format!("/v2.0/security-groups/{group}"),
            None,
        )
        .await?;
        Ok(())
    }

    async fn volume_status(&self, volume: &str) -> anyhow::Result<String> {
        let body = self
            .call(
                Service::Volume,
                Method::GET,
                &format!("/volumes/{volume}"),
                None,
            )
            .await?;
        Ok(body["volume"]["status"]
            .as_str()
            .unwrap_or_default()
            .to_owned())
    }

    async fn wait_for_volume(&self, volume: &str, wanted: &str) -> anyhow::Result<()> {
        for _ in 0..120 {
            let status = self.volume_status(volume).await?;
            if status == wanted {
                return Ok(());
            }
            if status.starts_with("error") {
                bail!("OpenStack meldt een fout bij het volume ({status})");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        bail!("het volume werd niet op tijd {wanted}")
    }

    pub async fn create_volume(
        &self,
        server: &str,
        name: &str,
        size_gb: u32,
    ) -> anyhow::Result<String> {
        let body = self
            .call(
                Service::Volume,
                Method::POST,
                "/volumes",
                Some(json!({ "volume": { "size": size_gb, "name": name } })),
            )
            .await?;
        let volume = body["volume"]["id"]
            .as_str()
            .context("OpenStack gaf geen volume-ID terug")?
            .to_owned();
        self.wait_for_volume(&volume, "available").await?;
        self.attach_volume(server, &volume).await?;
        Ok(volume)
    }

    pub async fn attach_volume(&self, server: &str, volume: &str) -> anyhow::Result<()> {
        self.call(
            Service::Compute,
            Method::POST,
            &format!("/servers/{server}/os-volume_attachments"),
            Some(json!({ "volumeAttachment": { "volumeId": volume } })),
        )
        .await?;
        self.wait_for_volume(volume, "in-use").await
    }

    pub async fn detach_volume(&self, server: &str, volume: &str) -> anyhow::Result<()> {
        if self.volume_status(volume).await? == "available" {
            return Ok(());
        }
        let (status, body) = self
            .send(
                Service::Compute,
                Method::DELETE,
                &format!("/servers/{server}/os-volume_attachments/{volume}"),
                None,
            )
            .await?;
        if !status.is_success() && status != StatusCode::NOT_FOUND {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        self.wait_for_volume(volume, "available").await
    }

    pub async fn delete_volume(&self, server: Option<&str>, volume: &str) -> anyhow::Result<()> {
        if let Some(server) = server {
            self.detach_volume(server, volume).await?;
        }
        let (status, body) = self
            .send(
                Service::Volume,
                Method::DELETE,
                &format!("/volumes/{volume}"),
                None,
            )
            .await?;
        if !status.is_success() && status != StatusCode::NOT_FOUND {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        Ok(())
    }

    pub async fn destroy(&self, id: &str) -> anyhow::Result<()> {
        let groups = self.security_groups(id).await.unwrap_or_default();
        let (status, body) = self
            .send(
                Service::Compute,
                Method::DELETE,
                &format!("/servers/{id}"),
                None,
            )
            .await?;
        if !status.is_success() && status != StatusCode::NOT_FOUND {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        for _ in 0..150 {
            let (status, _) = self
                .send(
                    Service::Compute,
                    Method::GET,
                    &format!("/servers/{id}"),
                    None,
                )
                .await?;
            if status == StatusCode::NOT_FOUND {
                break;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        for group in groups {
            let _ = self.delete_security_group(&group).await;
        }
        for snapshot in self.snapshots(id).await.unwrap_or_default() {
            let _ = self
                .call(
                    Service::Image,
                    Method::DELETE,
                    &format!("/v2/images/{}", snapshot.name),
                    None,
                )
                .await;
        }
        Ok(())
    }

    async fn own_image(&self, id: &str, image: &str) -> anyhow::Result<Value> {
        let image = self
            .call(
                Service::Image,
                Method::GET,
                &format!("/v2/images/{image}"),
                None,
            )
            .await?;
        if image["instance_uuid"] != id {
            bail!("deze snapshot hoort niet bij deze server");
        }
        Ok(image)
    }

    pub async fn snapshots(&self, id: &str) -> anyhow::Result<Vec<Snapshot>> {
        let images = self
            .call(
                Service::Image,
                Method::GET,
                "/v2/images?visibility=private&limit=200&sort_key=created_at&sort_dir=desc",
                None,
            )
            .await?;
        Ok(images["images"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|image| image["instance_uuid"] == id)
            .map(|image| Snapshot {
                name: image["id"].as_str().unwrap_or_default().to_owned(),
                description: image["name"].as_str().unwrap_or_default().to_owned(),
                created_at: image["created_at"]
                    .as_str()
                    .and_then(|created| OffsetDateTime::parse(created, &Rfc3339).ok())
                    .map(OffsetDateTime::unix_timestamp),
            })
            .collect())
    }

    pub async fn create_snapshot(
        &self,
        id: &str,
        name: &str,
        description: &str,
    ) -> anyhow::Result<()> {
        let label = if description.is_empty() {
            name
        } else {
            description
        };
        self.action(id, json!({ "createImage": { "name": label } }))
            .await?;
        Ok(())
    }

    pub async fn rollback_snapshot(&self, id: &str, image: &str) -> anyhow::Result<()> {
        self.own_image(id, image).await?;
        self.action(id, json!({ "rebuild": { "imageRef": image } }))
            .await?;
        Ok(())
    }

    pub async fn delete_snapshot(&self, id: &str, image: &str) -> anyhow::Result<()> {
        self.own_image(id, image).await?;
        self.call(
            Service::Image,
            Method::DELETE,
            &format!("/v2/images/{image}"),
            None,
        )
        .await?;
        Ok(())
    }

    async fn security_group(&self, id: &str) -> anyhow::Result<String> {
        self.security_groups(id)
            .await?
            .into_iter()
            .next()
            .context("deze server heeft geen security group")
    }

    pub async fn firewall_rules(&self, id: &str) -> anyhow::Result<Vec<FirewallRule>> {
        let group = self.security_group(id).await?;
        let rules = self
            .call(
                Service::Network,
                Method::GET,
                &format!("/v2.0/security-group-rules?security_group_id={group}"),
                None,
            )
            .await?;
        Ok(rules["security_group_rules"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|rule| {
                let text = |key: &str| {
                    rule[key]
                        .as_str()
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                };
                let dport = match (
                    rule["port_range_min"].as_u64(),
                    rule["port_range_max"].as_u64(),
                ) {
                    (Some(low), Some(high)) if low == high => Some(low.to_string()),
                    (Some(low), Some(high)) => Some(format!("{low}:{high}")),
                    _ => None,
                };
                FirewallRule {
                    id: text("id").unwrap_or_default(),
                    direction: if rule["direction"] == "egress" {
                        "out"
                    } else {
                        "in"
                    }
                    .to_owned(),
                    action: "ACCEPT".to_owned(),
                    proto: text("protocol"),
                    dport,
                    source: text("remote_ip_prefix"),
                    comment: text("description"),
                }
            })
            .collect())
    }

    pub async fn add_firewall_rule(&self, id: &str, rule: &FirewallRule) -> anyhow::Result<()> {
        if rule.action != "ACCEPT" {
            bail!("OpenStack kent alleen regels die verkeer toestaan");
        }
        let group = self.security_group(id).await?;
        let source = rule.source.as_deref().filter(|source| !source.is_empty());
        let mut body = json!({
            "security_group_id": group,
            "direction": if rule.direction == "out" { "egress" } else { "ingress" },
            "ethertype": if source.is_some_and(|source| source.contains(':')) { "IPv6" } else { "IPv4" },
            "description": rule.comment.clone().unwrap_or_default(),
        });
        if let Some(proto) = rule.proto.as_deref().filter(|proto| !proto.is_empty()) {
            body["protocol"] = json!(proto);
        }
        if let Some(source) = source {
            body["remote_ip_prefix"] = json!(source);
        }
        if let Some(ports) = rule.dport.as_deref().filter(|ports| !ports.is_empty()) {
            if ports.contains(',') {
                bail!("OpenStack kent per regel maar één poort of reeks");
            }
            let (low, high) = port_range(ports)?;
            body["port_range_min"] = json!(low);
            body["port_range_max"] = json!(high);
        }
        self.call(
            Service::Network,
            Method::POST,
            "/v2.0/security-group-rules",
            Some(json!({ "security_group_rule": body })),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_firewall_rule(&self, id: &str, rule: &str) -> anyhow::Result<()> {
        let group = self.security_group(id).await?;
        let found = self
            .call(
                Service::Network,
                Method::GET,
                &format!("/v2.0/security-group-rules/{rule}"),
                None,
            )
            .await?;
        if found["security_group_rule"]["security_group_id"] != group.as_str() {
            bail!("deze regel hoort niet bij deze server");
        }
        self.call(
            Service::Network,
            Method::DELETE,
            &format!("/v2.0/security-group-rules/{rule}"),
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
    use super::{cloud_config, error_message, port_range};
    use serde_json::json;

    #[test]
    fn builds_cloud_config_as_json() {
        let config = cloud_config(
            "web-01",
            "geheim\"wachtwoord",
            &["ssh-ed25519 AAAA jan".to_owned()],
        );
        let (header, body) = config.split_once('\n').unwrap();
        assert_eq!(header, "#cloud-config");
        let parsed: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(
            parsed["chpasswd"]["users"][0]["password"],
            "geheim\"wachtwoord"
        );
        assert_eq!(
            parsed["users"][0]["ssh_authorized_keys"][0],
            "ssh-ed25519 AAAA jan"
        );
    }

    #[test]
    fn reads_error_messages_and_ports() {
        assert_eq!(
            error_message(&json!({ "badRequest": { "message": "Invalid flavor", "code": 400 } }))
                .as_deref(),
            Some("Invalid flavor")
        );
        assert_eq!(
            error_message(&json!({ "NeutronError": { "message": "Quota exceeded" } })).as_deref(),
            Some("Quota exceeded")
        );
        assert_eq!(error_message(&json!({ "error": { "message": "The request you have made requires authentication." } })).as_deref(), Some("The request you have made requires authentication."));
        assert_eq!(port_range("22").unwrap(), (22, 22));
        assert_eq!(port_range("8000:8100").unwrap(), (8000, 8100));
        assert!(port_range("ssh").is_err());
    }
}
