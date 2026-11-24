use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::config::VERSION;
use crate::state::AppState;

pub async fn live() -> Json<Value> {
    Json(json!({ "status": "ok", "version": VERSION }))
}

pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "version": VERSION })),
        ),
        Err(error) => {
            tracing::error!(?error, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "unavailable", "version": VERSION })),
            )
        }
    }
}
