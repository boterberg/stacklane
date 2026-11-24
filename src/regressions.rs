use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

use crate::auth;
use crate::testing::{Client, PASSWORD};

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
