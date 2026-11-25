use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::{Context, bail};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_server")]
    pub server_id: String,
}

fn default_server() -> String {
    "localhost".to_owned()
}

pub struct PowerDns {
    base_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
}

pub fn reverse_name(ip: Ipv4Addr) -> String {
    let [a, b, c, d] = ip.octets();
    format!("{d}.{c}.{b}.{a}.in-addr.arpa.")
}

pub fn valid_hostname(name: &str) -> bool {
    let name = name.strip_suffix('.').unwrap_or(name);
    let labels: Vec<&str> = name.split('.').collect();
    name.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
        && labels
            .last()
            .is_some_and(|top| top.chars().any(|character| character.is_ascii_alphabetic()))
}

fn zone_for<'a>(zones: &'a [String], name: &str) -> Option<&'a str> {
    zones
        .iter()
        .filter(|zone| zone.ends_with(".in-addr.arpa."))
        .filter(|zone| name == zone.as_str() || name.ends_with(&format!(".{zone}")))
        .max_by_key(|zone| zone.len())
        .map(String::as_str)
}

impl PowerDns {
    pub fn new(
        base_url: &str,
        credentials: Credentials,
        settings: Settings,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            credentials,
            settings,
            http,
        })
    }

    fn server_path(&self) -> String {
        format!("/api/v1/servers/{}", self.settings.server_id)
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base_url))
            .header("X-API-Key", &self.credentials.api_key);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .context("PowerDNS is niet bereikbaar")?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            bail!(
                "PowerDNS antwoordde {status}: {}",
                body["error"].as_str().unwrap_or("onbekende fout")
            );
        }
        Ok(body)
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        let server = self.request(Method::GET, &self.server_path(), None).await?;
        let zones = self.reverse_zones().await?;
        Ok(format!(
            "PowerDNS {}, {} reverse-zones",
            server["version"].as_str().unwrap_or("?"),
            zones.len()
        ))
    }

    async fn reverse_zones(&self) -> anyhow::Result<Vec<String>> {
        let zones = self
            .request(Method::GET, &format!("{}/zones", self.server_path()), None)
            .await?;
        Ok(zones
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|zone| zone["name"].as_str())
            .filter(|name| name.ends_with(".in-addr.arpa."))
            .map(str::to_owned)
            .collect())
    }

    pub async fn zone_of(&self, ip: Ipv4Addr) -> anyhow::Result<Option<String>> {
        let zones = self.reverse_zones().await?;
        Ok(zone_for(&zones, &reverse_name(ip)).map(str::to_owned))
    }

    pub async fn ptr(&self, zone: &str, ip: Ipv4Addr) -> anyhow::Result<Option<String>> {
        let name = reverse_name(ip);
        let found = self
            .request(
                Method::GET,
                &format!(
                    "{}/zones/{zone}?rrset_name={name}&rrset_type=PTR",
                    self.server_path()
                ),
                None,
            )
            .await?;
        Ok(found["rrsets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|rrset| rrset["name"] == name.as_str() && rrset["type"] == "PTR")
            .flat_map(|rrset| rrset["records"].as_array().into_iter().flatten())
            .find_map(|record| record["content"].as_str())
            .map(|content| content.trim_end_matches('.').to_owned()))
    }

    pub async fn set_ptr(
        &self,
        zone: &str,
        ip: Ipv4Addr,
        hostname: Option<&str>,
    ) -> anyhow::Result<()> {
        let name = reverse_name(ip);
        let rrset = match hostname {
            Some(hostname) => json!({
                "name": name,
                "type": "PTR",
                "ttl": 3600,
                "changetype": "REPLACE",
                "records": [{ "content": format!("{}.", hostname.trim_end_matches('.')), "disabled": false }],
            }),
            None => json!({ "name": name, "type": "PTR", "changetype": "DELETE" }),
        };
        self.request(
            Method::PATCH,
            &format!("{}/zones/{zone}", self.server_path()),
            Some(json!({ "rrsets": [rrset] })),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_reverse_names_and_picks_the_closest_zone() {
        let ip = Ipv4Addr::new(192, 0, 2, 15);
        assert_eq!(reverse_name(ip), "15.2.0.192.in-addr.arpa.");
        let zones = vec![
            "0.192.in-addr.arpa.".to_owned(),
            "2.0.192.in-addr.arpa.".to_owned(),
            "example.com.".to_owned(),
            "12.0.192.in-addr.arpa.".to_owned(),
        ];
        assert_eq!(
            zone_for(&zones, &reverse_name(ip)),
            Some("2.0.192.in-addr.arpa.")
        );
        assert_eq!(
            zone_for(&zones, &reverse_name(Ipv4Addr::new(192, 0, 9, 1))),
            Some("0.192.in-addr.arpa.")
        );
        assert_eq!(
            zone_for(&zones, &reverse_name(Ipv4Addr::new(10, 0, 0, 1))),
            None
        );
    }

    #[test]
    fn accepts_only_plain_hostnames() {
        assert!(valid_hostname("mail.example.com"));
        assert!(valid_hostname("mail.example.com."));
        assert!(!valid_hostname("localhost"));
        assert!(!valid_hostname("-bad.example.com"));
        assert!(!valid_hostname("bad_name.example.com"));
        assert!(!valid_hostname("example.123"));
        assert!(!valid_hostname("a..example.com"));
        assert!(!valid_hostname(&format!("{}.com", "a".repeat(64))));
    }

    async fn record(
        axum::extract::State(calls): axum::extract::State<
            std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        >,
        request: axum::extract::Request,
    ) -> axum::Json<Value> {
        let line = format!("{} {}", request.method(), request.uri());
        let key = request
            .headers()
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = axum::body::to_bytes(request.into_body(), 1 << 16)
            .await
            .unwrap_or_default();
        calls.lock().unwrap().push(
            format!(
                "{line} {} {}",
                key.unwrap_or_default(),
                String::from_utf8_lossy(&body)
            )
            .trim_end()
            .to_owned(),
        );
        axum::Json(if line.ends_with("/zones") {
            json!([{ "name": "2.0.192.in-addr.arpa." }, { "name": "example.com." }])
        } else if line.contains("rrset_name=") {
            json!({ "rrsets": [{ "name": "15.2.0.192.in-addr.arpa.", "type": "PTR", "records": [{ "content": "mail.example.com." }] }] })
        } else {
            json!({ "version": "4.9.0" })
        })
    }

    #[tokio::test]
    async fn reads_and_writes_ptr_records() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new()
            .fallback(record)
            .with_state(calls.clone());
        tokio::spawn(async move { axum::serve(listener, app).await });
        let client = PowerDns::new(
            &base,
            Credentials {
                api_key: "geheim".into(),
            },
            Settings {
                server_id: "localhost".into(),
            },
        )
        .unwrap();
        let ip = Ipv4Addr::new(192, 0, 2, 15);

        assert!(client.test().await.unwrap().contains("1 reverse-zones"));
        let zone = client.zone_of(ip).await.unwrap().unwrap();
        assert_eq!(
            client.ptr(&zone, ip).await.unwrap().as_deref(),
            Some("mail.example.com")
        );
        client
            .set_ptr(&zone, ip, Some("web.example.com"))
            .await
            .unwrap();
        client.set_ptr(&zone, ip, None).await.unwrap();

        let calls = calls.lock().unwrap().clone();
        assert!(calls.iter().all(|call| call.contains(" geheim")));
        let patches: Vec<&String> = calls
            .iter()
            .filter(|call| {
                call.starts_with("PATCH /api/v1/servers/localhost/zones/2.0.192.in-addr.arpa.")
            })
            .collect();
        assert_eq!(patches.len(), 2);
        assert!(
            patches[0].contains("\"content\":\"web.example.com.\"")
                && patches[0].contains("REPLACE")
        );
        assert!(patches[1].contains("\"changetype\":\"DELETE\""));
    }
}
