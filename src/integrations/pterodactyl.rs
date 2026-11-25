use std::time::Duration;

use anyhow::Context;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_READ: usize = 2 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub application_key: String,
    pub client_key: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProductConfig {
    pub location_id: i64,
    pub nest_id: i64,
    pub egg_id: i64,
    pub memory: i64,
    pub disk: i64,
    pub cpu: i64,
    #[serde(default)]
    pub databases: i64,
    #[serde(default = "one")]
    pub allocations: i64,
    #[serde(default = "one")]
    pub backups: i64,
}

fn one() -> i64 {
    1
}

pub struct Owner<'a> {
    pub email: &'a str,
    pub email_verified: bool,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub stacklane_id: &'a str,
}

#[derive(Serialize)]
pub struct Provisioned {
    pub external_id: String,
    pub details: Value,
}

#[derive(Serialize)]
pub struct CatalogEgg {
    id: i64,
    name: String,
}

#[derive(Serialize)]
pub struct CatalogNest {
    id: i64,
    name: String,
    eggs: Vec<CatalogEgg>,
}

#[derive(Serialize)]
pub struct CatalogLocation {
    id: i64,
    name: String,
}

#[derive(Serialize)]
pub struct Catalog {
    locations: Vec<CatalogLocation>,
    nests: Vec<CatalogNest>,
}

pub struct Pterodactyl {
    base_url: String,
    credentials: Credentials,
    http: reqwest::Client,
}

fn pterodactyl_error(status: reqwest::StatusCode, body: &Value) -> anyhow::Error {
    let detail = body["errors"]
        .as_array()
        .and_then(|errors| errors.first())
        .and_then(|error| error["detail"].as_str())
        .unwrap_or("onbekende fout");
    anyhow::anyhow!("Pterodactyl antwoordde {status}: {detail}")
}

fn username_for(email: &str) -> String {
    let local: String = email
        .split('@')
        .next()
        .unwrap_or("klant")
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(20)
        .collect::<String>()
        .to_lowercase();
    let local = if local.is_empty() {
        "klant".to_owned()
    } else {
        local
    };
    format!("{local}{}", hex::encode(rand::random::<[u8; 2]>()))
}

