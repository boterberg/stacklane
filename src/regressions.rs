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
async fn a_suspended_volume_is_detached_from_its_server(db: PgPool) {
    use crate::testing::{NewService, fake_api, make_admin, proxmox_integration, service};
    let client = Client::new(db);
    let (base, calls) = fake_api(steady_node).await;
    let integration = proxmox_integration(&client.state, &base).await;
    let (user, _) = client.register("volume@example.test").await;
    let (admin, admin_cookie) = client.register("volume-staff@example.test").await;
    make_admin(&client.state, admin).await;
    let vps = service(
        &client.state,
        NewService {
            user,
            subscription: None,
            kind: "vps",
            status: "active",
            integration: Some(integration),
            external_id: Some("300"),
            details: json!({}),
        },
    )
    .await;
    let volume = service(
        &client.state,
        NewService {
            user,
            subscription: None,
            kind: "volume",
            status: "active",
            integration: Some(integration),
            external_id: Some("local:vm-300-disk-1"),
            details: json!({ "host": "300", "server_id": vps.to_string(), "size_gb": 10 }),
        },
    )
    .await;

    let reply = client
        .send(
            Method::POST,
            &format!("/admin/services/{volume}/suspend"),
            Some(&admin_cookie),
            None,
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    assert!(
        calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| call == "POST /api2/json/nodes/pve/qemu/300/config delete=scsi1")
    );
    let details: serde_json::Value =
        sqlx::query_scalar("SELECT details FROM services WHERE id = $1")
            .bind(volume)
            .fetch_one(&client.state.db)
            .await
            .unwrap();
    assert!(details.get("server_id").is_none());
    assert_eq!(details["suspended_from"], vps.to_string());
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
async fn shared_access_needs_consent_and_ends_with_the_owner(db: PgPool) {
    let client = Client::new(db);
    let (owner, owner_cookie) = client.register("vps-owner@example.test").await;
    let (_, member_cookie) = client.register("vps-member@example.test").await;
    let invited = client
        .send(
            Method::POST,
            "/vps/members",
            Some(&owner_cookie),
            Some(json!({ "email": "vps-member@example.test", "permissions": ["servers.view"] })),
        )
        .await;
    assert_eq!(invited.status, StatusCode::CREATED, "{}", invited.body);
    let owner_header = owner.to_string();
    let as_member = |cookie: String| {
        let owner_header = owner_header.clone();
        let client = &client;
        async move {
            client
                .send_with(
                    Method::GET,
                    "/vps/access",
                    Some(&cookie),
                    None,
                    &[("x-vps-owner", &owner_header)],
                )
                .await
                .status
        }
    };
    assert_eq!(
        as_member(member_cookie.clone()).await,
        StatusCode::FORBIDDEN
    );

    let accepted = client
        .send(
            Method::POST,
            &format!("/account/shared-access/{owner}/accept"),
            Some(&member_cookie),
            None,
        )
        .await;
    assert_eq!(accepted.status, StatusCode::NO_CONTENT);
    assert_eq!(as_member(member_cookie.clone()).await, StatusCode::OK);

    let deleted = client
        .send(
            Method::POST,
            "/account/delete",
            Some(&owner_cookie),
            Some(json!({ "password": PASSWORD })),
        )
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
    assert_eq!(as_member(member_cookie).await, StatusCode::FORBIDDEN);
}

async fn shared_member(
    client: &Client,
    owner_cookie: &str,
    owner: uuid::Uuid,
    email: &str,
    rights: &[&str],
) -> String {
    let (_, cookie) = client.register(email).await;
    let invited = client
        .send(
            Method::POST,
            "/vps/members",
            Some(owner_cookie),
            Some(json!({ "email": email, "permissions": rights })),
        )
        .await;
    assert_eq!(invited.status, StatusCode::CREATED, "{}", invited.body);
    client
        .send(
            Method::POST,
            &format!("/account/shared-access/{owner}/accept"),
            Some(&cookie),
            None,
        )
        .await;
    cookie
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_member_needs_the_matching_right_for_every_firewall_change(db: PgPool) {
    let client = Client::new(db);
    let (owner, owner_cookie) = client.register("fw-owner@example.test").await;
    let rules = json!([
        { "type": "in", "action": "ACCEPT", "proto": "tcp", "dport": "22", "source": "203.0.113.4" },
        { "type": "in", "action": "DROP", "proto": "tcp", "dport": "22" },
    ]);
    let created = client
        .send(
            Method::POST,
            "/vps/firewalls",
            Some(&owner_cookie),
            Some(json!({ "name": "ssh-guard", "rules": rules })),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_str().unwrap().to_owned();
    let member = shared_member(
        &client,
        &owner_cookie,
        owner,
        "fw-member@example.test",
        &["firewalls.view", "firewalls.update", "firewalls.create"],
    )
    .await;
    let owner_header = owner.to_string();
    let put = |body: serde_json::Value| {
        let (client, member, owner_header, id) =
            (&client, member.clone(), owner_header.clone(), id.clone());
        async move {
            client
                .send_with(
                    Method::PUT,
                    &format!("/vps/firewalls/{id}"),
                    Some(&member),
                    Some(body),
                    &[("x-vps-owner", &owner_header)],
                )
                .await
                .status
        }
    };
    let reversed = json!([rules[1], rules[0]]);
    assert_eq!(
        put(json!({ "name": "ssh-guard", "rules": reversed })).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        put(json!({ "name": "ssh-guard", "rules": rules, "auto_assign": true })).await,
        StatusCode::FORBIDDEN
    );
    let empty_auto = client
        .send_with(
            Method::POST,
            "/vps/firewalls",
            Some(&member),
            Some(json!({ "name": "auto", "auto_assign": true, "rules": [] })),
            &[("x-vps-owner", &owner_header)],
        )
        .await;
    assert_eq!(empty_auto.status, StatusCode::FORBIDDEN);
    assert_eq!(
        put(json!({ "name": "renamed", "rules": rules })).await,
        StatusCode::OK
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn concurrent_iso_imports_respect_the_quota(db: PgPool) {
    use crate::testing::{NewService, fake_api, proxmox_integration, service};
    let client = std::sync::Arc::new(Client::new(db));
    let (base, _) = fake_api(steady_node).await;
    let integration = proxmox_integration(&client.state, &base).await;
    let (user, cookie) = client.register("iso@example.test").await;
    service(
        &client.state,
        NewService {
            user,
            subscription: None,
            kind: "vps",
            status: "active",
            integration: Some(integration),
            external_id: Some("300"),
            details: json!({}),
        },
    )
    .await;
    let imports: Vec<_> = (0..6)
        .map(|index| {
            let (client, cookie) = (client.clone(), cookie.clone());
            tokio::spawn(async move {
                client
                    .send(
                        Method::POST,
                        "/vps/isos",
                        Some(&cookie),
                        Some(json!({ "name": format!("race {index}"), "url": "https://1.1.1.1/x.iso", "firmware": "bios" })),
                    )
                    .await
                    .status
            })
        })
        .collect();
    let mut created = 0;
    for import in imports {
        if import.await.unwrap() == StatusCode::CREATED {
            created += 1;
        }
    }
    assert_eq!(created, 2);
    for _ in 0..100 {
        let importing: i64 =
            sqlx::query_scalar("SELECT count(*) FROM vps_isos WHERE status = 'importing'")
                .fetch_one(&client.state.db)
                .await
                .unwrap();
        if importing == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
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

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn password_checks_stop_after_ten_tries(db: PgPool) {
    let client = Client::new(db);
    let (_, cookie) = client.register("guess@example.test").await;
    for _ in 0..10 {
        let reply = client
            .send(
                Method::POST,
                "/account/two-factor/recovery-codes",
                Some(&cookie),
                Some(json!({ "password": "wrong-guess-123", "code": "000000" })),
            )
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let reply = client
        .send(
            Method::POST,
            "/account/two-factor/recovery-codes",
            Some(&cookie),
            Some(json!({ "password": PASSWORD, "code": "000000" })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_delegate_cannot_remove_a_stronger_member(db: PgPool) {
    let client = Client::new(db);
    let (owner, owner_cookie) = client.register("rank-owner@example.test").await;
    let delegate = shared_member(
        &client,
        &owner_cookie,
        owner,
        "rank-delegate@example.test",
        &["access.users", "servers.view"],
    )
    .await;
    shared_member(
        &client,
        &owner_cookie,
        owner,
        "rank-admin@example.test",
        &["access.users", "servers.view", "servers.rebuild"],
    )
    .await;
    let members = client
        .send(Method::GET, "/vps/members", Some(&owner_cookie), None)
        .await;
    let stronger = members
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["email"] == "rank-admin@example.test")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let removed = client
        .send_with(
            Method::DELETE,
            &format!("/vps/members/{stronger}"),
            Some(&delegate),
            None,
            &[("x-vps-owner", &owner.to_string())],
        )
        .await;
    assert_eq!(removed.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_restart_frees_work_that_was_cut_off(db: PgPool) {
    use crate::testing::{NewService, service};
    let client = Client::new(db);
    let (user, _) = client.register("restart@example.test").await;
    let stuck = service(
        &client.state,
        NewService {
            user,
            subscription: None,
            kind: "vps",
            status: "provisioning",
            integration: None,
            external_id: None,
            details: json!({}),
        },
    )
    .await;
    crate::services::recover_interrupted(&client.state)
        .await
        .unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM services WHERE id = $1")
        .bind(stuck)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn terminating_a_service_ends_its_billing(db: PgPool) {
    use crate::testing::{NewService, make_admin, service};
    let client = Client::new(db);
    let (user, _) = client.register("ending@example.test").await;
    let (admin, _) = client.register("ending-staff@example.test").await;
    make_admin(&client.state, admin).await;
    let billing = open_invoice(&client, user).await;
    let ended = service(
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
    crate::services::change_state(&client.state, ended, "terminate")
        .await
        .unwrap();
    let db = &client.state.db;
    let subscription: String = sqlx::query_scalar("SELECT status FROM subscriptions WHERE id = $1")
        .bind(billing.subscription)
        .fetch_one(db)
        .await
        .unwrap();
    assert_eq!(subscription, "cancelled");
    let invoice: String = sqlx::query_scalar("SELECT status FROM invoices WHERE id = $1")
        .bind(billing.invoice)
        .fetch_one(db)
        .await
        .unwrap();
    assert_eq!(invoice, "cancelled");

    crate::payments::mark_invoice_paid(&client.state, billing.invoice)
        .await
        .unwrap();
    let flagged: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications WHERE user_id = $1")
        .bind(admin)
        .fetch_one(db)
        .await
        .unwrap();
    assert_eq!(flagged, 1);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn a_domain_on_one_hosting_package_cannot_be_ordered_again(db: PgPool) {
    use crate::testing::{NewService, fake_api, proxmox_integration, service};
    let client = Client::new(db);
    let (base, _) = fake_api(steady_node).await;
    let integration = proxmox_integration(&client.state, &base).await;
    let product = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO products (id, kind, slug, name, monthly_price_cents, integration_id)
         VALUES ($1, 'webhosting', 'web-test', 'Web', 500, $2)",
    )
    .bind(product)
    .bind(integration)
    .execute(&client.state.db)
    .await
    .unwrap();
    let (first, _) = client.register("site-owner@example.test").await;
    service(
        &client.state,
        NewService {
            user: first,
            subscription: None,
            kind: "webhosting",
            status: "active",
            integration: Some(integration),
            external_id: None,
            details: json!({}),
        },
    )
    .await;
    sqlx::query("UPDATE services SET name = 'example-shop.nl' WHERE user_id = $1")
        .bind(first)
        .execute(&client.state.db)
        .await
        .unwrap();
    let (_, second) = client.register("site-taker@example.test").await;
    let order = client
        .send(
            Method::POST,
            "/orders",
            Some(&second),
            Some(json!({ "product_id": product, "billing_interval": "monthly", "name": "www.Example-Shop.nl" })),
        )
        .await;
    assert_eq!(
        order.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        order.body
    );
    assert!(
        order.body.to_string().contains("al in gebruik"),
        "{}",
        order.body
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn an_order_left_without_an_invoice_expires(db: PgPool) {
    let client = Client::new(db);
    let (user, _) = client.register("orphan@example.test").await;
    let subscription = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, started_at, expires_at)
         VALUES ($1, 'ORPHAN1', $2, 'pending', 'monthly', now() - interval '2 days', now())",
    )
    .bind(subscription)
    .bind(user)
    .execute(&client.state.db)
    .await
    .unwrap();
    crate::renewals::run_once(&client.state).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM subscriptions WHERE id = $1")
        .bind(subscription)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
    assert_eq!(status, "expired");
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn an_older_address_does_not_block_saving_the_profile(db: PgPool) {
    let client = Client::new(db);
    let (user, cookie) = client.register("legacy@example.test").await;
    sqlx::query("UPDATE users SET email = 'jörg@müller.example' WHERE id = $1")
        .bind(user)
        .execute(&client.state.db)
        .await
        .unwrap();
    let mut profile = client
        .send(Method::GET, "/account", Some(&cookie), None)
        .await
        .body;
    profile["first_name"] = json!("Jörg");
    let saved = client
        .send(Method::PUT, "/account", Some(&cookie), Some(profile))
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
}

const KEPT_ISO: &str = "5a1e0000-0000-4000-8000-000000000001";

fn iso_storage(method: &str, path: &str, _: &str) -> (u16, serde_json::Value) {
    match (method, path) {
        ("GET", path) if path.contains("/content?content=iso") => (
            200,
            json!({ "data": [
                { "volid": "local:iso/stacklane-5a1e0000000040008000000000000001.iso" },
                { "volid": "local:iso/stacklane-0rphan.iso" },
                { "volid": "local:iso/debian-13.iso" },
            ] }),
        ),
        _ => (200, json!({ "data": null })),
    }
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn iso_files_nobody_owns_are_removed(db: PgPool) {
    use crate::testing::{fake_api, proxmox_integration};
    let client = Client::new(db);
    let (base, calls) = fake_api(iso_storage).await;
    let integration = proxmox_integration(&client.state, &base).await;
    let (user, _) = client.register("iso-sweep@example.test").await;
    sqlx::query(
        "INSERT INTO vps_isos (id, user_id, integration_id, name, url, firmware, status, external_id)
         VALUES ($1, $2, $3, 'kept', 'https://1.1.1.1/x.iso', 'bios', 'ready', 'stacklane-5a1e0000000040008000000000000001.iso')",
    )
    .bind(uuid::Uuid::parse_str(KEPT_ISO).unwrap())
    .bind(user)
    .bind(integration)
    .execute(&client.state.db)
    .await
    .unwrap();
    crate::routes::vps_isos::remove_unused(&client.state)
        .await
        .unwrap();
    let deletes: Vec<String> = calls
        .lock()
        .unwrap()
        .iter()
        .filter(|call| call.starts_with("DELETE"))
        .cloned()
        .collect();
    assert_eq!(deletes.len(), 1, "{deletes:?}");
    assert!(deletes[0].contains("stacklane-0rphan.iso"));
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn one_server_cannot_collect_unlimited_firewall_rules(db: PgPool) {
    use crate::testing::{NewService, fake_api, proxmox_integration, service};
    let client = Client::new(db);
    let (base, _) = fake_api(steady_node).await;
    let integration = proxmox_integration(&client.state, &base).await;
    let (user, cookie) = client.register("fw-crowd@example.test").await;
    let server = service(
        &client.state,
        NewService {
            user,
            subscription: None,
            kind: "vps",
            status: "active",
            integration: Some(integration),
            external_id: Some("300"),
            details: json!({}),
        },
    )
    .await;
    let ports: String = (1000..1015)
        .map(|port| port.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let rules: Vec<_> = (0..4)
        .map(|_| json!({ "type": "in", "action": "ACCEPT", "proto": "tcp", "dport": ports }))
        .collect();
    let save = |name: &str| {
        let body = json!({ "name": name, "rules": rules, "server_ids": [server] });
        let (client, cookie) = (&client, cookie.clone());
        async move {
            client
                .send(Method::POST, "/vps/firewalls", Some(&cookie), Some(body))
                .await
        }
    };
    let first = save("first").await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);
    let second = save("second").await;
    assert_eq!(
        second.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        second.body
    );
}
