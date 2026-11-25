use std::time::Duration;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::webhosting::NewAccount;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub username: String,
    pub api_token: String,
}

pub struct Cpanel {
    base_url: String,
    credentials: Credentials,
    http: reqwest::Client,
}

impl Cpanel {
    pub fn new(base_url: &str, credentials: Credentials) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(120))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url
                .trim_end_matches('/')
                .trim_end_matches("/json-api")
                .to_owned(),
            credentials,
            http,
        })
    }

    pub fn panel_url(&self) -> String {
        let host = self
            .base_url
            .rsplit_once(':')
            .map_or(self.base_url.as_str(), |(host, port)| {
                if port.chars().all(|c| c.is_ascii_digit()) {
                    host
                } else {
                    self.base_url.as_str()
                }
            });
        format!("{host}:2083")
    }

    async fn call(&self, function: &str, params: &[(&str, &str)]) -> anyhow::Result<Value> {
        let response = self
            .http
            .post(format!("{}/json-api/{function}", self.base_url))
            .query(&[("api.version", "1")])
            .form(params)
            .header(
                "Authorization",
                format!(
                    "whm {}:{}",
                    self.credentials.username, self.credentials.api_token
                ),
            )
            .send()
            .await
            .context("WHM is niet bereikbaar")?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            bail!("WHM weigert het API-token");
        }
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if body["metadata"]["result"] != 1 {
            bail!(
                "WHM antwoordde: {}",
                body["metadata"]["reason"]
                    .as_str()
                    .unwrap_or("onbekende fout")
            );
        }
        Ok(body["data"].clone())
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        let data = self.call("version", &[]).await?;
        Ok(format!(
            "cPanel & WHM {}",
            data["version"].as_str().unwrap_or("?")
        ))
    }

    pub async fn plans(&self) -> anyhow::Result<Value> {
        let data = self.call("listpkgs", &[]).await?;
        let plans: Vec<Value> = data["pkg"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|package| package["name"].as_str())
            .map(|name| json!({ "id": name, "name": name }))
            .collect();
        Ok(json!({ "plans": plans }))
    }

    pub async fn create(&self, account: &NewAccount<'_>) -> anyhow::Result<()> {
        self.call(
            "createacct",
            &[
                ("username", account.username),
                ("domain", account.domain),
                ("plan", account.plan),
                ("contactemail", account.email),
                ("password", account.password),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn suspend(&self, username: &str) -> anyhow::Result<()> {
        self.call(
            "suspendacct",
            &[
                ("user", username),
                ("reason", "Opgeschort vanuit Stacklane"),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn unsuspend(&self, username: &str) -> anyhow::Result<()> {
        self.call("unsuspendacct", &[("user", username)]).await?;
        Ok(())
    }

    pub async fn terminate(&self, username: &str) -> anyhow::Result<()> {
        self.call("removeacct", &[("username", username)]).await?;
        Ok(())
    }

    pub async fn login_url(&self, username: &str) -> anyhow::Result<String> {
        let data = self
            .call(
                "create_user_session",
                &[("user", username), ("service", "cpaneld")],
            )
            .await?;
        data["url"]
            .as_str()
            .map(str::to_owned)
            .context("WHM gaf geen inloglink terug")
    }
}

#[cfg(test)]
mod tests {
    use super::{Cpanel, Credentials};

    #[test]
    fn derives_the_cpanel_address() {
        let credentials = Credentials {
            username: "root".to_owned(),
            api_token: "x".to_owned(),
        };
        let whm = Cpanel::new("https://server.voorbeeld.nl:2087/", credentials.clone()).unwrap();
        assert_eq!(whm.panel_url(), "https://server.voorbeeld.nl:2083");
        let whm = Cpanel::new("https://whm.voorbeeld.nl", credentials).unwrap();
        assert_eq!(whm.panel_url(), "https://whm.voorbeeld.nl:2083");
    }
}
