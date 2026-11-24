use std::time::Duration;

use anyhow::{Context, bail};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::OnceCell;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_nameservers")]
    pub nameservers: Vec<String>,
}

pub fn default_nameservers() -> Vec<String> {
    vec![
        "ns1.openprovider.nl".to_owned(),
        "ns2.openprovider.be".to_owned(),
        "ns3.openprovider.eu".to_owned(),
    ]
}

pub struct Contact<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub company: Option<&'a str>,
    pub email: &'a str,
    pub phone: &'a str,
    pub street: &'a str,
    pub number: &'a str,
    pub zipcode: &'a str,
    pub city: &'a str,
    pub country: &'a str,
}

#[derive(Serialize, Clone)]
pub struct Availability {
    pub domain: String,
    pub available: bool,
    pub premium: bool,
    pub cost: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Registered {
    pub id: u64,
    pub status: String,
    pub expires_on: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Domain {
    pub status: String,
    pub expires_on: Option<String>,
    pub nameservers: Vec<String>,
    pub locked: bool,
}

pub struct Openprovider {
    base_url: String,
    credentials: Credentials,
    settings: Settings,
    http: reqwest::Client,
    token: OnceCell<String>,
}

pub fn split_domain(domain: &str) -> Option<(String, String)> {
    let domain = domain.trim().trim_end_matches('.').to_lowercase();
    let (name, extension) = domain.split_once('.')?;
    let label_ok = |label: &str| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
    };
    (label_ok(name) && extension.split('.').all(label_ok))
        .then(|| (name.to_owned(), extension.to_owned()))
}

pub fn split_phone(phone: &str) -> Option<(String, String, String)> {
    let cleaned: String = phone
        .chars()
        .filter(|character| !matches!(character, ' ' | '-' | '(' | ')'))
        .collect();
    let (country, number) = cleaned.strip_prefix('+')?.split_once('.')?;
    if country.is_empty()
        || country.len() > 3
        || number.len() < 4
        || !country
            .chars()
            .chain(number.chars())
            .all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let (area, subscriber) = number.split_at(2);
    Some((
        format!("+{country}"),
        area.to_owned(),
        subscriber.to_owned(),
    ))
}

fn nameservers(names: &[String]) -> Value {
    Value::Array(
        names
            .iter()
            .enumerate()
            .map(|(index, name)| json!({ "name": name, "seq_nr": index + 1 }))
            .collect(),
    )
}

impl Openprovider {
    pub fn new(
        base_url: &str,
        credentials: Credentials,
        settings: Settings,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let base_url = base_url
            .trim_end_matches('/')
            .trim_end_matches("/v1beta")
            .trim_end_matches("/v1");
        Ok(Self {
            base_url: if base_url.is_empty() {
                "https://api.openprovider.eu".to_owned()
            } else {
                base_url.to_owned()
            },
            credentials,
            settings,
            http,
            token: OnceCell::new(),
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    async fn envelope(response: reqwest::Response) -> anyhow::Result<Value> {
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        let code = body["code"].as_i64().unwrap_or(-1);
        if !status.is_success() || code != 0 {
            let detail = body["desc"]
                .as_str()
                .filter(|desc| !desc.is_empty())
                .unwrap_or("onbekende fout");
            bail!("Openprovider antwoordde {status} ({code}): {detail}");
        }
        Ok(body["data"].clone())
    }

    async fn login(&self) -> anyhow::Result<String> {
        let response = self
            .http
            .post(format!("{}/v1/auth/login", self.base_url))
            .json(&json!({
                "username": self.credentials.username,
                "password": self.credentials.password,
                "ip": "0.0.0.0",
            }))
            .send()
            .await
            .context("Openprovider is niet bereikbaar")?;
        let data = Self::envelope(response)
            .await
            .context("inloggen bij Openprovider mislukt")?;
        data["token"]
            .as_str()
            .map(str::to_owned)
            .context("Openprovider gaf geen token terug")
    }

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> anyhow::Result<Value> {
        let token = self.token.get_or_try_init(|| self.login()).await?;
        let mut request = self
            .http
            .request(method, format!("{}/v1{path}", self.base_url))
            .bearer_auth(token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .context("Openprovider is niet bereikbaar")?;
        Self::envelope(response).await
    }

    pub async fn test(&self) -> anyhow::Result<String> {
        self.call(Method::GET, "/domains?limit=1", None).await?;
        Ok("Openprovider".to_owned())
    }

    pub async fn check(&self, domains: &[(String, String)]) -> anyhow::Result<Vec<Availability>> {
        let list: Vec<Value> = domains
            .iter()
            .map(|(name, extension)| json!({ "name": name, "extension": extension }))
            .collect();
        let data = self
            .call(
                Method::POST,
                "/domains/check",
                Some(json!({ "domains": list, "with_price": true })),
            )
            .await?;
        Ok(data["results"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|result| {
                let price = &result["price"]["reseller"];
                Availability {
                    domain: result["domain"].as_str().unwrap_or_default().to_owned(),
                    available: result["status"] == "free",
                    premium: result["is_premium"].as_bool().unwrap_or(false),
                    cost: price["price"].as_f64(),
                    currency: price["currency"].as_str().map(str::to_owned),
                }
            })
            .collect())
    }

    fn contact_body(contact: &Contact<'_>) -> anyhow::Result<Value> {
        let (country_code, area_code, subscriber) =
            split_phone(contact.phone).context("ongeldig telefoonnummer")?;
        Ok(json!({
            "name": { "first_name": contact.first_name, "last_name": contact.last_name },
            "company_name": contact.company.unwrap_or_default(),
            "email": contact.email,
            "phone": { "country_code": country_code, "area_code": area_code, "subscriber_number": subscriber },
            "address": {
                "street": contact.street,
                "number": contact.number,
                "zipcode": contact.zipcode,
                "city": contact.city,
                "country": contact.country,
            },
        }))
    }

    pub async fn create_handle(&self, contact: &Contact<'_>) -> anyhow::Result<String> {
        let data = self
            .call(
                Method::POST,
                "/customers",
                Some(Self::contact_body(contact)?),
            )
            .await?;
        data["handle"]
            .as_str()
            .map(str::to_owned)
            .context("Openprovider gaf geen handle terug")
    }

    pub async fn update_handle(&self, handle: &str, contact: &Contact<'_>) -> anyhow::Result<()> {
        let mut body = Self::contact_body(contact)?;
        if let Some(object) = body.as_object_mut() {
            object.remove("name");
            object.remove("company_name");
        }
        self.call(Method::PUT, &format!("/customers/{handle}"), Some(body))
            .await?;
        Ok(())
    }

    fn registered(data: &Value) -> anyhow::Result<Registered> {
        Ok(Registered {
            id: data["id"]
                .as_u64()
                .context("Openprovider gaf geen domein-ID terug")?,
            status: data["status"].as_str().unwrap_or_default().to_owned(),
            expires_on: data["expiration_date"]
                .as_str()
                .map(|date| date.chars().take(10).collect()),
        })
    }

    pub async fn register(
        &self,
        domain: &str,
        handle: &str,
        names: &[String],
    ) -> anyhow::Result<Registered> {
        let (name, extension) = split_domain(domain).context("ongeldige domeinnaam")?;
        let data = self
            .call(
                Method::POST,
                "/domains",
                Some(json!({
                    "domain": { "name": name, "extension": extension },
                    "period": 1,
                    "owner_handle": handle,
                    "admin_handle": handle,
                    "tech_handle": handle,
                    "billing_handle": handle,
                    "name_servers": nameservers(names),
                    "autorenew": "off",
                })),
            )
            .await?;
        Self::registered(&data)
    }

    pub async fn transfer(
        &self,
        domain: &str,
        auth_code: &str,
        handle: &str,
        names: &[String],
    ) -> anyhow::Result<Registered> {
        let (name, extension) = split_domain(domain).context("ongeldige domeinnaam")?;
        let data = self
            .call(
                Method::POST,
                "/domains/transfer",
                Some(json!({
                    "domain": { "name": name, "extension": extension },
                    "auth_code": auth_code,
                    "owner_handle": handle,
                    "admin_handle": handle,
                    "tech_handle": handle,
                    "billing_handle": handle,
                    "name_servers": nameservers(names),
                    "autorenew": "off",
                })),
            )
            .await?;
        Self::registered(&data)
    }

    pub async fn domain(&self, id: &str) -> anyhow::Result<Domain> {
        let data = self
            .call(Method::GET, &format!("/domains/{id}"), None)
            .await?;
        Ok(Domain {
            status: data["status"].as_str().unwrap_or_default().to_owned(),
            expires_on: data["expiration_date"]
                .as_str()
                .map(|date| date.chars().take(10).collect()),
            nameservers: data["name_servers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|server| server["name"].as_str().map(str::to_owned))
                .collect(),
            locked: data["is_locked"].as_bool().unwrap_or(false),
        })
    }

    pub async fn set_nameservers(&self, id: &str, names: &[String]) -> anyhow::Result<()> {
        self.call(
            Method::PUT,
            &format!("/domains/{id}"),
            Some(json!({ "name_servers": nameservers(names) })),
        )
        .await?;
        Ok(())
    }

    pub async fn set_locked(&self, id: &str, locked: bool) -> anyhow::Result<()> {
        self.call(
            Method::PUT,
            &format!("/domains/{id}"),
            Some(json!({ "is_locked": locked })),
        )
        .await?;
        Ok(())
    }

    pub async fn renew(&self, id: &str) -> anyhow::Result<()> {
        self.call(
            Method::POST,
            &format!("/domains/{id}/renew"),
            Some(json!({ "period": 1 })),
        )
        .await?;
        Ok(())
    }

    pub async fn auth_code(&self, id: &str) -> anyhow::Result<String> {
        let data = self
            .call(Method::GET, &format!("/domains/{id}/authcode"), None)
            .await?;
        data["auth_code"]
            .as_str()
            .filter(|code| !code.is_empty())
            .map(str::to_owned)
            .context("Openprovider gaf geen autorisatiecode terug")
    }
}

#[cfg(test)]
mod tests {
    use super::{split_domain, split_phone};

    #[test]
    fn splits_domains_and_phones() {
        assert_eq!(
            split_domain("Voorbeeld.NL"),
            Some(("voorbeeld".to_owned(), "nl".to_owned()))
        );
        assert_eq!(
            split_domain("shop.co.uk."),
            Some(("shop".to_owned(), "co.uk".to_owned()))
        );
        assert_eq!(
            split_domain("xn--bcher-kva.de"),
            Some(("xn--bcher-kva".to_owned(), "de".to_owned()))
        );
        assert_eq!(split_domain("-fout.nl"), None);
        assert_eq!(split_domain("geen punt"), None);
        assert_eq!(split_domain("nl"), None);
        assert_eq!(
            split_phone("+31.612345678"),
            Some(("+31".to_owned(), "61".to_owned(), "2345678".to_owned()))
        );
        assert_eq!(
            split_phone("+32.4 70 12 34 56"),
            Some(("+32".to_owned(), "47".to_owned(), "0123456".to_owned()))
        );
        assert_eq!(split_phone("0612345678"), None);
        assert_eq!(split_phone("+31.12"), None);
    }
}
