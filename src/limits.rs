use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub struct Rule {
    name: &'static str,
    max: u32,
    window: Duration,
}

pub const LOGIN: Rule = Rule {
    name: "login",
    max: 20,
    window: Duration::from_secs(60),
};
pub const LOGIN_ACCOUNT: Rule = Rule {
    name: "login-account",
    max: 10,
    window: Duration::from_secs(15 * 60),
};
pub const REGISTER: Rule = Rule {
    name: "register",
    max: 5,
    window: Duration::from_secs(60 * 60),
};
pub const RECOVERY: Rule = Rule {
    name: "recovery",
    max: 5,
    window: Duration::from_secs(15 * 60),
};
pub const VERIFY: Rule = Rule {
    name: "verify",
    max: 20,
    window: Duration::from_secs(15 * 60),
};
pub const REAUTH: Rule = Rule {
    name: "reauth",
    max: 10,
    window: Duration::from_secs(15 * 60),
};
pub const TICKET: Rule = Rule {
    name: "ticket",
    max: 10,
    window: Duration::from_secs(60 * 60),
};
pub const TICKET_REPLY: Rule = Rule {
    name: "ticket-reply",
    max: 60,
    window: Duration::from_secs(60 * 60),
};
pub const INVITE: Rule = Rule {
    name: "invite",
    max: 20,
    window: Duration::from_secs(60 * 60),
};
pub const RDNS: Rule = Rule {
    name: "rdns",
    max: 30,
    window: Duration::from_secs(60 * 60),
};
pub const UNKNOWN_PAYMENT: Rule = Rule {
    name: "unknown-payment",
    max: 60,
    window: Duration::from_secs(60 * 60),
};
pub const DOMAIN_CHANGE: Rule = Rule {
    name: "domain-change",
    max: 20,
    window: Duration::from_secs(60 * 60),
};
pub const GAME_PANEL: Rule = Rule {
    name: "game-panel",
    max: 120,
    window: Duration::from_secs(60),
};
pub const DOMAIN_SEARCH: Rule = Rule {
    name: "domain-search",
    max: 30,
    window: Duration::from_secs(60),
};

type Hits = HashMap<(&'static str, String), (Instant, u32)>;

#[derive(Default)]
pub struct Limits {
    hits: Mutex<Hits>,
}

impl Limits {
    pub fn check(&self, rule: &Rule, key: &str) -> AppResult<()> {
        let now = Instant::now();
        let mut hits = self
            .hits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if hits.len() > 10_000 {
            hits.retain(|(name, _), (started, _)| now.duration_since(*started) < window_of(name));
        }
        let entry = hits.entry((rule.name, key.to_owned())).or_insert((now, 0));
        if now.duration_since(entry.0) >= rule.window {
            *entry = (now, 0);
        }
        if entry.1 >= rule.max {
            return Err(AppError::TooMany);
        }
        entry.1 += 1;
        Ok(())
    }
}

fn window_of(name: &str) -> Duration {
    [
        LOGIN,
        LOGIN_ACCOUNT,
        REGISTER,
        RECOVERY,
        VERIFY,
        REAUTH,
        TICKET,
        TICKET_REPLY,
        INVITE,
        RDNS,
        UNKNOWN_PAYMENT,
        DOMAIN_CHANGE,
        GAME_PANEL,
        DOMAIN_SEARCH,
    ]
    .into_iter()
    .find(|rule| rule.name == name)
    .map_or(Duration::from_secs(3600), |rule| rule.window)
}

pub struct ClientIp(pub String);

fn bucket(ip: IpAddr) -> String {
    match ip {
        IpAddr::V6(v6) if v6.to_ipv4_mapped().is_none() => {
            let [a, b, c, d, ..] = v6.segments();
            format!("{a:x}:{b:x}:{c:x}:{d:x}::/64")
        }
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map_or_else(|| v6.to_string(), |v4| v4.to_string()),
        IpAddr::V4(v4) => v4.to_string(),
    }
}

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip());
        let forwarded = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.rsplit(',').next())
            .and_then(|value| value.trim().parse::<IpAddr>().ok());
        let ip = match (peer, forwarded) {
            (Some(peer), Some(forwarded)) if peer.is_loopback() => Some(forwarded),
            (Some(peer), _) => Some(peer),
            (None, forwarded) => forwarded,
        };
        Ok(Self(ip.map_or_else(|| "unknown".to_owned(), bucket)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn client_ip(peer: &str, forwarded: Option<&str>) -> String {
        let mut request = axum::http::Request::builder();
        if let Some(forwarded) = forwarded {
            request = request.header("x-forwarded-for", forwarded);
        }
        let (mut parts, ()) = request.body(()).unwrap().into_parts();
        parts
            .extensions
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        let state = crate::testing::state(
            sqlx::PgPool::connect_lazy("postgres://unused@127.0.0.1/unused").unwrap(),
        );
        ClientIp::from_request_parts(&mut parts, &state)
            .await
            .ok()
            .unwrap()
            .0
    }

    #[tokio::test]
    async fn trusts_forwarded_addresses_only_from_the_local_proxy() {
        assert_eq!(
            client_ip("198.51.100.7:4000", Some("203.0.113.9")).await,
            "198.51.100.7"
        );
        assert_eq!(
            client_ip("127.0.0.1:4000", Some("203.0.113.9, 198.51.100.8")).await,
            "198.51.100.8"
        );
        assert_eq!(client_ip("127.0.0.1:4000", None).await, "127.0.0.1");
        assert_eq!(
            client_ip("[2001:db8:1:2:aaaa::1]:4000", None).await,
            client_ip("[2001:db8:1:2:bbbb::9]:4000", None).await
        );
    }

    #[test]
    fn refuses_after_the_maximum_until_the_window_ends() {
        let limits = Limits::default();
        let rule = Rule {
            name: "test",
            max: 2,
            window: Duration::from_millis(50),
        };
        assert!(limits.check(&rule, "a").is_ok());
        assert!(limits.check(&rule, "a").is_ok());
        assert!(limits.check(&rule, "a").is_err());
        assert!(limits.check(&rule, "b").is_ok());
        std::thread::sleep(Duration::from_millis(60));
        assert!(limits.check(&rule, "a").is_ok());
    }
}
