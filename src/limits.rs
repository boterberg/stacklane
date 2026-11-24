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
pub const INVITE: Rule = Rule {
    name: "invite",
    max: 20,
    window: Duration::from_secs(60 * 60),
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
        if hits.len() > 50_000 {
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
        INVITE,
        DOMAIN_SEARCH,
    ]
    .into_iter()
    .find(|rule| rule.name == name)
    .map_or(Duration::from_secs(3600), |rule| rule.window)
}

pub struct ClientIp(pub String);

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
        Ok(Self(
            ip.map_or_else(|| "unknown".to_owned(), |ip| ip.to_string()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
