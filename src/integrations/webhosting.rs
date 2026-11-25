pub struct NewAccount<'a> {
    pub username: &'a str,
    pub domain: &'a str,
    pub email: &'a str,
    pub password: &'a str,
    pub plan: &'a str,
    pub owner: &'a str,
}

pub fn valid_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    domain.len() <= 253
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
            .is_some_and(|tld| tld.len() >= 2 && !tld.chars().all(|c| c.is_ascii_digit()))
}

pub fn username_for(domain: &str, suffix: &str) -> String {
    let letters: String = domain
        .split('.')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .skip_while(char::is_ascii_digit)
        .take(6)
        .collect();
    let base = if letters.is_empty() {
        "site".to_owned()
    } else {
        letters
    };
    format!("{base}{suffix}")
}

pub fn password() -> String {
    const LOWER: &[u8] = b"abcdefghijkmnpqrstuvwxyz";
    const UPPER: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    const DIGITS: &[u8] = b"23456789";
    let bytes = rand::random::<[u8; 20]>();
    bytes
        .iter()
        .enumerate()
        .map(|(position, byte)| {
            let set = match position % 4 {
                0 | 2 => LOWER,
                1 => UPPER,
                _ => DIGITS,
            };
            char::from(set[usize::from(*byte) % set.len()])
        })
        .collect()
}

pub enum Hosting<'a> {
    Cpanel(&'a super::cpanel::Cpanel),
    DirectAdmin(&'a super::directadmin::DirectAdmin),
    Plesk(&'a super::plesk::Plesk),
}

pub fn plesk_parts(external_id: &str) -> (&str, &str) {
    external_id
        .split_once('|')
        .unwrap_or((external_id, external_id))
}

impl Hosting<'_> {
    pub fn from_driver(driver: &super::Driver) -> Option<Hosting<'_>> {
        match driver {
            super::Driver::Cpanel(client) => Some(Hosting::Cpanel(client)),
            super::Driver::DirectAdmin(client) => Some(Hosting::DirectAdmin(client)),
            super::Driver::Plesk(client) => Some(Hosting::Plesk(client)),
            _ => None,
        }
    }

    pub fn panel_name(&self) -> &'static str {
        match self {
            Self::Cpanel(_) => "cPanel",
            Self::DirectAdmin(_) => "DirectAdmin",
            Self::Plesk(_) => "Plesk",
        }
    }

    pub fn panel_url(&self) -> String {
        match self {
            Self::Cpanel(client) => client.panel_url(),
            Self::DirectAdmin(client) => client.panel_url(),
            Self::Plesk(client) => client.panel_url(),
        }
    }

    pub async fn plans(&self) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cpanel(client) => client.plans().await,
            Self::DirectAdmin(client) => client.plans().await,
            Self::Plesk(client) => client.plans().await,
        }
    }

    pub async fn create(&self, account: &NewAccount<'_>) -> anyhow::Result<String> {
        match self {
            Self::Cpanel(client) => client.create(account).await,
            Self::DirectAdmin(client) => client.create(account).await,
            Self::Plesk(client) => {
                client.create(account).await?;
                return Ok(format!("{}|{}", account.username, account.domain));
            }
        }?;
        Ok(account.username.to_owned())
    }

    pub async fn login_url(&self, external_id: &str, user_ip: &str) -> anyhow::Result<String> {
        match self {
            Self::Cpanel(client) => client.login_url(external_id).await,
            Self::DirectAdmin(client) => client.login_url(external_id).await,
            Self::Plesk(client) => client.login_url(plesk_parts(external_id).0, user_ip).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{password, username_for, valid_domain};

    #[test]
    fn makes_short_usernames() {
        assert_eq!(username_for("bakkerijdejong.nl", "a7"), "bakkera7");
        assert_eq!(username_for("123-test.nl", "b2"), "testb2");
        assert_eq!(username_for("xn--bcher.de", "c3"), "xnbchec3");
        assert_eq!(username_for("42.nl", "d4"), "sited4");
        assert!(valid_domain("bakkerijdejong.nl"));
        assert!(valid_domain("shop.voorbeeld.co.uk"));
        assert!(!valid_domain("geen domein"));
        assert!(!valid_domain("localhost"));
        assert!(!valid_domain("-a.nl"));
        assert!(!valid_domain("10.0.0.1"));
        let generated = password();
        assert_eq!(generated.len(), 20);
        assert!(generated.chars().any(|c| c.is_ascii_digit()));
        assert!(generated.chars().any(|c| c.is_ascii_uppercase()));
    }
}