impl Pterodactyl {
    pub fn new(base_url: &str, credentials: Credentials) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            credentials,
            http,
        })
    }

    async fn request(
        &self,
        key: &str,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(key)
            .header("Accept", "application/json")
            .query(query);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .context("Pterodactyl is niet bereikbaar")?;
        let status = response.status();
        let text = response.text().await?;
        let body: Value = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        if !status.is_success() {
            return Err(pterodactyl_error(status, &body));
        }
        Ok(body)
    }

    async fn application(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.request(
            &self.credentials.application_key,
            method,
            &format!("/api/application{path}"),
            query,
            body,
        )
        .await
    }

    async fn client(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.request(
            &self.credentials.client_key,
            method,
            &format!("/api/client{path}"),
            query,
            body,
        )
        .await
    }

    async fn list(&self, path: &str) -> anyhow::Result<Vec<Value>> {
        let mut items = Vec::new();
        let mut page = 1;
        loop {
            let body = self
                .application(
                    Method::GET,
                    path,
                    &[("page", page.to_string()), ("per_page", "100".into())],
                    None,
                )
                .await?;
            items.extend(body["data"].as_array().cloned().unwrap_or_default());
            let total_pages = body["meta"]["pagination"]["total_pages"]
                .as_i64()
                .unwrap_or(1);
            if page >= total_pages {
                return Ok(items);
            }
            page += 1;
        }
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        let nodes = self.list("/nodes").await?;
        self.client(Method::GET, "", &[], None)
            .await
            .context("de client-sleutel werkt niet")?;
        Ok(match nodes.len() {
            1 => "1 node gevonden".to_owned(),
            count => format!("{count} nodes gevonden"),
        })
    }

    pub async fn catalog(&self) -> anyhow::Result<Catalog> {
        let locations = self
            .list("/locations")
            .await?
            .iter()
            .map(|location| CatalogLocation {
                id: location["attributes"]["id"].as_i64().unwrap_or_default(),
                name: location["attributes"]["long"]
                    .as_str()
                    .filter(|name| !name.is_empty())
                    .or(location["attributes"]["short"].as_str())
                    .unwrap_or_default()
                    .to_owned(),
            })
            .collect();
        let mut nests = Vec::new();
        for nest in self.list("/nests").await? {
            let id = nest["attributes"]["id"].as_i64().unwrap_or_default();
            let eggs = self
                .list(&format!("/nests/{id}/eggs"))
                .await?
                .iter()
                .map(|egg| CatalogEgg {
                    id: egg["attributes"]["id"].as_i64().unwrap_or_default(),
                    name: egg["attributes"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                })
                .collect();
            nests.push(CatalogNest {
                id,
                name: nest["attributes"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                eggs,
            });
        }
        Ok(Catalog { locations, nests })
    }

    async fn linked_user(&self, stacklane_id: &str) -> anyhow::Result<Option<Value>> {
        match self
            .application(
                Method::GET,
                &format!("/users/external/stacklane-{stacklane_id}"),
                &[],
                None,
            )
            .await
        {
            Ok(found) => Ok(Some(found["attributes"].clone())),
            Err(error) if format!("{error:#}").contains(" 404 ") => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn change_email(&self, stacklane_id: &str, email: &str) -> anyhow::Result<()> {
        let Some(user) = self.linked_user(stacklane_id).await? else {
            return Ok(());
        };
        let id = user["id"]
            .as_i64()
            .context("Pterodactyl gaf geen gebruiker terug")?;
        if user["email"].as_str() == Some(email) {
            return Ok(());
        }
        self.application(
            Method::PATCH,
            &format!("/users/{id}"),
            &[],
            Some(json!({
                "email": email,
                "username": user["username"],
                "first_name": user["first_name"],
                "last_name": user["last_name"],
            })),
        )
        .await?;
        Ok(())
    }

    pub async fn forget_user(&self, stacklane_id: &str) -> anyhow::Result<()> {
        let Some(user) = self.linked_user(stacklane_id).await? else {
            return Ok(());
        };
        let id = user["id"]
            .as_i64()
            .context("Pterodactyl gaf geen gebruiker terug")?;
        self.application(Method::DELETE, &format!("/users/{id}"), &[], None)
            .await?;
        Ok(())
    }

    async fn user_id(&self, owner: &Owner<'_>) -> anyhow::Result<i64> {
        let external = format!("stacklane-{}", owner.stacklane_id);
        if let Ok(found) = self
            .application(
                Method::GET,
                &format!("/users/external/{external}"),
                &[],
                None,
            )
            .await
            && let Some(id) = found["attributes"]["id"].as_i64()
        {
            return Ok(id);
        }
        let existing = self
            .application(
                Method::GET,
                "/users",
                &[("filter[email]", owner.email.to_owned())],
                None,
            )
            .await?;
        if let Some(id) = matching_user(&existing, owner.email, &external) {
            if !owner.email_verified {
                anyhow::bail!(
                    "er bestaat al een gebruiker met dit e-mailadres in het gamepanel; de klant moet eerst zijn e-mailadres bevestigen"
                );
            }
            return Ok(id);
        }
        let created = self
            .application(
                Method::POST,
                "/users",
                &[],
                Some(json!({
                    "email": owner.email,
                    "username": username_for(owner.email),
                    "first_name": if owner.first_name.is_empty() { "Klant" } else { owner.first_name },
                    "last_name": if owner.last_name.is_empty() { "Stacklane" } else { owner.last_name },
                    "external_id": format!("stacklane-{}", owner.stacklane_id),
                })),
            )
            .await?;
        created["attributes"]["id"]
            .as_i64()
            .context("Pterodactyl gaf geen gebruiker terug")
    }

    pub async fn provision(
        &self,
        service_id: &str,
        name: &str,
        owner: &Owner<'_>,
        config: &ProductConfig,
    ) -> anyhow::Result<Provisioned> {
        let user = self.user_id(owner).await?;
        let egg = self
            .application(
                Method::GET,
                &format!("/nests/{}/eggs/{}", config.nest_id, config.egg_id),
                &[("include", "variables".into())],
                None,
            )
            .await?;
        let attributes = &egg["attributes"];
        let environment: serde_json::Map<String, Value> = attributes["relationships"]["variables"]
            ["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|variable| {
                let key = variable["attributes"]["env_variable"].as_str()?;
                Some((
                    key.to_owned(),
                    variable["attributes"]["default_value"].clone(),
                ))
            })
            .collect();

        let server = self
            .application(
                Method::POST,
                "/servers",
                &[],
                Some(json!({
                    "name": name,
                    "user": user,
                    "egg": config.egg_id,
                    "docker_image": attributes["docker_image"],
                    "startup": attributes["startup"],
                    "environment": environment,
                    "limits": { "memory": config.memory, "swap": 0, "disk": config.disk, "io": 500, "cpu": config.cpu },
                    "feature_limits": { "databases": config.databases, "allocations": config.allocations, "backups": config.backups },
                    "deploy": { "locations": [config.location_id], "dedicated_ip": false, "port_range": [] },
                    "start_on_completion": false,
                    "external_id": service_id,
                })),
            )
            .await?;
        let id = server["attributes"]["id"]
            .as_i64()
            .context("Pterodactyl gaf geen server terug")?;
        let full = self
            .application(
                Method::GET,
                &format!("/servers/{id}"),
                &[("include", "allocations".into())],
                None,
            )
            .await?;
        let allocation = full["attributes"]["relationships"]["allocations"]["data"]
            .as_array()
            .and_then(|allocations| allocations.first())
            .map(|allocation| &allocation["attributes"]);
        Ok(Provisioned {
            external_id: id.to_string(),
            details: json!({
                "identifier": full["attributes"]["identifier"],
                "uuid": full["attributes"]["uuid"],
                "address": allocation.map(|allocation| format!(
                    "{}:{}",
                    allocation["alias"].as_str().filter(|alias| !alias.is_empty()).or(allocation["ip"].as_str()).unwrap_or_default(),
                    allocation["port"]
                )),
                "memory": config.memory,
                "disk": config.disk,
                "cpu": config.cpu,
                "egg": attributes["name"],
            }),
        })
    }

    pub async fn suspend(&self, external_id: &str) -> anyhow::Result<()> {
        self.application(
            Method::POST,
            &format!("/servers/{external_id}/suspend"),
            &[],
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn unsuspend(&self, external_id: &str) -> anyhow::Result<()> {
        self.application(
            Method::POST,
            &format!("/servers/{external_id}/unsuspend"),
            &[],
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn terminate(&self, external_id: &str) -> anyhow::Result<()> {
        self.application(
            Method::DELETE,
            &format!("/servers/{external_id}"),
            &[],
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn install_state(&self, external_id: &str) -> anyhow::Result<Option<String>> {
        let server = self
            .application(Method::GET, &format!("/servers/{external_id}"), &[], None)
            .await?;
        Ok(server["attributes"]["status"].as_str().map(str::to_owned))
    }

    pub async fn resources(&self, identifier: &str) -> anyhow::Result<Value> {
        let body = self
            .client(
                Method::GET,
                &format!("/servers/{identifier}/resources"),
                &[],
                None,
            )
            .await?;
        Ok(body["attributes"].clone())
    }

    pub async fn power(&self, identifier: &str, signal: &str) -> anyhow::Result<()> {
        self.client(
            Method::POST,
            &format!("/servers/{identifier}/power"),
            &[],
            Some(json!({ "signal": signal })),
        )
        .await?;
        Ok(())
    }

    pub async fn websocket(&self, identifier: &str) -> anyhow::Result<Value> {
        let body = self
            .client(
                Method::GET,
                &format!("/servers/{identifier}/websocket"),
                &[],
                None,
            )
            .await?;
        Ok(body["data"].clone())
    }

    pub async fn list_files(&self, identifier: &str, directory: &str) -> anyhow::Result<Value> {
        let body = self
            .client(
                Method::GET,
                &format!("/servers/{identifier}/files/list"),
                &[("directory", directory.to_owned())],
                None,
            )
            .await?;
        Ok(Value::Array(
            body["data"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|entry| entry["attributes"].clone())
                .collect(),
        ))
    }

    pub async fn read_file(&self, identifier: &str, file: &str) -> anyhow::Result<String> {
        let response = self
            .http
            .get(format!(
                "{}/api/client/servers/{identifier}/files/contents",
                self.base_url
            ))
            .bearer_auth(&self.credentials.client_key)
            .query(&[("file", file)])
            .send()
            .await?;
        let status = response.status();
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > MAX_READ {
                anyhow::bail!("dit bestand is te groot om hier te openen");
            }
            bytes.extend_from_slice(&chunk);
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if !status.is_success() {
            return Err(pterodactyl_error(
                status,
                &serde_json::from_str(&text).unwrap_or(Value::Null),
            ));
        }
        Ok(text)
    }

    pub async fn write_file(
        &self,
        identifier: &str,
        file: &str,
        content: String,
    ) -> anyhow::Result<()> {
        let response = self
            .http
            .post(format!(
                "{}/api/client/servers/{identifier}/files/write",
                self.base_url
            ))
            .bearer_auth(&self.credentials.client_key)
            .header("Accept", "application/json")
            .query(&[("file", file)])
            .body(content)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.json().await.unwrap_or(Value::Null);
            return Err(pterodactyl_error(status, &body));
        }
        Ok(())
    }

    pub async fn pull_file(
        &self,
        identifier: &str,
        url: &str,
        directory: &str,
        filename: &str,
    ) -> anyhow::Result<()> {
        self.client(
            Method::POST,
            &format!("/servers/{identifier}/files/pull"),
            &[],
            Some(json!({ "url": url, "directory": directory, "filename": filename, "use_header": false, "foreground": true })),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_files(
        &self,
        identifier: &str,
        root: &str,
        files: &[&str],
    ) -> anyhow::Result<()> {
        self.client(
            Method::POST,
            &format!("/servers/{identifier}/files/delete"),
            &[],
            Some(json!({ "root": root, "files": files })),
        )
        .await?;
        Ok(())
    }

    pub async fn startup_variables(
        &self,
        identifier: &str,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let body = self
            .client(
                Method::GET,
                &format!("/servers/{identifier}/startup"),
                &[],
                None,
            )
            .await?;
        Ok(body["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| {
                let attributes = &entry["attributes"];
                Some((
                    attributes["env_variable"].as_str()?.to_owned(),
                    attributes["server_value"]
                        .as_str()
                        .or_else(|| attributes["default_value"].as_str())
                        .unwrap_or_default()
                        .to_owned(),
                ))
            })
            .collect())
    }

    pub async fn backups(&self, identifier: &str) -> anyhow::Result<Value> {
        let body = self
            .client(
                Method::GET,
                &format!("/servers/{identifier}/backups"),
                &[],
                None,
            )
            .await?;
        Ok(Value::Array(
            body["data"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|entry| entry["attributes"].clone())
                .collect(),
        ))
    }

    pub async fn create_backup(&self, identifier: &str, name: &str) -> anyhow::Result<Value> {
        let body = self
            .client(
                Method::POST,
                &format!("/servers/{identifier}/backups"),
                &[],
                Some(json!({ "name": name })),
            )
            .await?;
        Ok(body["attributes"].clone())
    }

    pub async fn users(&self) -> anyhow::Result<Vec<Value>> {
        self.list("/users").await
    }

    pub async fn servers(&self) -> anyhow::Result<Vec<Value>> {
        let mut servers = Vec::new();
        let mut page = 1;
        loop {
            let body = self
                .application(
                    Method::GET,
                    "/servers",
                    &[
                        ("page", page.to_string()),
                        ("per_page", "100".into()),
                        ("include", "allocations,egg".into()),
                    ],
                    None,
                )
                .await?;
            servers.extend(body["data"].as_array().cloned().unwrap_or_default());
            if page
                >= body["meta"]["pagination"]["total_pages"]
                    .as_i64()
                    .unwrap_or(1)
            {
                return Ok(servers);
            }
            page += 1;
        }
    }

    pub async fn backup_download(&self, identifier: &str, backup: &str) -> anyhow::Result<String> {
        let body = self
            .client(
                Method::GET,
                &format!("/servers/{identifier}/backups/{backup}/download"),
                &[],
                None,
            )
            .await?;
        body["attributes"]["url"]
            .as_str()
            .map(str::to_owned)
            .context("geen downloadlink")
    }
}

fn matching_user(list: &Value, email: &str, external: &str) -> Option<i64> {
    list["data"].as_array()?.iter().find_map(|user| {
        let attributes = &user["attributes"];
        let same_email = attributes["email"]
            .as_str()
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(email));
        let free = attributes["external_id"]
            .as_str()
            .is_none_or(|candidate| candidate == external);
        (same_email && free)
            .then(|| attributes["id"].as_i64())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::username_for;

    #[test]
    fn only_claims_the_panel_user_with_the_exact_email() {
        let list = serde_json::json!({ "data": [
            { "attributes": { "id": 1, "email": "xjan@example.test", "external_id": null } },
            { "attributes": { "id": 2, "email": "jan@example.test", "external_id": "stacklane-other" } },
            { "attributes": { "id": 3, "email": "Jan@Example.test", "external_id": null } },
        ] });
        assert_eq!(
            super::matching_user(&list, "jan@example.test", "stacklane-me"),
            Some(3)
        );
        assert_eq!(
            super::matching_user(&list, "piet@example.test", "stacklane-me"),
            None
        );
        let claimed = serde_json::json!({ "data": [
            { "attributes": { "id": 2, "email": "jan@example.test", "external_id": "stacklane-other" } },
        ] });
        assert_eq!(
            super::matching_user(&claimed, "jan@example.test", "stacklane-me"),
            None
        );
    }

    #[test]
    fn usernames_are_safe_and_unique_enough() {
        let name = username_for("Jan.Jansen+test@voorbeeld.nl");
        assert!(name.starts_with("janjansentest"));
        assert_eq!(name.len(), "janjansentest".len() + 4);
        assert!(username_for("@x.nl").starts_with("klant"));
    }
}
