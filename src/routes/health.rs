use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::config::VERSION;
use crate::state::AppState;

pub async fn live() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

fn local(request: &axum::extract::Request) -> bool {
    !request.headers().contains_key("x-forwarded-for")
        && request
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .is_some_and(|info| info.0.ip().is_loopback())
}

pub async fn ready(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> (StatusCode, Json<Value>) {
    let version = local(&request).then_some(VERSION);
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "version": version })),
        ),
        Err(error) => {
            tracing::error!(?error, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "unavailable" })),
            )
        }
    }
}
