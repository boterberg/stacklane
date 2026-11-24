use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

use crate::auth;
use crate::testing::{Client, PASSWORD};
use tower::ServiceExt;

async fn verification_token(client: &Client, user: uuid::Uuid, email: &str) -> String {
    let token = hex::encode(rand::random::<[u8; 32]>());
    sqlx::query(
        "INSERT INTO email_verifications (token_hash, user_id, email, expires_at)
         VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(auth::hash_token(&token))
    .bind(user)
    .bind(email)
    .execute(&client.state.db)
    .await
    .unwrap();
    token
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_session_alone_cannot_change_the_email_address(db: PgPool) {
    let client = Client::new(db);
    let (_, cookie) = client.register("owner@example.test").await;
    let mut account = client
        .send(Method::GET, "/account", Some(&cookie), None)
        .await
        .body;
    account["email"] = json!("attacker@example.test");

    let refused = client
        .send(
            Method::PUT,
            "/account",
            Some(&cookie),
            Some(account.clone()),
        )
        .await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(refused.body["errors"]["current_password"].is_string());

    account["current_password"] = json!("wrong password!");
    let wrong = client
        .send(
            Method::PUT,
            "/account",
            Some(&cookie),
            Some(account.clone()),
        )
        .await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);

    account["current_password"] = json!(PASSWORD);
    let accepted = client
        .send(Method::PUT, "/account", Some(&cookie), Some(account))
        .await;
    assert_eq!(accepted.status, StatusCode::OK);
    assert_eq!(accepted.body["email"], "owner@example.test");
    let me = client
        .send(Method::GET, "/me", Some(&cookie), None)
        .await
        .body;
    assert_eq!(me["email"], "owner@example.test");
    assert_eq!(me["pending_email"], "attacker@example.test");
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_new_address_counts_only_after_its_link_is_used(db: PgPool) {
    let client = Client::new(db);
    let (user, cookie) = client.register("old@example.test").await;
    sqlx::query(
        "INSERT INTO password_resets (token_hash, user_id, expires_at) VALUES ('x', $1, now() + interval '1 hour')",
    )
    .bind(user)
    .execute(&client.state.db)
    .await
    .unwrap();
    let token = verification_token(&client, user, "new@example.test").await;

    let verified = client
        .send(
            Method::POST,
            "/auth/verify-email",
            None,
            Some(json!({ "token": token })),
        )
        .await;
    assert_eq!(verified.status, StatusCode::NO_CONTENT);
    let me = client
        .send(Method::GET, "/me", Some(&cookie), None)
        .await
        .body;
    assert_eq!(me["email"], "new@example.test");
    assert_eq!(me["email_verified"], true);
    let resets: i64 = sqlx::query_scalar("SELECT count(*) FROM password_resets WHERE user_id = $1")
        .bind(user)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
    assert_eq!(resets, 0);

    let reused = client
        .send(
            Method::POST,
            "/auth/verify-email",
            None,
            Some(json!({ "token": token })),
        )
        .await;
    assert_eq!(reused.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn imports_only_merge_into_confirmed_or_imported_accounts(db: PgPool) {
    use crate::imports::{Account, matching_account};
    let client = Client::new(db);
    let (user, _) = client.register("first@example.test").await;
    assert!(matches!(
        matching_account(&client.state.db, "first@example.test")
            .await
            .unwrap(),
        Account::Unconfirmed
    ));
    sqlx::query("UPDATE users SET email_verified_at = now() WHERE id = $1")
        .bind(user)
        .execute(&client.state.db)
        .await
        .unwrap();
    assert!(matches!(
        matching_account(&client.state.db, "first@example.test").await.unwrap(),
        Account::Usable(id) if id == user
    ));
    assert!(matches!(
        matching_account(&client.state.db, "nobody@example.test")
            .await
            .unwrap(),
        Account::Missing
    ));
}

async fn enable_two_factor(client: &Client, user: uuid::Uuid) -> Vec<u8> {
    let secret = auth::totp::new_secret();
    let sealed = crate::payments::conceal(&client.state, &hex::encode(&secret)).unwrap();
    sqlx::query("UPDATE users SET totp_secret = $2, two_factor_enabled_at = now() WHERE id = $1")
        .bind(user)
        .bind(sealed)
        .execute(&client.state.db)
        .await
        .unwrap();
    secret
}

fn current_code(secret: &[u8]) -> String {
    let step = auth::totp::step(time::OffsetDateTime::now_utc().unix_timestamp());
    format!("{:06}", auth::totp::code_at(secret, step as u64))
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn wrong_two_factor_codes_lock_the_login(db: PgPool) {
    let client = Client::new(db);
    let (user, _) = client.register("locked@example.test").await;
    let secret = enable_two_factor(&client, user).await;
    for _ in 0..5 {
        let reply = client
            .login("locked@example.test", PASSWORD, Some("000000"))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let reply = client
        .login(
            "locked@example.test",
            PASSWORD,
            Some(&current_code(&secret)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.cookie.is_none());
    let notices: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications WHERE user_id = $1")
        .bind(user)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
    assert_eq!(notices, 1);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn one_two_factor_code_opens_one_session(db: PgPool) {
    let client = std::sync::Arc::new(Client::new(db));
    let (user, _) = client.register("replay@example.test").await;
    let secret = enable_two_factor(&client, user).await;
    let code = current_code(&secret);
    let attempts = (0..4).map(|_| {
        let client = client.clone();
        let code = code.clone();
        tokio::spawn(async move {
            client
                .login("replay@example.test", PASSWORD, Some(&code))
                .await
                .status
        })
    });
    let mut accepted = 0;
    for attempt in attempts {
        if attempt.await.unwrap() == StatusCode::OK {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 1);
}

async fn api_token(client: &Client, cookie: &str) -> String {
    let created = client
        .send(
            Method::POST,
            "/account/api-tokens",
            Some(cookie),
            Some(json!({ "name": "script", "scopes": ["account:read"], "expires_in_days": null })),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    created.body["secret"].as_str().unwrap().to_owned()
}

async fn token_works(client: &Client, secret: &str) -> bool {
    let bearer = format!("Bearer {secret}");
    client
        .send_with(
            Method::GET,
            "/me",
            None,
            None,
            &[("authorization", &bearer)],
        )
        .await
        .status
        == StatusCode::OK
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_password_reset_revokes_api_tokens(db: PgPool) {
    let client = Client::new(db);
    let (user, cookie) = client.register("tokens@example.test").await;
    let secret = api_token(&client, &cookie).await;
    assert!(token_works(&client, &secret).await);
    sqlx::query(
        "INSERT INTO password_resets (token_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '1 hour')",
    )
    .bind(auth::hash_token("reset"))
    .bind(user)
    .execute(&client.state.db)
    .await
    .unwrap();
    let reset = client
        .send(
            Method::POST,
            "/auth/reset-password",
            None,
            Some(json!({ "token": "reset", "password": "Another-pass-678" })),
        )
        .await;
    assert_eq!(reset.status, StatusCode::NO_CONTENT);
    assert!(!token_works(&client, &secret).await);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_password_change_revokes_api_tokens_and_reset_links(db: PgPool) {
    let client = Client::new(db);
    let (user, cookie) = client.register("change@example.test").await;
    let secret = api_token(&client, &cookie).await;
    sqlx::query(
        "INSERT INTO password_resets (token_hash, user_id, expires_at) VALUES ('old', $1, now() + interval '1 hour')",
    )
    .bind(user)
    .execute(&client.state.db)
    .await
    .unwrap();
    let changed = client
        .send(
            Method::PUT,
            "/account/password",
            Some(&cookie),
            Some(json!({ "current_password": PASSWORD, "password": "Another-pass-678" })),
        )
        .await;
    assert_eq!(changed.status, StatusCode::NO_CONTENT);
    assert!(!token_works(&client, &secret).await);
    let resets: i64 = sqlx::query_scalar("SELECT count(*) FROM password_resets WHERE user_id = $1")
        .bind(user)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
    assert_eq!(resets, 0);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn password_guessing_is_throttled(db: PgPool) {
    let client = Client::new(db);
    client.register("guess@example.test").await;
    let mut statuses = Vec::new();
    for _ in 0..11 {
        statuses.push(
            client
                .login("guess@example.test", "not the password", None)
                .await
                .status,
        );
    }
    assert!(
        statuses[..10]
            .iter()
            .all(|status| *status == StatusCode::UNPROCESSABLE_ENTITY)
    );
    assert_eq!(statuses[10], StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn forgot_password_sends_one_mail_per_cooldown(db: PgPool) {
    let client = Client::new(db);
    let (user, _) = client.register("forgot@example.test").await;
    for _ in 0..3 {
        let reply = client
            .send(
                Method::POST,
                "/auth/forgot-password",
                None,
                Some(json!({ "email": "forgot@example.test" })),
            )
            .await;
        assert_eq!(reply.status, StatusCode::ACCEPTED);
    }
    let resets: i64 = sqlx::query_scalar("SELECT count(*) FROM password_resets WHERE user_id = $1")
        .bind(user)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
    assert_eq!(resets, 1);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn password_hashing_does_not_stall_other_requests(db: PgPool) {
    let client = std::sync::Arc::new(Client::new(db));
    client.register("busy@example.test").await;
    let started = std::time::Instant::now();
    let logins: Vec<_> = (0..4)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move {
                client
                    .login("busy@example.test", "not the password", None)
                    .await
            })
        })
        .collect();
    tokio::task::yield_now().await;
    let meta_started = std::time::Instant::now();
    let meta = client.send(Method::GET, "/meta", None, None).await;
    let meta_took = meta_started.elapsed();
    for login in logins {
        login.await.unwrap();
    }
    let logins_took = started.elapsed();
    assert_eq!(meta.status, StatusCode::OK);
    assert!(
        meta_took * 2 < logins_took,
        "meta {meta_took:?}, logins {logins_took:?}"
    );
}

pub struct Billing {
    pub subscription: uuid::Uuid,
    pub invoice: i64,
    pub payment: uuid::Uuid,
}

async fn open_invoice(client: &Client, user: uuid::Uuid) -> Billing {
    let db = &client.state.db;
    let subscription = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, expires_at)
         VALUES ($1, $2, $3, 'active', 'monthly', now() + interval '1 month')",
    )
    .bind(subscription)
    .bind(format!("T{}", &subscription.simple().to_string()[..8]))
    .bind(user)
    .execute(db)
    .await
    .unwrap();
    let invoice: i64 = sqlx::query_scalar(
        "INSERT INTO invoices (user_id, status, total_cents, tax_cents) VALUES ($1, 'open', 1210, 210) RETURNING id",
    )
    .bind(user)
    .fetch_one(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO invoice_lines (id, invoice_id, subscription_id, description, amount_cents)
         VALUES ($1, $2, $3, 'Test', 1000)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(invoice)
    .bind(subscription)
    .execute(db)
    .await
    .unwrap();
    let payment = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payments (id, invoice_id, provider, external_id, status, amount_cents, save_mandate)
         VALUES ($1, $2, 'mollie', $3, 'open', 1210, true)",
    )
    .bind(payment)
    .bind(invoice)
    .bind(format!("tr_{}", payment.simple()))
    .execute(db)
    .await
    .unwrap();
    Billing {
        subscription,
        invoice,
        payment,
    }
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_redelivered_webhook_keeps_a_revoked_mandate_revoked(db: PgPool) {
    let client = Client::new(db);
    let (user, cookie) = client.register("mandate@example.test").await;
    let billing = open_invoice(&client, user).await;
    let state = &client.state;
    crate::payments::record_mandate(
        state,
        billing.payment,
        user,
        "mollie",
        "cst_1",
        "mdt_1",
        "iDEAL",
    )
    .await
    .unwrap();
    crate::payments::update_payment(state, billing.payment, "paid", Some("iDEAL"))
        .await
        .unwrap();
    let mandate: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM payment_mandates WHERE mandate_ref = 'mdt_1'")
            .fetch_one(&state.db)
            .await
            .unwrap();
    let revoked = client
        .send(
            Method::DELETE,
            &format!("/billing/mandates/{mandate}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(revoked.status, StatusCode::NO_CONTENT);

    crate::payments::record_mandate(
        state,
        billing.payment,
        user,
        "mollie",
        "cst_1",
        "mdt_1",
        "iDEAL",
    )
    .await
    .unwrap();
    let next = open_invoice(&client, user).await;
    crate::payments::record_mandate(
        state,
        next.payment,
        user,
        "mollie",
        "cst_1",
        "mdt_1",
        "iDEAL",
    )
    .await
    .unwrap();

    let still_revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM payment_mandates WHERE id = $1")
            .bind(mandate)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert!(still_revoked);
    let automatic: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM subscriptions WHERE user_id = $1 AND payment_mode = 'automatic'",
    )
    .bind(user)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(automatic, 0);
    let paid: String = sqlx::query_scalar("SELECT status FROM invoices WHERE id = $1")
        .bind(billing.invoice)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(paid, "paid");
    let mode: String = sqlx::query_scalar("SELECT payment_mode FROM subscriptions WHERE id = $1")
        .bind(billing.subscription)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(mode, "manual");
}

async fn status_of(client: &Client, service: uuid::Uuid) -> (String, Option<String>) {
    sqlx::query_as("SELECT status, suspension_reason FROM services WHERE id = $1")
        .bind(service)
        .fetch_one(&client.state.db)
        .await
        .unwrap()
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn paying_does_not_lift_an_admin_suspension(db: PgPool) {
    use crate::testing::{NewService, make_admin, service};
    let client = Client::new(db);
    let (user, _) = client.register("paying@example.test").await;
    let (admin, admin_cookie) = client.register("staff@example.test").await;
    make_admin(&client.state, admin).await;
    let billing = open_invoice(&client, user).await;
    let held = service(
        &client.state,
        NewService {
            user,
            subscription: Some(billing.subscription),
            kind: "other",
            status: "active",
            integration: None,
            external_id: None,
            details: json!({}),
        },
    )
    .await;
    let unpaid = service(
        &client.state,
        NewService {
            user,
            subscription: Some(billing.subscription),
            kind: "other",
            status: "active",
            integration: None,
            external_id: None,
            details: json!({}),
        },
    )
    .await;
    let suspended = client
        .send(
            Method::POST,
            &format!("/admin/services/{held}/suspend"),
            Some(&admin_cookie),
            None,
        )
        .await;
    assert_eq!(
        suspended.status,
        StatusCode::NO_CONTENT,
        "{}",
        suspended.body
    );
    crate::services::change_state_because(&client.state, unpaid, "suspend", "payment")
        .await
        .unwrap();

    crate::payments::mark_invoice_paid(&client.state, billing.invoice)
        .await
        .unwrap();

    assert_eq!(
        status_of(&client, held).await,
        ("suspended".into(), Some("admin".into()))
    );
    assert_eq!(status_of(&client, unpaid).await, ("active".into(), None));
}

fn flaky_node(method: &str, path: &str, _: &str) -> (u16, serde_json::Value) {
    use std::sync::atomic::Ordering;
    match (method, path) {
        ("POST", path) if path.ends_with("/config") && !NODE_UP.load(Ordering::SeqCst) => {
            (500, json!({ "message": "node offline" }))
        }
        ("GET", path) if path.ends_with("/status/current") => {
            (200, json!({ "data": { "status": "running" } }))
        }
        ("GET", path) if path.ends_with("/config") => (
            200,
            json!({ "data": { "scsi0": "local:vm-300-disk-0,size=20G", "scsi1": "local:vm-300-disk-1,size=10G" } }),
        ),
        _ => (200, json!({ "data": null })),
    }
}

static NODE_UP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn steady_node(method: &str, path: &str, body: &str) -> (u16, serde_json::Value) {
    if method == "POST" && path.ends_with("/config") {
        return (200, json!({ "data": null }));
    }
    flaky_node(method, path, body)
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_failed_suspension_is_retried_on_the_next_run(db: PgPool) {
    use crate::testing::{NewService, fake_api, proxmox_integration, service};
    use std::sync::atomic::Ordering;
    let client = Client::new(db);
    let (base, _) = fake_api(flaky_node).await;
    let integration = proxmox_integration(&client.state, &base).await;
    let (user, _) = client.register("unpaid@example.test").await;
    let billing = open_invoice(&client, user).await;
    sqlx::query("UPDATE subscriptions SET expires_at = now() - interval '60 days' WHERE id = $1")
        .bind(billing.subscription)
        .execute(&client.state.db)
        .await
        .unwrap();
    let vps = service(
        &client.state,
        NewService {
            user,
            subscription: Some(billing.subscription),
            kind: "vps",
            status: "active",
            integration: Some(integration),
            external_id: Some("300"),
            details: json!({}),
        },
    )
    .await;

    NODE_UP.store(false, Ordering::SeqCst);
    crate::renewals::run_once(&client.state).await.unwrap();
    assert_eq!(status_of(&client, vps).await.0, "active");

    NODE_UP.store(true, Ordering::SeqCst);
    crate::renewals::run_once(&client.state).await.unwrap();
    assert_eq!(
        status_of(&client, vps).await,
        ("suspended".into(), Some("payment".into()))
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn billing_runs_never_overlap(db: PgPool) {
    let client = Client::new(db);
    let mut other = client.state.db.acquire().await.unwrap();
    let taken: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext('stacklane billing run'))")
            .fetch_one(&mut *other)
            .await
            .unwrap();
    assert!(taken);
    let (user, _) = client.register("renewal@example.test").await;
    let billing = open_invoice(&client, user).await;
    sqlx::query("UPDATE subscriptions SET expires_at = now() + interval '1 day' WHERE id = $1")
        .bind(billing.subscription)
        .execute(&client.state.db)
        .await
        .unwrap();
    let run = crate::renewals::run_once(&client.state).await.unwrap();
    assert_eq!(run.invoices + run.suspended + run.ended + run.expired, 0);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_foreign_site_cannot_post_with_the_session_cookie(db: PgPool) {
    let client = Client::new(db);
    let (_, cookie) = client.register("csrf@example.test").await;
    let foreign = client
        .send_with(
            Method::POST,
            "/auth/logout",
            Some(&cookie),
            None,
            &[("origin", "https://evil.panel.test")],
        )
        .await;
    assert_eq!(foreign.status, StatusCode::FORBIDDEN);
    let me = client.send(Method::GET, "/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::OK);
    let own = client
        .send(Method::POST, "/auth/logout", Some(&cookie), None)
        .await;
    assert_eq!(own.status, StatusCode::NO_CONTENT);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn pages_cannot_be_framed(db: PgPool) {
    let client = Client::new(db);
    let response = client
        .app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/meta")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let csp = response.headers()["content-security-policy"]
        .to_str()
        .unwrap();
    assert!(csp.contains("frame-ancestors 'none'"));
    assert_eq!(response.headers()["x-frame-options"], "DENY");
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn turning_off_two_factor_needs_a_code(db: PgPool) {
    let client = Client::new(db);
    let (user, cookie) = client.register("off@example.test").await;
    let secret = enable_two_factor(&client, user).await;
    let without = client
        .send(
            Method::POST,
            "/account/two-factor/disable",
            Some(&cookie),
            Some(json!({ "password": PASSWORD })),
        )
        .await;
    assert_eq!(without.status, StatusCode::UNPROCESSABLE_ENTITY);
    let renew = client
        .send(
            Method::POST,
            "/account/two-factor/recovery-codes",
            Some(&cookie),
            Some(json!({ "password": PASSWORD, "code": "000000" })),
        )
        .await;
    assert_eq!(renew.status, StatusCode::UNPROCESSABLE_ENTITY);
    let with = client
        .send(
            Method::POST,
            "/account/two-factor/disable",
            Some(&cookie),
            Some(json!({ "password": PASSWORD, "code": current_code(&secret) })),
        )
        .await;
    assert_eq!(with.status, StatusCode::NO_CONTENT, "{}", with.body);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn only_local_checks_see_the_version(db: PgPool) {
    use axum::extract::ConnectInfo;
    let client = Client::new(db);
    let ask = |peer: &str, forwarded: bool| {
        let mut request = axum::http::Request::builder().uri("/readyz");
        if forwarded {
            request = request.header("x-forwarded-for", "203.0.113.9");
        }
        let mut request = request.body(axum::body::Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()));
        let app = client.app.clone();
        async move {
            let response = app.oneshot(request).await.unwrap();
            let bytes = axum::body::to_bytes(response.into_body(), 1 << 16)
                .await
                .unwrap();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        }
    };
    assert!(ask("127.0.0.1:9000", false).await["version"].is_string());
    assert!(ask("127.0.0.1:9000", true).await["version"].is_null());
    assert!(ask("198.51.100.7:9000", false).await["version"].is_null());
}
