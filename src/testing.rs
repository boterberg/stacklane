use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::config::Config;
use crate::crypto::SecretBox;
use crate::mail::Mailer;
use crate::state::AppState;
use crate::updates::Updates;

pub fn state(db: PgPool) -> AppState {
    let config = Config {
        database_url: String::new(),
        bind_addr: "127.0.0.1:0".into(),
        cookie_secure: false,
        static_dir: None,
        demo_mode: false,
        secrets: Some(
            SecretBox::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                .expect("test key"),
        ),
        public_url: "http://panel.test".into(),
        update_dir: PathBuf::from("/nonexistent"),
    };
    AppState {
        db,
        updates: Arc::new(Updates::new(config.update_dir.clone(), None)),
        config: Arc::new(config),
        mailer: Arc::new(Mailer::Disabled),
    }
}

pub struct Client {
    pub app: Router,
    pub state: AppState,
}

pub struct Reply {
    pub status: StatusCode,
    pub body: Value,
    pub cookie: Option<String>,
}

impl Client {
    pub fn new(db: PgPool) -> Self {
        let state = state(db);
        Self {
            app: crate::routes::router(state.clone()),
            state,
        }
    }

    pub async fn send(
        &self,
        method: Method,
        path: &str,
        cookie: Option<&str>,
        body: Option<Value>,
    ) -> Reply {
        self.send_with(method, path, cookie, body, &[]).await
    }

    pub async fn send_with(
        &self,
        method: Method,
        path: &str,
        cookie: Option<&str>,
        body: Option<Value>,
        headers: &[(&str, &str)],
    ) -> Reply {
        let mut request = Request::builder()
            .method(method)
            .uri(format!("/api{path}"))
            .header(header::ORIGIN, "http://panel.test");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .expect("request");
        let response = self.app.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let cookie = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| value.starts_with("stacklane_session="))
            .and_then(|value| value.split(';').next())
            .map(str::to_owned);
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("body");
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Reply {
            status,
            body,
            cookie,
        }
    }

    pub async fn register(&self, email: &str) -> (Uuid, String) {
        let reply = self
            .send(
                Method::POST,
                "/auth/register",
                None,
                Some(json!({
                    "name": "Test Klant",
                    "email": email,
                    "password": PASSWORD,
                    "country": "NL",
                })),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::OK,
            "register {email}: {}",
            reply.body
        );
        let id = reply.body["id"]
            .as_str()
            .expect("id")
            .parse()
            .expect("uuid");
        (id, reply.cookie.expect("session cookie"))
    }

    pub async fn login(&self, email: &str, password: &str, code: Option<&str>) -> Reply {
        self.send(
            Method::POST,
            "/auth/login",
            None,
            Some(json!({ "email": email, "password": password, "code": code })),
        )
        .await
    }
}

pub const PASSWORD: &str = "Dummy-pass-12345";
