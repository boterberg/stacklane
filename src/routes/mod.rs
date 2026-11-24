mod account;
mod admin_billing;
mod admin_domains;
mod admin_provisioning;
mod admin_settings;
mod admin_vat;
mod api_tokens;
mod auth;
pub mod billing;
mod dashboard;
mod domains;
mod email_verification;
mod gameservers;
mod health;
pub mod me;
mod notifications;
mod payments;
mod security;
mod shop;
mod ssh_keys;
mod support;
mod system;
mod two_factor;
mod vps;
pub mod vps_access;
pub mod vps_firewalls;
pub mod vps_isos;
mod vps_volumes;
mod webhosting;
pub mod whois_profiles;

use axum::extract::State;
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
        .route("/auth/verify-email", post(email_verification::verify))
        .route("/account/email/resend", post(email_verification::resend))
        .route("/account/email/pending", delete(email_verification::cancel))
        .route("/me", get(me::show))
        .route("/account", get(account::show).put(account::update))
        .route("/account/delete", post(account::destroy))
        .route(
            "/account/payment-defaults",
            get(account::payment_defaults).put(account::update_payment_defaults),
        )
        .route("/vps/access", get(vps_access::me))
        .route(
            "/vps/members",
            get(vps_access::index).post(vps_access::create),
        )
        .route(
            "/vps/members/{id}",
            put(vps_access::update).delete(vps_access::destroy),
        )
        .route("/account/shared-access", get(vps_access::shared))
        .route("/account/shared-access/{owner}", delete(vps_access::leave))
        .route(
            "/account/shared-access/{owner}/accept",
            post(vps_access::accept),
        )
        .route("/vps/servers", get(vps::index))
        .route("/vps/snapshots", get(vps::all_snapshots))
        .route("/vps/isos", get(vps_isos::index).post(vps_isos::create))
        .route("/vps/isos/{id}", delete(vps_isos::destroy))
        .route(
            "/vps/servers/{id}/iso",
            get(vps_isos::server)
                .post(vps_isos::mount)
                .delete(vps_isos::unmount),
        )
        .route("/vps/volumes", get(vps_volumes::index))
        .route("/vps/volumes/options", get(vps_volumes::options))
        .route("/vps/volumes/{id}/attach", post(vps_volumes::attach))
        .route("/vps/volumes/{id}/detach", post(vps_volumes::detach))
        .route("/vps/firewalls/servers", get(vps_firewalls::servers))
        .route(
            "/vps/firewalls",
            get(vps_firewalls::index).post(vps_firewalls::create),
        )
        .route(
            "/vps/firewalls/{id}",
            get(vps_firewalls::show)
                .put(vps_firewalls::update)
                .delete(vps_firewalls::destroy),
        )
        .route("/vps/servers/{id}", get(vps::show))
        .route("/vps/servers/{id}/power", post(vps::power))
        .route("/vps/servers/{id}/password", get(vps::root_password))
        .route("/vps/servers/{id}/panel", post(vps::panel_login))
        .route(
            "/vps/servers/{id}/snapshots",
            get(vps::snapshots).post(vps::create_snapshot),
        )
        .route(
            "/vps/servers/{id}/snapshots/{name}",
            delete(vps::delete_snapshot),
        )
        .route(
            "/vps/servers/{id}/snapshots/{name}/rollback",
            post(vps::rollback_snapshot),
        )
        .route(
            "/vps/servers/{id}/firewall",
            get(vps::firewall).post(vps::add_firewall_rule),
        )
        .route(
            "/vps/servers/{id}/firewall/{rule}",
            delete(vps::delete_firewall_rule),
        )
        .route("/shop/products", get(shop::products))
        .route("/orders", post(shop::place))
        .route("/webhosting", get(webhosting::index))
        .route("/webhosting/{id}", get(webhosting::show))
        .route("/webhosting/{id}/panel", post(webhosting::panel_login))
        .route("/ssh-keys", get(ssh_keys::index).post(ssh_keys::create))
        .route("/domains/search", get(domains::search))
        .route("/domains/orders", post(domains::order))
        .route("/domains", get(domains::index))
        .route("/domains/{id}", get(domains::show))
        .route("/domains/{id}/nameservers", put(domains::set_nameservers))
        .route("/domains/{id}/auth-code", post(domains::auth_code))
        .route("/domains/{id}/lock", put(domains::set_lock))
        .route(
            "/whois-profiles",
            get(whois_profiles::index).post(whois_profiles::create),
        )
        .route(
            "/whois-profiles/{id}",
            put(whois_profiles::update).delete(whois_profiles::destroy),
        )
        .route("/ssh-keys/{id}", delete(ssh_keys::destroy))
        .route("/account/password", put(security::change_password))
        .route("/account/two-factor", get(two_factor::status))
        .route("/account/two-factor/setup", post(two_factor::setup))
        .route("/account/two-factor/enable", post(two_factor::enable))
        .route("/account/two-factor/disable", post(two_factor::disable))
        .route(
            "/account/two-factor/recovery-codes",
            post(two_factor::new_recovery_codes),
        )
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
            "/admin/domain-extensions",
            get(admin_domains::index).post(admin_domains::create),
        )
        .route(
            "/admin/domain-extensions/{id}",
            put(admin_domains::update).delete(admin_domains::destroy),
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
        .route("/admin/reports/vat", get(admin_vat::report))
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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            demo_guard,
        ))
        .layer(axum::middleware::from_fn(crate::i18n::middleware))
        .with_state(state)
}

fn demo_allows(method: &axum::http::Method, path: &str) -> bool {
    let Some(admin) = path.strip_prefix("/api/admin/") else {
        return true;
    };
    if admin.starts_with("integrations") {
        return false;
    }
    if matches!(*method, axum::http::Method::GET | axum::http::Method::HEAD) {
        return true;
    }
    let parts: Vec<&str> = admin.split('/').collect();
    matches!(
        parts.as_slice(),
        ["tickets", _, "replies" | "status"]
            | ["products"]
            | ["products", _]
            | ["services", _, _]
            | ["invoices", _, "mark-paid" | "cancel"]
    )
}

async fn demo_guard(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if state.config.demo_mode && !demo_allows(request.method(), request.uri().path()) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "message": crate::i18n::tr("In de demo kun je dit niet wijzigen.") })),
        )
            .into_response();
    }
    next.run(request).await
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "message": crate::i18n::tr("Niet gevonden.") })),
    )
}

#[cfg(test)]
mod tests {
    use axum::http::Method;

    use super::demo_allows;

    #[test]
    fn the_demo_keeps_admins_away_from_configuration() {
        assert!(!demo_allows(&Method::POST, "/api/admin/integrations"));
        assert!(!demo_allows(
            &Method::GET,
            "/api/admin/integrations/1/catalog"
        ));
        assert!(!demo_allows(&Method::PUT, "/api/admin/payment-settings"));
        assert!(!demo_allows(&Method::PUT, "/api/admin/settings"));
        assert!(!demo_allows(&Method::POST, "/api/admin/domain-extensions"));
        assert!(!demo_allows(&Method::POST, "/api/admin/system/update"));
        assert!(demo_allows(&Method::GET, "/api/admin/settings"));
        assert!(demo_allows(
            &Method::POST,
            "/api/admin/tickets/1001/replies"
        ));
        assert!(demo_allows(
            &Method::POST,
            "/api/admin/invoices/100001/mark-paid"
        ));
        assert!(demo_allows(&Method::POST, "/api/auth/login"));
    }
}
