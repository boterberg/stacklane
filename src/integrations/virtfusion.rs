use std::time::Duration;

use anyhow::{Context, bail};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::vps::Status;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub api_token: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Image {
    pub template: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub hypervisor_group_id: u32,
    #[serde(default)]
    pub images: Vec<Image>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProductConfig {
    pub package_id: u32,
    #[serde(default = "one")]
    pub ipv4: u32,
    #[serde(default)]
    pub cores: u32,
    #[serde(default)]
    pub memory_mb: u32,
    #[serde(default)]
    pub disk_gb: u32,
}

fn one() -> u32 {
    1
}

pub struct Owner<'a> {
    pub reference: &'a str,
    pub name: &'a str,
    pub email: &'a str,
}

pub struct Created {
    pub id: u64,
    pub key_ids: Vec<u64>,
}

pub struct VirtFusion {
    base_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
}

fn error_message(body: &Value) -> Option<String> {
    body["msg"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            body["errors"].as_array().map(|errors| {
                errors
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        })
        .or_else(|| body.as_str().map(str::to_owned))
        .filter(|message| !message.is_empty())
}

fn ipv4(server: &Value) -> Option<String> {
    server["network"]["interfaces"]
        .as_array()?
        .iter()
        .flat_map(|interface| interface["ipv4"].as_array().into_iter().flatten())
        .find_map(|address| address["address"].as_str().map(str::to_owned))
}

fn power_state(server: &Value) -> &str {
    server["remoteState"]["state"]
        .as_str()
        .or_else(|| server["state"].as_str())
        .unwrap_or("unknown")
}

impl VirtFusion {
    pub fn new(
        base_url: &str,
        credentials: Credentials,
        settings: Settings,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url
                .trim_end_matches('/')
                .trim_end_matches("/api/v1")
                .to_owned(),
            credentials,
            settings,
            http,
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<(StatusCode, Value)> {
        let mut request = self
            .http
            .request(method, format!("{}/api/v1{path}", self.base_url))
            .bearer_auth(&self.credentials.api_token)
            .header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .context("VirtFusion is niet bereikbaar")?;
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            bail!("VirtFusion weigert het API-token");
        }
        let body: Value = response.json().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> anyhow::Result<Value> {
        let (status, body) = self.send(method, path, body).await?;
        if !status.is_success() {
            bail!(
                "VirtFusion antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        Ok(body)
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        self.call(Method::GET, "/connect", None).await?;
        let groups = self
            .call(Method::GET, "/compute/hypervisors/groups?results=200", None)
            .await?;
        let group = groups["data"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|group| group["id"] == self.settings.hypervisor_group_id)
            .with_context(|| {
                format!(
                    "hypervisorgroep {} niet gevonden",
                    self.settings.hypervisor_group_id
                )
            })?;
        Ok(format!(
            "VirtFusion, groep {}",
            group["name"].as_str().unwrap_or("?")
        ))
    }

    pub async fn packages(&self) -> anyhow::Result<Value> {
        let body = self.call(Method::GET, "/packages", None).await?;
        let packages: Vec<Value> = body["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|package| package["enabled"] != false)
            .map(|package| {
                json!({
                    "id": package["id"],
                    "name": package["name"],
                    "cores": package["cpuCores"],
                    "memory_mb": package["memory"],
                    "disk_gb": package["primaryStorage"],
                })
            })
            .collect();
        Ok(json!({ "packages": packages }))
    }

    pub async fn change_email(&self, reference: &str, email: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Method::PUT,
                &format!("/users/{reference}/byExtRelation?relStr=true"),
                Some(json!({ "email": email })),
            )
            .await?;
        if status.is_success() || status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        bail!(
            "VirtFusion antwoordde {status}: {}",
            error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
        )
    }

    pub async fn forget_user(&self, reference: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Method::DELETE,
                &format!("/users/{reference}/byExtRelation?relStr=true"),
                None,
            )
            .await?;
        if status.is_success() || status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        bail!(
            "VirtFusion antwoordde {status}: {}",
            error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
        )
    }

    async fn user(&self, owner: &Owner<'_>) -> anyhow::Result<u64> {
        let (status, body) = self
            .send(
                Method::GET,
                &format!("/users/{}/byExtRelation?relStr=true", owner.reference),
                None,
            )
            .await?;
        if status.is_success() {
            return body["data"]["id"]
                .as_u64()
                .context("VirtFusion gaf geen gebruiker terug");
        }
        if status != StatusCode::NOT_FOUND {
            bail!(
                "VirtFusion antwoordde {status}: {}",
                error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
            );
        }
        let created = self
            .call(
                Method::POST,
                "/users",
                Some(json!({
                    "name": owner.name,
                    "email": owner.email,
                    "relStr": owner.reference,
                    "sendMail": false,
                })),
            )
            .await?;
        created["data"]["id"]
            .as_u64()
            .context("VirtFusion gaf geen gebruiker terug")
    }

    pub async fn create(
        &self,
        owner: &Owner<'_>,
        config: &ProductConfig,
        ssh_keys: &[(String, String)],
    ) -> anyhow::Result<Created> {
        let user_id = self.user(owner).await?;
        let mut key_ids = Vec::new();
        for (name, key) in ssh_keys {
            let added = self
                .call(
                    Method::POST,
                    "/ssh_keys",
                    Some(json!({ "userId": user_id, "name": name, "publicKey": key })),
                )
                .await?;
            if let Some(id) = added["data"]["id"].as_u64() {
                key_ids.push(id);
            }
        }
        let created = self
            .call(
                Method::POST,
                "/servers",
                Some(json!({
                    "packageId": config.package_id,
                    "userId": user_id,
                    "hypervisorId": self.settings.hypervisor_group_id,
                    "ipv4": config.ipv4,
                })),
            )
            .await?;
        let id = created["data"]["id"]
            .as_u64()
            .context("VirtFusion gaf geen server-ID terug")?;
        Ok(Created { id, key_ids })
    }

    pub async fn build(
        &self,
        id: u64,
        template: u32,
        hostname: &str,
        key_ids: &[u64],
    ) -> anyhow::Result<Option<String>> {
        let built = self
            .call(
                Method::POST,
                &format!("/servers/{id}/build"),
                Some(json!({
                    "operatingSystemId": template,
                    "name": hostname,
                    "hostname": hostname,
                    "sshKeys": key_ids,
                    "email": false,
                })),
            )
            .await?;
        Ok(ipv4(&built["data"]))
    }

    async fn server(&self, id: &str, live: bool) -> anyhow::Result<Value> {
        let body = self
            .call(
                Method::GET,
                &format!("/servers/{id}?remoteState={live}"),
                None,
            )
            .await?;
        Ok(body["data"].clone())
    }

    pub async fn ipv4(&self, id: u64) -> anyhow::Result<Option<String>> {
        Ok(ipv4(&self.server(&id.to_string(), false).await?))
    }

    pub async fn status(&self, id: &str) -> anyhow::Result<Status> {
        let server = self.server(id, true).await?;
        let resources = &server["settings"]["resources"];
        Ok(Status {
            state: if power_state(&server) == "running" {
                "running"
            } else {
                "stopped"
            }
            .to_owned(),
            cpu: None,
            cpus: resources["cpuCores"].as_u64().unwrap_or(0),
            memory: None,
            max_memory: resources["memory"].as_u64().unwrap_or(0) * 1024 * 1024,
            uptime: None,
        })
    }

    pub async fn power(&self, id: &str, action: &str) -> anyhow::Result<()> {
        let action = match action {
            "start" => "boot",
            "shutdown" => "shutdown",
            "stop" => "poweroff",
            "reboot" => "restart",
            _ => bail!("onbekende actie {action}"),
        };
        self.call(Method::POST, &format!("/servers/{id}/power/{action}"), None)
            .await?;
        Ok(())
    }

    pub async fn suspend(&self, id: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(
                Method::POST,
                &format!("/servers/{id}/suspend"),
                Some(json!({ "reason": "Opgeschort vanuit Stacklane" })),
            )
            .await?;
        if status.is_success() || status == StatusCode::CONFLICT {
            return Ok(());
        }
        bail!(
            "VirtFusion antwoordde {status}: {}",
            error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
        )
    }

    pub async fn unsuspend(&self, id: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(Method::POST, &format!("/servers/{id}/unsuspend"), None)
            .await?;
        if status.is_success() || status == StatusCode::CONFLICT {
            return Ok(());
        }
        bail!(
            "VirtFusion antwoordde {status}: {}",
            error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
        )
    }

    pub async fn destroy(&self, id: &str) -> anyhow::Result<()> {
        let (status, body) = self
            .send(Method::DELETE, &format!("/servers/{id}?delay=0"), None)
            .await?;
        if status.is_success() || status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        bail!(
            "VirtFusion antwoordde {status}: {}",
            error_message(&body).unwrap_or_else(|| "onbekende fout".to_owned())
        )
    }

    pub async fn login_url(&self, reference: &str, id: &str) -> anyhow::Result<String> {
        let body = self
            .call(
                Method::POST,
                &format!("/users/{reference}/serverAuthenticationTokens/{id}?relStr=true"),
                Some(json!({})),
            )
            .await?;
        let path = body["data"]["authentication"]["endpoint_complete"]
            .as_str()
            .context("VirtFusion gaf geen inloglink terug")?;
        Ok(format!("{}{path}", self.base_url))
    }
}

pub fn product_details(config: &ProductConfig) -> Value {
    json!({ "cores": config.cores, "memory_mb": config.memory_mb, "disk_gb": config.disk_gb })
}

#[cfg(test)]
mod tests {
    use super::{error_message, ipv4, power_state};
    use serde_json::json;

    #[test]
    fn reads_servers_and_errors() {
        let server = json!({
            "state": "complete",
            "remoteState": { "state": "running" },
            "network": { "interfaces": [{ "ipv4": [{ "address": "203.0.113.9" }] }] },
        });
        assert_eq!(ipv4(&server).as_deref(), Some("203.0.113.9"));
        assert_eq!(power_state(&server), "running");
        assert_eq!(power_state(&json!({ "state": "queued" })), "queued");
        assert_eq!(
            error_message(&json!({ "msg": "server not found" })).as_deref(),
            Some("server not found")
        );
        assert_eq!(
            error_message(&json!({ "errors": ["packageId is invalid", "userId is invalid"] }))
                .as_deref(),
            Some("packageId is invalid, userId is invalid")
        );
        assert_eq!(
            error_message(&json!("server already suspended")).as_deref(),
            Some("server already suspended")
        );
    }
}
