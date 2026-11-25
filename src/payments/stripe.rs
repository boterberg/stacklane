use anyhow::Context;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::Value;
use sha2::Sha256;

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    secret_key: String,
}

pub struct Checkout<'a> {
    pub customer: &'a str,
    pub amount_cents: i64,
    pub currency: &'a str,
    pub description: &'a str,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    pub payment_id: &'a str,
    pub save_card: bool,
}

impl Client {
    pub fn new(secret_key: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
            base_url: std::env::var("STRIPE_API_URL")
                .unwrap_or_else(|_| "https://api.stripe.com/v1".into()),
            secret_key,
        }
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> anyhow::Result<Value> {
        let response = request
            .basic_auth(&self.secret_key, Some(""))
            .send()
            .await
            .context("Stripe is niet bereikbaar")?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or_default();
        if !status.is_success() {
            let message = body
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("onbekende fout");
            anyhow::bail!("Stripe gaf een fout ({status}): {message}");
        }
        Ok(body)
    }

    pub async fn check(&self) -> anyhow::Result<()> {
        self.send(self.http.get(format!("{}/balance", self.base_url)))
            .await
            .map(|_| ())
    }

    pub async fn create_customer(
        &self,
        name: &str,
        email: &str,
        reference: &str,
    ) -> anyhow::Result<String> {
        let body = self
            .send(
                self.http
                    .post(format!("{}/customers", self.base_url))
                    .form(&[
                        ("name", name),
                        ("email", email),
                        ("metadata[stacklane_user]", reference),
                    ]),
            )
            .await?;
        body.get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .context("Stripe gaf geen klantnummer terug")
    }

    pub async fn create_checkout(
        &self,
        checkout: &Checkout<'_>,
    ) -> anyhow::Result<(String, String)> {
        let amount = checkout.amount_cents.to_string();
        let currency = checkout.currency.to_lowercase();
        let mut form = vec![
            ("mode", "payment"),
            ("customer", checkout.customer),
            ("success_url", checkout.success_url),
            ("cancel_url", checkout.cancel_url),
            ("client_reference_id", checkout.payment_id),
            ("line_items[0][quantity]", "1"),
            ("line_items[0][price_data][currency]", currency.as_str()),
            ("line_items[0][price_data][unit_amount]", amount.as_str()),
            (
                "line_items[0][price_data][product_data][name]",
                checkout.description,
            ),
            ("metadata[payment_id]", checkout.payment_id),
            (
                "payment_intent_data[metadata][payment_id]",
                checkout.payment_id,
            ),
        ];
        if checkout.save_card {
            form.push(("payment_intent_data[setup_future_usage]", "off_session"));
        }
        let body = self
            .send(
                self.http
                    .post(format!("{}/checkout/sessions", self.base_url))
                    .form(&form),
            )
            .await?;
        let id = body
            .get("id")
            .and_then(Value::as_str)
            .context("Stripe gaf geen sessie terug")?;
        let url = body
            .get("url")
            .and_then(Value::as_str)
            .context("Stripe gaf geen betaallink terug")?;
        Ok((id.to_owned(), url.to_owned()))
    }

    pub async fn payment_intent(&self, id: &str) -> anyhow::Result<Value> {
        self.send(
            self.http
                .get(format!("{}/payment_intents/{id}", self.base_url))
                .query(&[("expand[]", "payment_method")]),
        )
        .await
    }

    pub async fn charge_off_session(
        &self,
        customer: &str,
        payment_method: &str,
        amount_cents: i64,
        currency: &str,
        description: &str,
        payment_id: &str,
    ) -> anyhow::Result<Value> {
        let amount = amount_cents.to_string();
        let currency = currency.to_lowercase();
        self.send(
            self.http
                .post(format!("{}/payment_intents", self.base_url))
                .form(&[
                    ("amount", amount.as_str()),
                    ("currency", currency.as_str()),
                    ("customer", customer),
                    ("payment_method", payment_method),
                    ("description", description),
                    ("off_session", "true"),
                    ("confirm", "true"),
                    ("metadata[payment_id]", payment_id),
                ]),
        )
        .await
    }
}

pub fn describe_card(payment_method: &Value) -> String {
    match (
        payment_method
            .pointer("/card/brand")
            .and_then(Value::as_str),
        payment_method
            .pointer("/card/last4")
            .and_then(Value::as_str),
    ) {
        (Some(brand), Some(last4)) => format!("{brand} ···· {last4}"),
        _ => payment_method
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("stripe")
            .to_owned(),
    }
}

pub fn map_intent_status(status: &str) -> &'static str {
    match status {
        "succeeded" => "paid",
        "canceled" => "cancelled",
        "requires_payment_method" => "failed",
        _ => "open",
    }
}

pub fn verify_signature(payload: &[u8], header: &str, secret: &str, now: i64) -> bool {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.extend(hex::decode(value).ok()),
            _ => {}
        }
    }
    let Some(timestamp) = timestamp else {
        return false;
    };
    if (now - timestamp).abs() > 300 {
        return false;
    }
    signatures.iter().any(|signature| {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        mac.verify_slice(signature).is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(payload: &[u8], secret: &str, timestamp: i64) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{timestamp}.").as_bytes());
        mac.update(payload);
        format!(
            "t={timestamp},v1={}",
            hex::encode(mac.finalize().into_bytes())
        )
    }

    #[test]
    fn accepts_only_valid_recent_signatures() {
        let payload = br#"{"type":"checkout.session.completed"}"#;
        let header = sign(payload, "whsec_test", 1_000_000);
        assert!(verify_signature(payload, &header, "whsec_test", 1_000_100));
        assert!(!verify_signature(
            payload,
            &header,
            "whsec_other",
            1_000_100
        ));
        assert!(!verify_signature(b"{}", &header, "whsec_test", 1_000_100));
        assert!(!verify_signature(payload, &header, "whsec_test", 1_000_900));
    }

    #[test]
    fn describes_cards() {
        let method =
            serde_json::json!({ "type": "card", "card": { "brand": "visa", "last4": "4242" } });
        assert_eq!(describe_card(&method), "visa ···· 4242");
        assert_eq!(map_intent_status("succeeded"), "paid");
    }
}
