use serde::{Deserialize, Serialize};

pub const EU_RATES: [(&str, i32); 27] = [
    ("AT", 2000),
    ("BE", 2100),
    ("BG", 2000),
    ("CY", 1900),
    ("CZ", 2100),
    ("DE", 1900),
    ("DK", 2500),
    ("EE", 2400),
    ("ES", 2100),
    ("FI", 2550),
    ("FR", 2000),
    ("GR", 2400),
    ("HR", 2500),
    ("HU", 2700),
    ("IE", 2300),
    ("IT", 2200),
    ("LT", 2100),
    ("LU", 1700),
    ("LV", 2100),
    ("MT", 1800),
    ("NL", 2100),
    ("PL", 2300),
    ("PT", 2300),
    ("RO", 2100),
    ("SE", 2500),
    ("SI", 2200),
    ("SK", 2300),
];

pub fn eu_rate(country: &str) -> Option<i32> {
    EU_RATES
        .iter()
        .find(|(code, _)| *code == country)
        .map(|(_, rate)| *rate)
}

pub fn vies_prefix(country: &str) -> &str {
    if country == "GR" { "EL" } else { country }
}

pub fn split_vat_number(raw: &str, country: &str) -> Option<(String, String)> {
    let cleaned: String = raw
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_uppercase())
        .collect();
    let prefix = vies_prefix(country);
    let (code, number) = match cleaned.get(..2) {
        Some(head) if head.chars().all(|c| c.is_ascii_alphabetic()) => {
            (head.to_owned(), cleaned[2..].to_owned())
        }
        _ => (prefix.to_owned(), cleaned),
    };
    let country_of_code = if code == "EL" { "GR" } else { code.as_str() };
    (code == prefix && eu_rate(country_of_code).is_some() && (2..=12).contains(&number.len()))
        .then_some((code, number))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViesReply {
    is_valid: bool,
    #[serde(default)]
    user_error: Option<String>,
}

pub async fn check_vies(code: &str, number: &str) -> anyhow::Result<bool> {
    let reply: ViesReply = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("stacklane/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(format!(
            "https://ec.europa.eu/taxation_customs/vies/rest-api/ms/{code}/vat/{number}"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    match reply.user_error.as_deref() {
        None | Some("VALID" | "INVALID") => Ok(reply.is_valid),
        Some(other) => anyhow::bail!("VIES is niet beschikbaar ({other})"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scheme {
    Domestic,
    Oss,
    ReverseCharge,
    OutsideEu,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Domestic => "domestic",
            Self::Oss => "oss",
            Self::ReverseCharge => "reverse_charge",
            Self::OutsideEu => "outside_eu",
        }
    }
}

pub struct Seller<'a> {
    pub country: &'a str,
    pub rate: i32,
    pub oss: bool,
}

pub struct Buyer<'a> {
    pub country: &'a str,
    pub verified_vat_number: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Decision {
    pub scheme: Scheme,
    pub country: String,
    pub rate: i32,
}

pub fn decide(seller: &Seller, buyer: &Buyer) -> Decision {
    let domestic = || Decision {
        scheme: Scheme::Domestic,
        country: seller.country.to_owned(),
        rate: seller.rate,
    };
    if buyer.country == seller.country {
        return domestic();
    }
    let Some(buyer_rate) = eu_rate(buyer.country) else {
        return Decision {
            scheme: Scheme::OutsideEu,
            country: buyer.country.to_owned(),
            rate: 0,
        };
    };
    if buyer.verified_vat_number {
        return Decision {
            scheme: Scheme::ReverseCharge,
            country: buyer.country.to_owned(),
            rate: 0,
        };
    }
    if seller.oss {
        return Decision {
            scheme: Scheme::Oss,
            country: buyer.country.to_owned(),
            rate: buyer_rate,
        };
    }
    domestic()
}

pub fn convert(gross_at_home: i64, home_rate: i32, rate: i32) -> i64 {
    if home_rate == rate {
        return gross_at_home;
    }
    let home = i64::from(10_000 + home_rate);
    let target = i64::from(10_000 + rate);
    (gross_at_home * target * 2 + home).div_euclid(home * 2)
}

#[cfg(test)]
mod tests {
    use super::{Buyer, Scheme, Seller, convert, decide, eu_rate, split_vat_number, vies_prefix};

    const SELLER: Seller = Seller {
        country: "NL",
        rate: 2100,
        oss: true,
    };

    fn scheme(country: &str, verified: bool, oss: bool) -> (Scheme, i32) {
        let seller = Seller { oss, ..SELLER };
        let decision = decide(
            &seller,
            &Buyer {
                country,
                verified_vat_number: verified,
            },
        );
        (decision.scheme, decision.rate)
    }

    #[test]
    fn picks_the_right_scheme() {
        assert_eq!(scheme("NL", true, true), (Scheme::Domestic, 2100));
        assert_eq!(scheme("BE", false, true), (Scheme::Oss, 2100));
        assert_eq!(scheme("DE", false, true), (Scheme::Oss, 1900));
        assert_eq!(scheme("DE", false, false), (Scheme::Domestic, 2100));
        assert_eq!(scheme("DE", true, true), (Scheme::ReverseCharge, 0));
        assert_eq!(scheme("DE", true, false), (Scheme::ReverseCharge, 0));
        assert_eq!(scheme("US", false, true), (Scheme::OutsideEu, 0));
        assert_eq!(scheme("GB", true, true), (Scheme::OutsideEu, 0));
        assert_eq!(eu_rate("FI"), Some(2550));
        assert_eq!(vies_prefix("GR"), "EL");
        let split = |raw, country| split_vat_number(raw, country);
        assert_eq!(
            split("NL 8590.48.356 B01", "NL"),
            Some(("NL".to_owned(), "859048356B01".to_owned()))
        );
        assert_eq!(
            split("123456789", "BE"),
            Some(("BE".to_owned(), "123456789".to_owned()))
        );
        assert_eq!(split("NL859048356B01", "DE"), None);
        assert_eq!(split("DE123456789", "NL"), None);
        assert_eq!(
            split("EL123456789", "GR"),
            Some(("EL".to_owned(), "123456789".to_owned()))
        );
        assert_eq!(split("GB123456789", "GB"), None);
        assert_eq!(split("x", "NL"), None);
    }

    #[test]
    fn converts_gross_prices_between_rates() {
        assert_eq!(convert(1210, 2100, 2100), 1210);
        assert_eq!(convert(1210, 2100, 1900), 1190);
        assert_eq!(convert(1210, 2100, 0), 1000);
        assert_eq!(convert(1210, 2100, 2700), 1270);
        assert_eq!(convert(499, 2100, 1700), 483);
    }
}
