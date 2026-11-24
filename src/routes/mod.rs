mod auth;
mod dashboard;
mod health;
pub mod me;
mod notifications;
mod support;

use axum::http::StatusCode;
use axum::routing::{any, get, patch, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

use crate::demo;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/forgot-password", post(auth::forgot_password))
        .route("/me", get(me::show))
        .route("/me/settings", patch(me::update_settings))
        .route("/dashboard", get(dashboard::show))
        .route("/notifications", get(notifications::index))
        .route("/support/departments", get(support::departments))
        .route(
            "/support/tickets",
            get(support::index).post(support::create),
        )
        .route("/support/tickets/{number}", get(support::show))
        .route("/support/tickets/{number}/replies", post(support::reply))
        .route("/support/tickets/{number}/close", post(support::close))
        .route("/admin/tickets", get(support::staff_index))
        .route("/admin/tickets/{number}", get(support::staff_show))
        .route(
            "/admin/tickets/{number}/replies",
            post(support::staff_reply),
        )
        .route(
            "/admin/tickets/{number}/status",
            patch(support::staff_update_status),
        )
        .route("/meta", get(demo::meta))
        .route("/demo/login", post(demo::login))
        .route("/{*rest}", any(api_not_found));

    let router = Router::new()
        .nest("/api", api)
        .route("/healthz", get(health::live))
        .route("/readyz", get(health::ready));

    let router = match &state.config.static_dir {
        Some(dir) => router
            .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")))),
        None => router,
    };

    router.with_state(state)
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "message": "Niet gevonden." })),
    )
}
