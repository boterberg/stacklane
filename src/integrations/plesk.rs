use std::time::Duration;

use anyhow::{Context, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::webhosting::NewAccount;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub ip: String,
}

pub struct Plesk {
    base_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
}

pub fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn xml_value<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim())
}

pub fn plan_names(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.ends_with(':'))
        .map(str::to_owned)
        .collect()
}

impl Plesk {
    pub fn new(
        base_url: &str,
        credentials: Credentials,
        settings: Settings,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url
                .trim_end_matches('/')
                .trim_end_matches("/api/v2")
                .to_owned(),
            credentials,
            settings,
            http,
        })
    }

    pub fn panel_url(&self) -> String {
        self.base_url.clone()
    }

    async fn rest(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<(reqwest::StatusCode, Value)> {
        let mut request = self
            .http
            .request(method, format!("{}/api/v2{path}", self.base_url))
            .header("X-API-Key", &self.credentials.api_key)
            .header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.context("Plesk is niet bereikbaar")?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            bail!("Plesk weigert de API-sleutel");
        }
        Ok((status, response.json().await.unwrap_or(Value::Null)))
    }

    async fn cli(&self, utility: &str, params: &[&str]) -> anyhow::Result<String> {
        let (status, body) = self
            .rest(
                reqwest::Method::POST,
                &format!("/cli/{utility}/call"),
                Some(json!({ "params": params })),
            )
            .await?;
        if !status.is_success() || body["code"].as_i64().unwrap_or(1) != 0 {
            let detail = body["stderr"]
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .or_else(|| body["message"].as_str())
                .unwrap_or("onbekende fout");
            bail!("Plesk antwoordde: {}", detail.trim());
        }
        Ok(body["stdout"].as_str().unwrap_or_default().to_owned())
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        let (status, body) = self.rest(reqwest::Method::GET, "/server", None).await?;
        if !status.is_success() {
            bail!("Plesk antwoordde {status}");
        }
        Ok(format!(
            "Plesk {}",
            body["panel_version"].as_str().unwrap_or("?")
        ))
    }

    pub async fn plans(&self) -> anyhow::Result<Value> {
        let stdout = self.cli("service_plan", &["--list"]).await?;
        let plans: Vec<Value> = plan_names(&stdout)
            .into_iter()
            .map(|name| json!({ "id": name, "name": name }))
            .collect();
        Ok(json!({ "plans": plans }))
    }

    pub async fn create(&self, account: &NewAccount<'_>) -> anyhow::Result<()> {
        let (status, body) = self
            .rest(
                reqwest::Method::POST,
                "/clients",
                Some(json!({
                    "name": account.owner,
                    "login": account.username,
                    "password": account.password,
                    "email": account.email,
                    "type": "customer",
                })),
            )
            .await?;
        if !status.is_success() {
            bail!(
                "Plesk kon de klant niet aanmaken: {}",
                body["message"].as_str().unwrap_or("onbekende fout")
            );
        }
        self.cli(
            "subscription",
            &[
                "--create",
                account.domain,
                "-owner",
                account.username,
                "-service-plan",
                account.plan,
                "-ip",
                &self.settings.ip,
                "-login",
                account.username,
                "-passwd",
                account.password,
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn suspend(&self, domain: &str) -> anyhow::Result<()> {
        self.cli("subscription", &["--webspace-off", domain])
            .await?;
        Ok(())
    }

    pub async fn unsuspend(&self, domain: &str) -> anyhow::Result<()> {
        self.cli("subscription", &["--webspace-on", domain]).await?;
        Ok(())
    }

    pub async fn terminate(&self, domain: &str, username: &str) -> anyhow::Result<()> {
        self.cli("subscription", &["--remove", domain]).await?;
        let _ = self.cli("customer", &["--remove", username]).await;
        Ok(())
    }

    pub async fn login_url(&self, username: &str, user_ip: &str) -> anyhow::Result<String> {
        let packet = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><packet><server><create_session><login>{}</login><data><user_ip>{}</user_ip><source_server></source_server></data></create_session></server></packet>",
            escape_xml(username),
            STANDARD.encode(user_ip)
        );
        let response = self
            .http
            .post(format!("{}/enterprise/control/agent.php", self.base_url))
            .header("KEY", &self.credentials.api_key)
            .header("Content-Type", "text/xml")
            .body(packet)
            .send()
            .await
            .context("Plesk is niet bereikbaar")?;
        let xml = response.text().await.unwrap_or_default();
        if xml_value(&xml, "status") != Some("ok") {
            bail!(
                "Plesk antwoordde: {}",
                xml_value(&xml, "errtext").unwrap_or("onbekende fout")
            );
        }
        let session = xml_value(&xml, "id").context("Plesk gaf geen sessie terug")?;
        Ok(format!(
            "{}/enterprise/rsession_init.php?PHPSESSID={session}",
            self.base_url
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_xml, plan_names, xml_value};

    #[test]
    fn reads_plesk_output() {
        assert_eq!(escape_xml("a<b&\"c\""), "a&lt;b&amp;&quot;c&quot;");
        let xml = "<packet><server><create_session><result><status>ok</status><id>abc123</id></result></create_session></server></packet>";
        assert_eq!(xml_value(xml, "status"), Some("ok"));
        assert_eq!(xml_value(xml, "id"), Some("abc123"));
        assert_eq!(xml_value(xml, "errtext"), None);
        assert_eq!(
            plan_names("Default Domain\nUnlimited\n\n"),
            ["Default Domain", "Unlimited"]
        );
    }
}
