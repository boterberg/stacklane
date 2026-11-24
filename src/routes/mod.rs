mod admin_billing;
mod admin_provisioning;
mod admin_settings;
mod api_tokens;
mod auth;
pub mod billing;
mod dashboard;
mod gameservers;
mod health;
pub mod me;
mod notifications;
mod payments;
mod security;
mod support;
mod system;

use axum::http::StatusCode;
use axum::routing::{any, delete, get, patch, post, put};
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
        .route("/auth/reset-password", post(auth::reset_password))
        .route("/me", get(me::show))
        .route("/account/password", put(security::change_password))
        .route(
            "/account/sessions/logout-others",
            post(security::logout_other_sessions),
        )
        .route(
            "/account/api-tokens",
            get(api_tokens::index).post(api_tokens::create),
        )
        .route("/account/api-tokens/{id}", delete(api_tokens::destroy))
        .route("/me/settings", patch(me::update_settings))
        .route("/dashboard", get(dashboard::show))
        .route("/billing/subscriptions", get(billing::subscriptions))
        .route("/billing/subscriptions/{code}", get(billing::subscription))
        .route(
            "/billing/subscriptions/{code}/cancel",
            post(billing::cancel_subscription),
        )
        .route(
            "/billing/subscriptions/{code}/resume",
            post(billing::resume_subscription),
        )
        .route(
            "/billing/subscriptions/{code}/payment-method",
            put(billing::set_payment_method),
        )
        .route("/billing/invoices", get(billing::invoices))
        .route("/billing/invoices/{id}", get(billing::invoice))
        .route("/billing/invoices/{id}/pdf", get(billing::invoice_pdf))
        .route("/billing/invoices/{id}/pay", post(payments::pay))
        .route("/billing/payment-options", get(payments::options))
        .route("/billing/mandates/{id}", delete(payments::revoke_mandate))
        .route("/webhooks/mollie", post(payments::mollie_webhook))
        .route("/webhooks/stripe", post(payments::stripe_webhook))
        .route(
            "/admin/payment-settings",
            get(payments::admin_settings).put(payments::update_admin_settings),
        )
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
        .route("/gameservers", get(gameservers::index))
        .route("/gameservers/{id}", get(gameservers::show))
        .route("/gameservers/{id}/power", post(gameservers::power))
        .route("/gameservers/{id}/websocket", get(gameservers::websocket))
        .route("/gameservers/{id}/files", get(gameservers::files))
        .route(
            "/gameservers/{id}/files/content",
            get(gameservers::read_file).put(gameservers::write_file),
        )
        .route(
            "/gameservers/{id}/backups",
            get(gameservers::backups).post(gameservers::create_backup),
        )
        .route(
            "/gameservers/{id}/backups/{backup}/download",
            get(gameservers::download_backup),
        )
        .route(
            "/admin/integrations",
            get(admin_provisioning::integrations).post(admin_provisioning::create_integration),
        )
        .route(
            "/admin/integrations/{id}",
            delete(admin_provisioning::delete_integration),
        )
        .route(
            "/admin/integrations/{id}/test",
            post(admin_provisioning::test_integration),
        )
        .route(
            "/admin/integrations/{id}/catalog",
            get(admin_provisioning::catalog),
        )
        .route(
            "/admin/products",
            get(admin_provisioning::products).post(admin_provisioning::create_product),
        )
        .route(
            "/admin/services",
            get(admin_provisioning::services_index).post(admin_provisioning::create_service),
        )
        .route(
            "/admin/services/{id}/{action}",
            post(admin_provisioning::service_action),
        )
        .route("/admin/customers", get(admin_provisioning::customers))
        .route("/admin/invoices", get(admin_billing::index))
        .route("/admin/invoices/{id}", get(admin_billing::show))
        .route("/admin/invoices/{id}/pdf", get(admin_billing::pdf))
        .route(
            "/admin/invoices/{id}/mark-paid",
            post(admin_billing::mark_paid),
        )
        .route("/admin/invoices/{id}/cancel", post(admin_billing::cancel))
        .route(
            "/admin/settings",
            get(admin_settings::show).put(admin_settings::update),
        )
        .route(
            "/admin/system/update",
            get(system::show).post(system::start),
        )
        .route("/admin/system/update/check", post(system::check))
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

    router
        .layer(axum::middleware::from_fn(crate::i18n::middleware))
        .with_state(state)
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "message": crate::i18n::tr("Niet gevonden.") })),
    )
}
