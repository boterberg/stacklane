use axum::http::Method;

pub const PREFIX: &str = "stl_";

pub struct Area {
    pub key: &'static str,
    pub paths: &'static [&'static str],
    pub admin_only: bool,
}

pub const AREAS: &[Area] = &[
    Area {
        key: "account",
        paths: &["/me", "/notifications", "/dashboard", "/meta"],
        admin_only: false,
    },
    Area {
        key: "gameservers",
        paths: &["/gameservers"],
        admin_only: false,
    },
    Area {
        key: "vps",
        paths: &["/vps", "/ssh-keys"],
        admin_only: false,
    },
    Area {
        key: "webhosting",
        paths: &["/webhosting"],
        admin_only: false,
    },
    Area {
        key: "billing",
        paths: &["/billing", "/shop", "/orders"],
        admin_only: false,
    },
    Area {
        key: "support",
        paths: &["/support"],
        admin_only: false,
    },
    Area {
        key: "admin",
        paths: &["/admin"],
        admin_only: true,
    },
];

fn area_for(path: &str) -> Option<&'static Area> {
    let path = path.strip_prefix("/api").unwrap_or(path);
    AREAS.iter().find(|area| {
        area.paths.iter().any(|prefix| {
            path == *prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    })
}

fn writes(method: &Method, path: &str) -> bool {
    !matches!(*method, Method::GET | Method::HEAD) || path.ends_with("/websocket")
}

pub fn allows(scopes: &[String], method: &Method, path: &str) -> bool {
    let Some(area) = area_for(path) else {
        return false;
    };
    let write = format!("{}:write", area.key);
    if writes(method, path) {
        return scopes.contains(&write);
    }
    scopes.contains(&write) || scopes.contains(&format!("{}:read", area.key))
}

pub fn valid_scope(scope: &str, is_admin: bool) -> bool {
    let Some((key, level)) = scope.split_once(':') else {
        return false;
    };
    matches!(level, "read" | "write")
        && AREAS
            .iter()
            .any(|area| area.key == key && (is_admin || !area.admin_only))
}

pub fn generate() -> String {
    format!("{PREFIX}{}", hex::encode(rand::random::<[u8; 24]>()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scopes(list: &[&str]) -> Vec<String> {
        list.iter().map(|scope| (*scope).to_owned()).collect()
    }

    #[test]
    fn reading_needs_read_or_write() {
        let read = scopes(&["gameservers:read"]);
        assert!(allows(&read, &Method::GET, "/api/gameservers/abc"));
        assert!(!allows(&read, &Method::POST, "/api/gameservers/abc/power"));
        assert!(!allows(
            &read,
            &Method::GET,
            "/api/gameservers/abc/websocket"
        ));
        let write = scopes(&["gameservers:write"]);
        assert!(allows(&write, &Method::GET, "/gameservers"));
        assert!(allows(&write, &Method::POST, "/gameservers/abc/power"));
    }

    #[test]
    fn unknown_paths_are_closed() {
        let all = scopes(&["account:write", "billing:write", "support:write"]);
        assert!(!allows(&all, &Method::GET, "/api/account/api-tokens"));
        assert!(!allows(&all, &Method::POST, "/api/auth/logout"));
        assert!(!allows(&all, &Method::GET, "/api/billingx"));
        assert!(allows(&all, &Method::GET, "/api/me"));
    }

    #[test]
    fn admin_scopes_only_for_admins() {
        assert!(valid_scope("admin:read", true));
        assert!(!valid_scope("admin:read", false));
        assert!(valid_scope("support:write", false));
        assert!(!valid_scope("support:delete", false));
        assert!(!valid_scope("vps", false));
    }
}
