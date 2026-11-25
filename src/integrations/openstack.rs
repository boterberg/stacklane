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
    #[serde(default)]
    pub floating_network_id: Option<String>,
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
const RESCUE_VERSION: &str = "compute 2.87";
const REBUILD_VERSION: &str = "compute 2.57";

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

fn floating_ip_of(body: &Value) -> anyhow::Result<(String, String)> {
    let ip = &body["floatingip"];
    match (ip["id"].as_str(), ip["floating_ip_address"].as_str()) {
        (Some(id), Some(address)) => Ok((id.to_owned(), address.to_owned())),
        _ => bail!("OpenStack gaf geen floating IP terug"),
    }
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
            .redirect(reqwest::redirect::Policy::none())
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
        let version = match service {
            Service::Volume => VOLUME_VERSION,
            _ => COMPUTE_VERSION,
        };
        self.send_as(service, version, method, path, body).await
    }

    async fn send_as(
        &self,
        service: Service,
        version: &str,
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
        if self.has_floating_ips() {
            let network = self
                .call(
                    Service::Network,
                    Method::GET,
                    &format!("/v2.0/networks/{}", self.floating_network()?),
                    None,
                )
                .await
                .context("het netwerk voor floating IPs is niet gevonden")?;
            anyhow::ensure!(
                network["network"]["router:external"] == true,
                "het netwerk voor floating IPs is geen extern netwerk"
            );
        }
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
            let rule = self
                .call(
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
                .await;
            if let Err(error) = rule {
                self.delete_security_group(&id).await.ok();
                return Err(error);
            }
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

    pub async fn rebuild(
        &self,
        id: &str,
        image_id: &str,
        hostname: &str,
        password: &str,
        ssh_keys: &[String],
    ) -> anyhow::Result<()> {
        let user_data = STANDARD.encode(cloud_config(hostname, password, ssh_keys));
        let (status, body) = self
            .send_as(
                Service::Compute,
                REBUILD_VERSION,
                Method::POST,
                &format!("/servers/{id}/action"),
                Some(json!({ "rebuild": {
                    "imageRef": image_id,
                    "adminPass": password,
                    "user_data": user_data,
                } })),
            )
            .await?;
        if !status.is_success() {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        self.wait_until_active(id).await?;
        Ok(())
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

    pub async fn suspend(&self, id: &str) -> anyhow::Result<()> {
        if self.status(id).await?.state == "running" {
            self.power(id, "stop").await?;
        }
        Ok(())
    }

    pub async fn unsuspend(&self, id: &str) -> anyhow::Result<()> {
        if self.status(id).await?.state != "running" {
            self.power(id, "start").await?;
        }
        Ok(())
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
        let (status, body) = self
            .send(
                Service::Network,
                Method::DELETE,
                &format!("/v2.0/security-groups/{group}"),
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

    pub fn has_floating_ips(&self) -> bool {
        self.floating_network().is_ok()
    }

    fn floating_network(&self) -> anyhow::Result<&str> {
        self.settings
            .floating_network_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .context("er is geen extern netwerk voor floating IPs ingesteld")
    }

    pub async fn create_floating_ip(&self, description: &str) -> anyhow::Result<(String, String)> {
        let body = self
            .call(
                Service::Network,
                Method::POST,
                "/v2.0/floatingips",
                Some(json!({ "floatingip": {
                    "floating_network_id": self.floating_network()?,
                    "description": description,
                } })),
            )
            .await?;
        floating_ip_of(&body)
    }

    pub async fn floating_ip(&self, id: &str) -> anyhow::Result<Option<(String, String)>> {
        let (status, body) = self
            .send(
                Service::Network,
                Method::GET,
                &format!("/v2.0/floatingips/{id}"),
                None,
            )
            .await?;
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        floating_ip_of(&body).map(Some)
    }

    async fn server_port(&self, server: &str) -> anyhow::Result<String> {
        let ports = self
            .call(
                Service::Network,
                Method::GET,
                &format!("/v2.0/ports?device_id={server}"),
                None,
            )
            .await?;
        let ports = ports["ports"].as_array().cloned().unwrap_or_default();
        ports
            .iter()
            .find(|port| port["network_id"] == self.settings.network_id.as_str())
            .or_else(|| ports.first())
            .and_then(|port| port["id"].as_str())
            .map(str::to_owned)
            .context("de server heeft geen netwerkpoort")
    }

    pub async fn attach_floating_ip(&self, id: &str, server: &str) -> anyhow::Result<()> {
        let port = self.server_port(server).await?;
        self.call(
            Service::Network,
            Method::PUT,
            &format!("/v2.0/floatingips/{id}"),
            Some(json!({ "floatingip": { "port_id": port } })),
        )
        .await?;
        Ok(())
    }

    pub async fn detach_floating_ip(&self, id: &str) -> anyhow::Result<()> {
        self.call(
            Service::Network,
            Method::PUT,
            &format!("/v2.0/floatingips/{id}"),
            Some(json!({ "floatingip": { "port_id": null } })),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_floating_ip(&self, id: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Service::Network,
                Method::DELETE,
                &format!("/v2.0/floatingips/{id}"),
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

    pub async fn create_network(&self, name: &str, cidr: &str) -> anyhow::Result<String> {
        let network = self
            .call(
                Service::Network,
                Method::POST,
                "/v2.0/networks",
                Some(json!({ "network": { "name": name, "port_security_enabled": false } })),
            )
            .await?;
        let id = network["network"]["id"]
            .as_str()
            .context("OpenStack gaf geen netwerk terug")?
            .to_owned();
        let subnet = self
            .call(
                Service::Network,
                Method::POST,
                "/v2.0/subnets",
                Some(json!({ "subnet": {
                    "network_id": id,
                    "name": name,
                    "cidr": cidr,
                    "ip_version": 4,
                    "gateway_ip": null,
                    "enable_dhcp": true,
                } })),
            )
            .await;
        if let Err(error) = subnet {
            self.delete_network(&id).await.ok();
            return Err(error);
        }
        Ok(id)
    }

    pub async fn delete_network(&self, id: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Service::Network,
                Method::DELETE,
                &format!("/v2.0/networks/{id}"),
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

    pub async fn attach_network(
        &self,
        server: &str,
        network: &str,
    ) -> anyhow::Result<(String, String)> {
        let body = self
            .call(
                Service::Compute,
                Method::POST,
                &format!("/servers/{server}/os-interface"),
                Some(json!({ "interfaceAttachment": { "net_id": network } })),
            )
            .await?;
        let attachment = &body["interfaceAttachment"];
        let port = attachment["port_id"]
            .as_str()
            .context("OpenStack gaf geen poort terug")?;
        let address = attachment["fixed_ips"]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(|ip| ip["ip_address"].as_str())
            .context("OpenStack gaf geen IP-adres terug")?;
        Ok((port.to_owned(), address.to_owned()))
    }

    pub async fn detach_network(&self, server: &str, port: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Service::Compute,
                Method::DELETE,
                &format!("/servers/{server}/os-interface/{port}"),
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

    pub async fn import_iso(&self, name: &str, url: &str, uefi: bool) -> anyhow::Result<String> {
        let mut image = json!({
            "name": name,
            "disk_format": "iso",
            "container_format": "bare",
            "visibility": "private",
            "hw_rescue_device": "cdrom",
            "hw_rescue_bus": "sata",
        });
        if uefi {
            image["hw_firmware_type"] = json!("uefi");
        }
        let created = self
            .call(Service::Image, Method::POST, "/v2/images", Some(image))
            .await?;
        let id = created["id"]
            .as_str()
            .context("OpenStack gaf geen image-ID terug")?
            .to_owned();
        let (status, body) = self
            .send(
                Service::Image,
                Method::POST,
                &format!("/v2/images/{id}/import"),
                Some(json!({ "method": { "name": "web-download", "uri": url } })),
            )
            .await?;
        if !status.is_success() {
            let _ = self.delete_iso(&id).await;
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "importeren mislukt".to_owned())
            );
        }
        let mut failures = 0;
        for _ in 0..1800 {
            let image = match self
                .call(
                    Service::Image,
                    Method::GET,
                    &format!("/v2/images/{id}"),
                    None,
                )
                .await
            {
                Ok(image) => {
                    failures = 0;
                    image
                }
                Err(error) => {
                    failures += 1;
                    if failures >= 5 {
                        let _ = self.delete_iso(&id).await;
                        return Err(error);
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };
            match image["status"].as_str() {
                Some("active") => return Ok(id),
                Some("killed" | "deleted" | "deactivated") => {
                    let _ = self.delete_iso(&id).await;
                    bail!("OpenStack kon de ISO niet ophalen");
                }
                _ => tokio::time::sleep(Duration::from_secs(2)).await,
            }
        }
        let _ = self.delete_iso(&id).await;
        bail!("het ophalen van de ISO duurde te lang")
    }

    pub async fn delete_iso(&self, image: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Service::Image,
                Method::DELETE,
                &format!("/v2/images/{image}"),
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

    async fn rescue_action(&self, id: &str, body: Value) -> anyhow::Result<()> {
        let (status, reply) = self
            .send_as(
                Service::Compute,
                RESCUE_VERSION,
                Method::POST,
                &format!("/servers/{id}/action"),
                Some(body),
            )
            .await?;
        if !status.is_success() {
            bail!(
                "OpenStack antwoordde {status}: {}",
                error_message(&reply).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        Ok(())
    }

    pub async fn mount_iso(&self, id: &str, image: &str) -> anyhow::Result<()> {
        self.rescue_action(id, json!({ "rescue": { "rescue_image_ref": image } }))
            .await
    }

    pub async fn unmount_iso(&self, id: &str) -> anyhow::Result<()> {
        self.unrescue(id).await
    }

    pub async fn rescue(&self, id: &str, password: &str) -> anyhow::Result<()> {
        self.rescue_action(id, json!({ "rescue": { "adminPass": password } }))
            .await
    }

    pub async fn unrescue(&self, id: &str) -> anyhow::Result<()> {
        self.rescue_action(id, json!({ "unrescue": null })).await
    }

    pub async fn destroy(&self, id: &str) -> anyhow::Result<()> {
        let groups = self.security_groups(id).await?;
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
        let mut left = Vec::new();
        for group in groups {
            let mut deleted = false;
            for attempt in 0..5 {
                if attempt > 0 {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
                if self.delete_security_group(&group).await.is_ok() {
                    deleted = true;
                    break;
                }
            }
            if !deleted {
                left.push(group);
            }
        }
        for snapshot in self.snapshots(id).await? {
            let (status, _) = self
                .send(
                    Service::Image,
                    Method::DELETE,
                    &format!("/v2/images/{}", snapshot.name),
                    None,
                )
                .await?;
            if !status.is_success() && status != StatusCode::NOT_FOUND {
                left.push(snapshot.name);
            }
        }
        if !left.is_empty() {
            bail!(
                "snapshots konden niet worden verwijderd: {}",
                left.join(", ")
            );
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
        let mut found = Vec::new();
        let mut path =
            format!("/v2/images?instance_uuid={id}&limit=200&sort_key=created_at&sort_dir=desc");
        for _ in 0..50 {
            let page = self.call(Service::Image, Method::GET, &path, None).await?;
            found.extend(
                page["images"]
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
                    }),
            );
            match page["next"].as_str() {
                Some(next) if next.starts_with("/v2/images?") => path = next.to_owned(),
                _ => return Ok(found),
            }
        }
        bail!("OpenStack gaf te veel pagina's met images terug")
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
    use super::{Credentials, OpenStack, Value, cloud_config, error_message, port_range};

    #[tokio::test]
    async fn lists_every_snapshot_page_of_a_server() {
        use axum::extract::{Query, State};
        use std::collections::HashMap;

        async fn token(State(base): State<String>) -> impl axum::response::IntoResponse {
            let endpoint = |kind: &str| serde_json::json!({ "type": kind, "endpoints": [{ "interface": "public", "url": base }] });
            (
                [("x-subject-token", "t")],
                axum::Json(serde_json::json!({ "token": { "catalog": [
                    endpoint("compute"), endpoint("image"), endpoint("network"),
                ] } })),
            )
        }
        async fn images(Query(query): Query<HashMap<String, String>>) -> axum::Json<Value> {
            let image = |id: &str, server: &str| serde_json::json!({ "id": id, "name": id, "instance_uuid": server });
            axum::Json(if query.contains_key("marker") {
                serde_json::json!({ "images": [image("old-1", "srv"), image("old-2", "srv")] })
            } else {
                serde_json::json!({
                    "images": [image("new", "srv"), image("other", "elsewhere")],
                    "next": "/v2/images?marker=other&instance_uuid=srv&limit=200",
                })
            })
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new()
            .route("/v3/auth/tokens", axum::routing::post(token))
            .route("/v2/images", axum::routing::get(images))
            .with_state(base.clone());
        tokio::spawn(async move { axum::serve(listener, app).await });
        let client = OpenStack::new(
            &base,
            Credentials {
                application_credential_id: "id".into(),
                application_credential_secret: "secret".into(),
            },
            serde_json::from_value(serde_json::json!({ "network_id": "net" })).unwrap(),
        )
        .unwrap();
        let names: Vec<String> = client
            .snapshots("srv")
            .await
            .unwrap()
            .into_iter()
            .map(|snapshot| snapshot.name)
            .collect();
        assert_eq!(names, ["new", "old-1", "old-2"]);
    }
    #[tokio::test]
    async fn floating_ips_go_through_neutron() {
        use axum::extract::{Request, State};
        use std::sync::{Arc, Mutex};

        type Shared = (String, Arc<Mutex<Vec<String>>>);
        async fn token(State((base, _)): State<Shared>) -> impl axum::response::IntoResponse {
            let endpoint = |kind: &str| serde_json::json!({ "type": kind, "endpoints": [{ "interface": "public", "url": base }] });
            (
                [("x-subject-token", "t")],
                axum::Json(serde_json::json!({ "token": { "catalog": [
                    endpoint("compute"), endpoint("image"), endpoint("network"),
                ] } })),
            )
        }
        async fn neutron(
            State((_, calls)): State<Shared>,
            request: Request,
        ) -> (axum::http::StatusCode, axum::Json<Value>) {
            let method = request.method().to_string();
            let path = request.uri().to_string();
            let body = axum::body::to_bytes(request.into_body(), 1 << 16)
                .await
                .unwrap_or_default();
            calls.lock().unwrap().push(
                format!("{method} {path} {}", String::from_utf8_lossy(&body))
                    .trim_end()
                    .to_owned(),
            );
            let fip = serde_json::json!({ "floatingip": { "id": "fip", "floating_ip_address": "203.0.113.5" } });
            match (method.as_str(), path.as_str()) {
                ("GET", "/v2.0/ports?device_id=srv") => (
                    axum::http::StatusCode::OK,
                    axum::Json(serde_json::json!({ "ports": [
                        { "id": "private", "network_id": "other" },
                        { "id": "public", "network_id": "net" },
                    ] })),
                ),
                ("DELETE", _) => (axum::http::StatusCode::NOT_FOUND, axum::Json(Value::Null)),
                _ => (axum::http::StatusCode::OK, axum::Json(fip)),
            }
        }
        let calls = Arc::new(Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new()
            .route("/v3/auth/tokens", axum::routing::post(token))
            .fallback(neutron)
            .with_state((base.clone(), calls.clone()));
        tokio::spawn(async move { axum::serve(listener, app).await });
        let client = |settings: Value| {
            OpenStack::new(
                &base,
                Credentials {
                    application_credential_id: "id".into(),
                    application_credential_secret: "secret".into(),
                },
                serde_json::from_value(settings).unwrap(),
            )
            .unwrap()
        };
        assert!(!client(json!({ "network_id": "net" })).has_floating_ips());
        assert!(
            !client(json!({ "network_id": "net", "floating_network_id": " " })).has_floating_ips()
        );
        let client = client(json!({ "network_id": "net", "floating_network_id": "ext" }));
        assert!(client.has_floating_ips());

        let created = client.create_floating_ip("Stacklane x").await.unwrap();
        assert_eq!(created, ("fip".to_owned(), "203.0.113.5".to_owned()));
        client.attach_floating_ip("fip", "srv").await.unwrap();
        client.detach_floating_ip("fip").await.unwrap();
        client.delete_floating_ip("fip").await.unwrap();
        client.rescue("srv", "tijdelijk").await.unwrap();
        client.unrescue("srv").await.unwrap();

        let calls = calls.lock().unwrap().clone();
        let parsed = |index: usize| -> Value {
            serde_json::from_str(
                calls[index]
                    .split_once(' ')
                    .unwrap()
                    .1
                    .split_once(' ')
                    .unwrap()
                    .1,
            )
            .unwrap()
        };
        assert!(calls[0].starts_with("POST /v2.0/floatingips "));
        assert_eq!(parsed(0)["floatingip"]["floating_network_id"], "ext");
        assert!(calls[2].starts_with("PUT /v2.0/floatingips/fip "));
        assert_eq!(parsed(2)["floatingip"]["port_id"], "public");
        assert_eq!(parsed(3)["floatingip"]["port_id"], Value::Null);
        assert_eq!(calls[4], "DELETE /v2.0/floatingips/fip");
        assert!(calls[5].starts_with("POST /servers/srv/action "));
        assert_eq!(parsed(5)["rescue"]["adminPass"], "tijdelijk");
        assert!(parsed(6).as_object().unwrap().contains_key("unrescue"));
    }

    #[tokio::test]
    async fn private_networks_go_through_neutron_and_nova() {
        use axum::extract::{Request, State};
        use std::sync::{Arc, Mutex};

        type Shared = (String, Arc<Mutex<Vec<String>>>);
        async fn token(State((base, _)): State<Shared>) -> impl axum::response::IntoResponse {
            let endpoint = |kind: &str| serde_json::json!({ "type": kind, "endpoints": [{ "interface": "public", "url": base }] });
            (
                [("x-subject-token", "t")],
                axum::Json(serde_json::json!({ "token": { "catalog": [
                    endpoint("compute"), endpoint("image"), endpoint("network"),
                ] } })),
            )
        }
        async fn api(
            State((_, calls)): State<Shared>,
            request: Request,
        ) -> (axum::http::StatusCode, axum::Json<Value>) {
            let method = request.method().to_string();
            let path = request.uri().to_string();
            let body = axum::body::to_bytes(request.into_body(), 1 << 16)
                .await
                .unwrap_or_default();
            let body = String::from_utf8_lossy(&body).into_owned();
            calls
                .lock()
                .unwrap()
                .push(format!("{method} {path} {body}").trim_end().to_owned());
            let ok = axum::http::StatusCode::OK;
            match (method.as_str(), path.as_str()) {
                ("POST", "/v2.0/networks") => (
                    ok,
                    axum::Json(serde_json::json!({ "network": { "id": "lan" } })),
                ),
                ("POST", "/v2.0/subnets") if body.contains("10.9.0.0/24") => (
                    axum::http::StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({ "NeutronError": { "message": "overlap" } })),
                ),
                ("POST", "/servers/srv/os-interface") => (
                    ok,
                    axum::Json(serde_json::json!({ "interfaceAttachment": {
                        "port_id": "port-1",
                        "fixed_ips": [{ "ip_address": "10.8.0.5", "subnet_id": "s" }],
                    } })),
                ),
                ("DELETE", _) => (axum::http::StatusCode::NOT_FOUND, axum::Json(Value::Null)),
                _ => (
                    ok,
                    axum::Json(serde_json::json!({ "subnet": { "id": "s" } })),
                ),
            }
        }
        let calls = Arc::new(Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new()
            .route("/v3/auth/tokens", axum::routing::post(token))
            .fallback(api)
            .with_state((base.clone(), calls.clone()));
        tokio::spawn(async move { axum::serve(listener, app).await });
        let client = OpenStack::new(
            &base,
            Credentials {
                application_credential_id: "id".into(),
                application_credential_secret: "secret".into(),
            },
            serde_json::from_value(json!({ "network_id": "net" })).unwrap(),
        )
        .unwrap();

        assert_eq!(
            client
                .create_network("stacklane-a", "10.8.0.0/24")
                .await
                .unwrap(),
            "lan"
        );
        assert!(
            client
                .create_network("stacklane-b", "10.9.0.0/24")
                .await
                .is_err()
        );
        assert_eq!(
            client.attach_network("srv", "lan").await.unwrap(),
            ("port-1".to_owned(), "10.8.0.5".to_owned())
        );
        client.detach_network("srv", "port-1").await.unwrap();
        client.delete_network("lan").await.unwrap();

        let calls = calls.lock().unwrap().clone();
        assert!(calls[0].starts_with("POST /v2.0/networks "));
        assert!(calls[0].contains("\"port_security_enabled\":false"));
        assert!(calls[1].contains("\"gateway_ip\":null"));
        assert_eq!(calls[4], "DELETE /v2.0/networks/lan");
        assert!(calls[5].contains("\"net_id\":\"lan\""));
        assert_eq!(calls[6], "DELETE /servers/srv/os-interface/port-1");
        assert_eq!(calls[7], "DELETE /v2.0/networks/lan");
    }

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
