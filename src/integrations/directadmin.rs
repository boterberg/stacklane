use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::webhosting::NewAccount;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub username: String,
    pub login_key: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub ip: String,
}

pub struct DirectAdmin {
    base_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
}

pub fn parse_reply(body: &str) -> BTreeMap<String, String> {
    if let Ok(Value::Object(object)) = serde_json::from_str::<Value>(body) {
        return object
            .into_iter()
            .map(|(key, value)| {
                let text = match value {
                    Value::String(text) => text,
                    other => other.to_string(),
                };
                (key, text)
            })
            .collect();
    }
    url::form_urlencoded::parse(body.trim().as_bytes())
        .into_owned()
        .collect()
}

pub fn parse_list(body: &str) -> Vec<String> {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        let items: Vec<&Value> = match &value {
            Value::Array(items) => items.iter().collect(),
            Value::Object(object) => object.values().collect(),
            _ => Vec::new(),
        };
        return items
            .into_iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
    }
    url::form_urlencoded::parse(body.trim().as_bytes())
        .filter(|(key, _)| key == "list[]")
        .map(|(_, value)| value.into_owned())
        .collect()
}

fn failed(reply: &BTreeMap<String, String>) -> Option<String> {
    let flagged = reply
        .get("error")
        .is_some_and(|error| error != "0" && error != "false");
    flagged.then(|| {
        [reply.get("text"), reply.get("details"), reply.get("result")]
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(": ")
    })
}

pub fn find_url(text: &str) -> Option<String> {
    let start = text.find("https://").or_else(|| text.find("http://"))?;
    let rest = &text[start..];
    let end = rest
        .find(|character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | '<' | '\\')
        })
        .unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

impl DirectAdmin {
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
            base_url: base_url.trim_end_matches('/').to_owned(),
            credentials,
            settings,
            http,
        })
    }

    pub fn panel_url(&self) -> String {
        self.base_url.clone()
    }

    async fn send(
        &self,
        method: reqwest::Method,
        command: &str,
        form: &[(&str, &str)],
        as_user: Option<&str>,
    ) -> anyhow::Result<String> {
        let login = match as_user {
            Some(user) => format!("{}|{user}", self.credentials.username),
            None => self.credentials.username.clone(),
        };
        let mut request = self
            .http
            .request(method.clone(), format!("{}/{command}", self.base_url))
            .basic_auth(login, Some(&self.credentials.login_key))
            .query(&[("json", "yes")]);
        request = if method == reqwest::Method::GET {
            request.query(form)
        } else {
            request.form(form)
        };
        let response = request
            .send()
            .await
            .context("DirectAdmin is niet bereikbaar")?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            bail!("DirectAdmin weigert de inlogsleutel");
        }
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("DirectAdmin antwoordde {status}");
        }
        Ok(body)
    }

    async fn post(
        &self,
        command: &str,
        form: &[(&str, &str)],
    ) -> anyhow::Result<BTreeMap<String, String>> {
        let reply = parse_reply(
            &self
                .send(reqwest::Method::POST, command, form, None)
                .await?,
        );
        if let Some(message) = failed(&reply) {
            bail!("DirectAdmin antwoordde: {message}");
        }
        Ok(reply)
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        let packages = self.packages().await?;
        Ok(format!("DirectAdmin, {} pakketten", packages.len()))
    }

    async fn packages(&self) -> anyhow::Result<Vec<String>> {
        Ok(parse_list(
            &self
                .send(reqwest::Method::GET, "CMD_API_PACKAGES_USER", &[], None)
                .await?,
        ))
    }

    pub async fn plans(&self) -> anyhow::Result<Value> {
        let plans: Vec<Value> = self
            .packages()
            .await?
            .into_iter()
            .map(|name| json!({ "id": name, "name": name }))
            .collect();
        Ok(json!({ "plans": plans }))
    }

    pub async fn create(&self, account: &NewAccount<'_>) -> anyhow::Result<()> {
        self.post(
            "CMD_API_ACCOUNT_USER",
            &[
                ("action", "create"),
                ("add", "Submit"),
                ("username", account.username),
                ("email", account.email),
                ("passwd", account.password),
                ("passwd2", account.password),
                ("domain", account.domain),
                ("package", account.plan),
                ("ip", &self.settings.ip),
                ("notify", "no"),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn suspend(&self, username: &str) -> anyhow::Result<()> {
        self.post(
            "CMD_API_SELECT_USERS",
            &[
                ("location", "CMD_SELECT_USERS"),
                ("suspend", "Suspend"),
                ("select0", username),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn unsuspend(&self, username: &str) -> anyhow::Result<()> {
        self.post(
            "CMD_API_SELECT_USERS",
            &[
                ("location", "CMD_SELECT_USERS"),
                ("suspend", "Unsuspend"),
                ("select0", username),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn terminate(&self, username: &str) -> anyhow::Result<()> {
        self.post(
            "CMD_API_SELECT_USERS",
            &[
                ("confirmed", "Confirm"),
                ("delete", "yes"),
                ("select0", username),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn login_url(&self, username: &str) -> anyhow::Result<String> {
        let body = self
            .send(
                reqwest::Method::POST,
                "CMD_API_LOGIN_KEYS",
                &[
                    ("action", "create"),
                    ("type", "one_time_url"),
                    ("keyname", "stacklane"),
                    ("expiry", "1h"),
                    ("max_uses", "1"),
                    ("clear_key", "yes"),
                ],
                Some(username),
            )
            .await?;
        let reply = parse_reply(&body);
        if let Some(message) = failed(&reply) {
            bail!("DirectAdmin antwoordde: {message}");
        }
        find_url(&body)
            .or_else(|| {
                ["/api/login/url", "/CMD_LOGIN_URL"]
                    .iter()
                    .find_map(|path| {
                        let start = body.find(path)?;
                        find_url(&format!("{}{}", self.base_url, &body[start..]))
                    })
            })
            .context("DirectAdmin gaf geen inloglink terug")
    }
}

#[cfg(test)]
mod tests {
    use super::{failed, find_url, parse_list, parse_reply};

    #[test]
    fn reads_legacy_and_json_replies() {
        let reply = parse_reply("error=1&text=Unable%20to%20create&details=Username%20taken");
        assert_eq!(
            failed(&reply).as_deref(),
            Some("Unable to create: Username taken")
        );
        let reply = parse_reply(r#"{"error":"0","text":"User created"}"#);
        assert_eq!(failed(&reply), None);
        let reply = parse_reply(r#"{"success":"User created","result":""}"#);
        assert_eq!(failed(&reply), None);
        assert_eq!(parse_list("list[]=basic&list[]=pro"), ["basic", "pro"]);
        assert_eq!(parse_list(r#"["basic","pro"]"#), ["basic", "pro"]);
        assert_eq!(parse_list(r#"{"0":"basic","1":"pro"}"#), ["basic", "pro"]);
        assert_eq!(
            find_url(r#"{"details":"https://da.voorbeeld.nl:2222/api/login/url?key=abc"}"#)
                .as_deref(),
            Some("https://da.voorbeeld.nl:2222/api/login/url?key=abc")
        );
    }
}
