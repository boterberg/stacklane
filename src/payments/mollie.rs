use anyhow::Context;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

#[derive(Deserialize)]
pub struct Payment {
    pub id: String,
    pub status: String,
    pub method: Option<String>,
    #[serde(rename = "customerId")]
    pub customer_id: Option<String>,
    #[serde(rename = "mandateId")]
    pub mandate_id: Option<String>,
    #[serde(default)]
    pub details: Value,
    #[serde(default)]
    pub metadata: Value,
    #[serde(rename = "_links", default)]
    links: Value,
}

impl Payment {
    pub fn checkout_url(&self) -> Option<String> {
        self.links
            .pointer("/checkout/href")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    pub fn describe_method(&self) -> String {
        let method = self.method.as_deref().unwrap_or("mollie");
        let account = ["consumerAccount", "cardNumber"]
            .iter()
            .find_map(|key| self.details.get(key).and_then(Value::as_str))
            .map(|value| {
                value
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
            });
        match account {
            Some(last) => format!("{method} ···· {last}"),
            None => method.to_owned(),
        }
    }
}

pub enum Sequence<'a> {
    OneOff,
    First { customer: &'a str },
    Recurring { customer: &'a str, mandate: &'a str },
}

pub struct NewPayment<'a> {
    pub amount_cents: i64,
    pub currency: &'a str,
    pub description: &'a str,
    pub redirect_url: &'a str,
    pub webhook_url: &'a str,
    pub payment_id: &'a str,
    pub sequence: Sequence<'a>,
}

pub fn amount(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
            base_url: std::env::var("MOLLIE_API_URL")
                .unwrap_or_else(|_| "https://api.mollie.com/v2".into()),
            api_key,
        }
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> anyhow::Result<Value> {
        let response = request
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("Mollie is niet bereikbaar")?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or_default();
        if !status.is_success() {
            let detail = body
                .get("detail")
                .and_then(Value::as_str)
                .unwrap_or("onbekende fout");
            anyhow::bail!("Mollie gaf een fout ({status}): {detail}");
        }
        Ok(body)
    }

    pub async fn check(&self) -> anyhow::Result<()> {
        self.send(self.http.get(format!("{}/methods", self.base_url)))
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
                    .json(&json!({ "name": name, "email": email, "metadata": { "stacklane_user": reference } })),
            )
            .await?;
        body.get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .context("Mollie gaf geen klantnummer terug")
    }

    pub async fn create_payment(&self, payment: &NewPayment<'_>) -> anyhow::Result<Payment> {
        let mut body = json!({
            "amount": { "currency": payment.currency, "value": amount(payment.amount_cents) },
            "description": payment.description,
            "redirectUrl": payment.redirect_url,
            "webhookUrl": payment.webhook_url,
            "metadata": { "payment_id": payment.payment_id },
        });
        match payment.sequence {
            Sequence::OneOff => {}
            Sequence::First { customer } => {
                body["sequenceType"] = json!("first");
                body["customerId"] = json!(customer);
            }
            Sequence::Recurring { customer, mandate } => {
                body["sequenceType"] = json!("recurring");
                body["customerId"] = json!(customer);
                body["mandateId"] = json!(mandate);
                body.as_object_mut()
                    .map(|object| object.remove("redirectUrl"));
            }
        }
        let value = self
            .send(
                self.http
                    .post(format!("{}/payments", self.base_url))
                    .header("Idempotency-Key", payment.payment_id)
                    .json(&body),
            )
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn payment(&self, id: &str) -> anyhow::Result<Payment> {
        let value = self
            .send(self.http.get(format!("{}/payments/{id}", self.base_url)))
            .await?;
        Ok(serde_json::from_value(value)?)
    }
}

pub fn map_status(status: &str) -> &'static str {
    match status {
        "paid" => "paid",
        "failed" => "failed",
        "canceled" => "cancelled",
        "expired" => "expired",
        _ => "open",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_amounts_for_mollie() {
        assert_eq!(amount(650), "6.50");
        assert_eq!(amount(12_000), "120.00");
        assert_eq!(amount(5), "0.05");
    }

    #[test]
    fn describes_the_payment_method() {
        let payment: Payment = serde_json::from_value(json!({
            "id": "tr_1", "status": "paid", "method": "ideal",
            "details": { "consumerAccount": "NL55INGB0000000000" }
        }))
        .unwrap();
        assert_eq!(payment.describe_method(), "ideal ···· 0000");
        assert_eq!(map_status("canceled"), "cancelled");
    }
}
