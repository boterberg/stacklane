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
        limits: Arc::new(crate::limits::Limits::default()),
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

pub type Calls = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

pub async fn fake_api(respond: fn(&str, &str, &str) -> (u16, Value)) -> (String, Calls) {
    let calls: Calls = Calls::default();
    let recorded = calls.clone();
    let app = Router::new().fallback(move |request: axum::extract::Request| {
        let recorded = recorded.clone();
        async move {
            let method = request.method().to_string();
            let path = request
                .uri()
                .path_and_query()
                .map(|value| value.to_string())
                .unwrap_or_default();
            let bytes = axum::body::to_bytes(request.into_body(), 1 << 20)
                .await
                .unwrap_or_default();
            let body = String::from_utf8_lossy(&bytes).into_owned();
            recorded
                .lock()
                .unwrap()
                .push(format!("{method} {path} {body}").trim_end().to_owned());
            let (status, value) = respond(&method, &path, &body);
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
                axum::Json(value),
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (base, calls)
}

pub async fn proxmox_integration(state: &AppState, base: &str) -> Uuid {
    let id = Uuid::new_v4();
    let sealed = crate::integrations::seal_credentials(
        state,
        &json!({ "token_id": "t", "token_secret": "s" }),
    )
    .expect("seal");
    sqlx::query(
        "INSERT INTO integrations (id, kind, name, base_url, credentials, settings)
         VALUES ($1, 'proxmox', 'Test', $2, $3, $4)",
    )
    .bind(id)
    .bind(base)
    .bind(sealed)
    .bind(json!({ "node": "pve", "gateway": "192.0.2.1", "ip_from": "192.0.2.10", "ip_to": "192.0.2.20" }))
    .execute(&state.db)
    .await
    .expect("integration");
    id
}

pub struct NewService<'a> {
    pub user: Uuid,
    pub subscription: Option<Uuid>,
    pub kind: &'a str,
    pub status: &'a str,
    pub integration: Option<Uuid>,
    pub external_id: Option<&'a str>,
    pub details: Value,
}

pub async fn service(state: &AppState, new: NewService<'_>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO services (id, user_id, subscription_id, integration_id, kind, name, status, external_id, details)
         VALUES ($1, $2, $3, $4, $5, 'Test', $6, $7, $8)",
    )
    .bind(id)
    .bind(new.user)
    .bind(new.subscription)
    .bind(new.integration)
    .bind(new.kind)
    .bind(new.status)
    .bind(new.external_id)
    .bind(new.details)
    .execute(&state.db)
    .await
    .expect("service");
    id
}

pub async fn make_admin(state: &AppState, user: Uuid) {
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = $1")
        .bind(user)
        .execute(&state.db)
        .await
        .expect("admin");
}
