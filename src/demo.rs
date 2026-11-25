use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth;
use crate::error::{AppError, AppResult};
use crate::routes::me::{Profile, load_profile};
use crate::state::AppState;

pub const CUSTOMER_EMAIL: &str = "klant@demo.stacklane.io";
pub const ADMIN_EMAIL: &str = "beheer@demo.stacklane.io";

#[derive(Serialize)]
pub struct Meta {
    demo: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    next_reset_at: Option<OffsetDateTime>,
    company: Option<crate::settings::Company>,
    currency: String,
    branding: crate::routes::branding::BrandingInfo,
}

#[derive(Deserialize)]
pub struct DemoLogin {
    role: String,
}

fn next_full_hour(now: OffsetDateTime) -> OffsetDateTime {
    let truncated = now
        .replace_minute(0)
        .and_then(|value| value.replace_second(0))
        .and_then(|value| value.replace_nanosecond(0))
        .unwrap_or(now);
    truncated + Duration::hours(1)
}

pub async fn meta(State(state): State<AppState>) -> AppResult<Json<Meta>> {
    let demo = state.config.demo_mode;
    let company: crate::settings::Company = crate::settings::get(&state.db, "company").await?;
    let billing: crate::settings::Billing = crate::settings::get(&state.db, "billing").await?;
    Ok(Json(Meta {
        demo,
        next_reset_at: demo.then(|| next_full_hour(OffsetDateTime::now_utc())),
        company: (!company.name.is_empty()).then_some(company),
        currency: billing.currency,
        branding: crate::routes::branding::info(&state).await?,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<DemoLogin>,
) -> AppResult<(CookieJar, Json<Profile>)> {
    if !state.config.demo_mode {
        return Err(AppError::NotFound);
    }
    let email = match input.role.as_str() {
        "customer" => CUSTOMER_EMAIL,
        "admin" => ADMIN_EMAIL,
        _ => return Err(AppError::field("role", "Kies klant of beheerder.")),
    };
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::field("role", "De demo wordt op dit moment opnieuw opgezet."))?;
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let (token, max_age) = auth::create_session(&state, user_id, false, user_agent).await?;
    let profile = load_profile(&state, user_id).await?;
    Ok((
        jar.add(auth::session_cookie(&state, token, max_age)),
        Json(profile),
    ))
}

struct Seed<'a> {
    transaction: Transaction<'a, Postgres>,
    now: OffsetDateTime,
}

impl Seed<'_> {
    fn ago(&self, hours: i64) -> OffsetDateTime {
        self.now - Duration::hours(hours)
    }

    fn ahead(&self, days: i64) -> OffsetDateTime {
        self.now + Duration::days(days)
    }

    async fn user(
        &mut self,
        email: &str,
        first_name: &str,
        last_name: &str,
        role: &str,
        company: Option<&str>,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        let password = hex::encode(rand::random::<[u8; 24]>());
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, first_name, last_name, role, company_name, is_business,
                                country, address_line1, city, postal_code, theme, created_at, email_verified_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'NL', 'Stationsplein 12', 'Utrecht', '3511 ED', 'dark', $9, $9)",
        )
        .bind(id)
        .bind(email)
        .bind(auth::hash(&password).await?)
        .bind(first_name)
        .bind(last_name)
        .bind(role)
        .bind(company)
        .bind(company.is_some())
        .bind(self.ago(24 * 90))
        .execute(&mut *self.transaction)
        .await?;
        Ok(id)
    }

    async fn department(&mut self, name: &str) -> anyhow::Result<Uuid> {
        Ok(
            sqlx::query_scalar("SELECT id FROM departments WHERE name = $1")
                .bind(name)
                .fetch_one(&mut *self.transaction)
                .await?,
        )
    }

    async fn ticket(
        &mut self,
        customer: Uuid,
        department: &str,
        subject: &str,
        status: &str,
        messages: &[(Option<Uuid>, bool, i64, &str)],
    ) -> anyhow::Result<()> {
        let department_id = self.department(department).await?;
        let ticket_id = Uuid::new_v4();
        let created_at = self.ago(messages.first().map(|message| message.2).unwrap_or(0));
        let last_reply_at = self.ago(messages.last().map(|message| message.2).unwrap_or(0));
        sqlx::query(
            "INSERT INTO tickets (id, user_id, department_id, subject, status, created_at, last_reply_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(ticket_id)
        .bind(customer)
        .bind(department_id)
        .bind(subject)
        .bind(status)
        .bind(created_at)
        .bind(last_reply_at)
        .execute(&mut *self.transaction)
        .await?;
        for (author, is_staff, hours_ago, body) in messages {
            sqlx::query(
                "INSERT INTO ticket_messages (id, ticket_id, author_user_id, is_staff, body_html, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(ticket_id)
            .bind(author)
            .bind(is_staff)
            .bind(body)
            .bind(self.ago(*hours_ago))
            .execute(&mut *self.transaction)
            .await?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn subscription(
        &mut self,
        customer: Uuid,
        code: &str,
        status: &str,
        interval: &str,
        payment_mode: &str,
        expires_in_days: i64,
        item: (&str, &str, i32),
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, payment_mode, started_at, expires_at, cancelled_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CASE WHEN $4 = 'cancelled' THEN $7 + interval '20 days' END)",
        )
        .bind(id)
        .bind(code)
        .bind(customer)
        .bind(status)
        .bind(interval)
        .bind(payment_mode)
        .bind(self.ago(24 * 60))
        .bind(self.ahead(expires_in_days))
        .execute(&mut *self.transaction)
        .await?;
        sqlx::query(
            "INSERT INTO subscription_items (id, subscription_id, description, specs_line, monthly_price_cents)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(item.0)
        .bind(item.1)
        .bind(item.2)
        .execute(&mut *self.transaction)
        .await?;
        Ok(id)
    }

    async fn product(
        &mut self,
        kind: &str,
        name: &str,
        description: &str,
        specs: serde_json::Value,
        monthly_price_cents: i32,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        let slug = format!("{kind}-{}", name.to_lowercase().replace(' ', "-"));
        sqlx::query(
            "INSERT INTO products (id, kind, slug, name, description, specs, monthly_price_cents)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(kind)
        .bind(slug)
        .bind(name)
        .bind(description)
        .bind(specs)
        .bind(monthly_price_cents)
        .execute(&mut *self.transaction)
        .await?;
        Ok(id)
    }

    async fn service(
        &mut self,
        customer: Uuid,
        product: Uuid,
        subscription: Uuid,
        kind: &str,
        name: &str,
        details: serde_json::Value,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO services (id, user_id, product_id, subscription_id, kind, name, status, details, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)",
        )
        .bind(id)
        .bind(customer)
        .bind(product)
        .bind(subscription)
        .bind(kind)
        .bind(name)
        .bind(details)
        .bind(self.ago(24 * 60))
        .execute(&mut *self.transaction)
        .await?;
        Ok(id)
    }

    async fn attach(
        &mut self,
        service: Uuid,
        integration: Uuid,
        external_id: &str,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE services SET integration_id = $2, external_id = $3 WHERE id = $1")
            .bind(service)
            .bind(integration)
            .bind(external_id)
            .execute(&mut *self.transaction)
            .await?;
        Ok(())
    }

    async fn integration(
        &mut self,
        kind: &str,
        name: &str,
        settings: serde_json::Value,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO integrations (id, kind, name, base_url, credentials, settings)
             VALUES ($1, $2, $3, 'https://demo.invalid', ''::bytea, $4)",
        )
        .bind(id)
        .bind(kind)
        .bind(name)
        .bind(settings)
        .execute(&mut *self.transaction)
        .await?;
        Ok(id)
    }

    async fn invoice(
        &mut self,
        customer: Uuid,
        subscription: Uuid,
        status: &str,
        days_ago: i64,
        description: &str,
        amount_cents: i32,
    ) -> anyhow::Result<()> {
        let issued = (self.now - Duration::days(days_ago)).date();
        let tax_cents = amount_cents - (f64::from(amount_cents) / 1.21).round() as i32;
        let invoice_id: i64 = sqlx::query_scalar(
            "INSERT INTO invoices (user_id, status, issued_at, due_at, paid_at, total_cents, tax_cents, billing_snapshot)
             VALUES ($1, $2, $3, $3 + 14, CASE WHEN $2 = 'paid' THEN $3::timestamptz END, $4, $5,
                     '{\"company\": \"Jansen Bouw\", \"city\": \"Utrecht\", \"country\": \"NL\"}')
             RETURNING id",
        )
        .bind(customer)
        .bind(status)
        .bind(issued)
        .bind(amount_cents)
        .bind(tax_cents)
        .fetch_one(&mut *self.transaction)
        .await?;
        sqlx::query(
            "INSERT INTO invoice_lines (id, invoice_id, subscription_id, description, period_start, period_end, amount_cents)
             VALUES ($1, $2, $3, $4, $5, $5 + 29, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(invoice_id)
        .bind(subscription)
        .bind(description)
        .bind(issued)
        .bind(amount_cents)
        .execute(&mut *self.transaction)
        .await?;
        Ok(())
    }

    async fn notification(
        &mut self,
        user: Uuid,
        title: &str,
        body: &str,
        link: &str,
        hours_ago: i64,
    ) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, created_at) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(user)
            .bind(title)
            .bind(body)
            .bind(link)
            .bind(self.ago(hours_ago))
            .execute(&mut *self.transaction)
            .await?;
        Ok(())
    }
}

pub async fn reset(pool: &PgPool) -> anyhow::Result<()> {
    let transaction = pool.begin().await?;
    let mut seed = Seed {
        transaction,
        now: OffsetDateTime::now_utc(),
    };

    sqlx::query(
        "TRUNCATE sessions, notifications, ticket_messages, tickets, invoice_lines, invoices,
                  subscription_items, subscriptions, services, products, users, integrations,
                  domain_extensions, ptr_cleanups, mail_log, brand_assets RESTART IDENTITY CASCADE",
    )
    .execute(&mut *seed.transaction)
    .await?;
    sqlx::query("DELETE FROM settings WHERE key <> 'company'")
        .execute(&mut *seed.transaction)
        .await?;
    sqlx::query("ALTER SEQUENCE invoices_id_seq RESTART WITH 100001")
        .execute(&mut *seed.transaction)
        .await?;
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('company', $1)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(serde_json::json!({
        "name": "Stacklane Demo B.V.",
        "address": "Demostraat 1, 1234 AB Amsterdam",
        "email": "support@demo.stacklane.io",
        "website": "demo.stacklane.io",
        "coc_number": "12345678",
        "vat_number": "NL001234567B01",
        "iban": "NL00 DEMO 0123 4567 89",
    }))
    .execute(&mut *seed.transaction)
    .await?;

    let admin = seed
        .user(ADMIN_EMAIL, "Sam", "de Vries", "admin", None)
        .await?;
    let customer = seed
        .user(
            CUSTOMER_EMAIL,
            "Jan",
            "Jansen",
            "customer",
            Some("Jansen Bouw"),
        )
        .await?;

    seed.ticket(
        customer,
        "Webhosting",
        "Website laadt traag sinds de verhuizing",
        "answered",
        &[
            (Some(customer), false, 50, "<p>Hallo,</p><p>Sinds we gisteren zijn verhuisd laadt <strong>jansenbouw.nl</strong> erg traag, soms wel tien seconden. Kunnen jullie meekijken?</p>"),
            (Some(admin), true, 26, "<p>Hoi Jan,</p><p>We zien dat de PHP-versie nog op 7.4 stond. We hebben hem naar 8.3 gezet en de cache aangezet. Wil je het opnieuw proberen?</p>"),
        ],
    )
    .await?;
    seed.ticket(
        customer,
        "Gameservers",
        "Back-up van gisteren terugzetten",
        "open",
        &[(Some(customer), false, 3, "<p>Een van de spelers heeft per ongeluk een deel van de wereld opgeblazen. Kan de back-up van gisteravond 22:00 terug?</p>")],
    )
    .await?;
    seed.ticket(
        customer,
        "Algemeen",
        "Vraag over de factuur van vorige maand",
        "closed",
        &[
            (Some(customer), false, 24 * 21, "<p>Op de factuur staat de VPS twee keer. Klopt dat?</p>"),
            (Some(admin), true, 24 * 20, "<p>Dat was een fout aan onze kant. We hebben het bedrag teruggestort, je ziet het binnen drie werkdagen op je rekening.</p>"),
            (Some(customer), false, 24 * 20 - 2, "<p>Top, bedankt!</p>"),
        ],
    )
    .await?;

    let vps = seed
        .subscription(
            customer,
            "K7QX2M",
            "active",
            "monthly",
            "manual",
            5,
            ("VPS: Cloud S", "1 vCPU · 2 GB RAM · 20 GB opslag", 650),
        )
        .await?;
    let game = seed
        .subscription(
            customer,
            "P3MD8R",
            "active",
            "monthly",
            "automatic",
            19,
            ("Gameserver: 4 GB", "Minecraft · 4 GB RAM", 1250),
        )
        .await?;
    let hosting = seed
        .subscription(
            customer,
            "W9HT4C",
            "active",
            "yearly",
            "manual",
            210,
            ("Webhosting: Plus", "20 GB opslag · jansenbouw.nl", 500),
        )
        .await?;
    seed.subscription(
        customer,
        "B2LN6V",
        "cancelled",
        "monthly",
        "manual",
        -40,
        ("VPS: Cloud M", "2 vCPU · 4 GB RAM · 40 GB opslag", 1250),
    )
    .await?;

    let cloud_s = seed
        .product(
            "vps",
            "Cloud S",
            "Voor een website, een API of een kleine database.",
            json!({ "cores": 1, "memory_mb": 2048, "disk_gb": 20 }),
            650,
        )
        .await?;
    let cloud_m = seed
        .product(
            "vps",
            "Cloud M",
            "Voor drukkere sites en een paar containers naast elkaar.",
            json!({ "cores": 2, "memory_mb": 4096, "disk_gb": 40 }),
            1250,
        )
        .await?;
    seed.product(
        "vps",
        "Cloud L",
        "Voor webshops, CI-runners en zwaardere databases.",
        json!({ "cores": 4, "memory_mb": 8192, "disk_gb": 80 }),
        2400,
    )
    .await?;
    seed.product(
        "webhosting",
        "Start",
        "1 website · 5 GB opslag · 5 e-mailadressen",
        json!({ "plan": "start" }),
        300,
    )
    .await?;
    let plus = seed
        .product(
            "webhosting",
            "Plus",
            "10 websites · 20 GB opslag · onbeperkt e-mail",
            json!({ "plan": "plus" }),
            500,
        )
        .await?;
    seed.product(
        "webhosting",
        "Pro",
        "Onbeperkt websites · 50 GB opslag · dagelijkse back-ups",
        json!({ "plan": "pro" }),
        900,
    )
    .await?;
    seed.product(
        "gameserver",
        "2 GB",
        "Voor een kleine server met vrienden.",
        json!({ "memory": 2048 }),
        650,
    )
    .await?;
    let four_gb = seed
        .product(
            "gameserver",
            "4 GB",
            "Voor 10 tot 20 spelers of een paar mods.",
            json!({ "memory": 4096 }),
            1250,
        )
        .await?;
    seed.product(
        "gameserver",
        "8 GB",
        "Voor grote modpacks en drukke servers.",
        json!({ "memory": 8192 }),
        2400,
    )
    .await?;

    let web = seed.service(
        customer,
        cloud_s,
        vps,
        "vps",
        "web-01",
        json!({ "hostname": "web-01.jansenbouw.nl", "ipv4": "203.0.113.10", "image": "Ubuntu 24.04", "cores": 1, "memory_mb": 2048, "disk_gb": 20 }),
    )
    .await?;
    seed.service(
        customer,
        four_gb,
        game,
        "gameserver",
        "Jansen SMP",
        json!({ "address": "play.jansenbouw.nl:25565", "egg": "Paper", "memory": 4096, "cpu": 200, "disk": 20480 }),
    )
    .await?;
    seed.service(
        customer,
        plus,
        hosting,
        "webhosting",
        "jansenbouw.nl",
        json!({ "username": "jansenbo", "plan": "plus", "panel": "DirectAdmin" }),
    )
    .await?;

    extras(&mut seed, customer, web, cloud_m).await?;

    seed.invoice(customer, vps, "paid", 55, "VPS: Cloud S", 650)
        .await?;
    seed.invoice(customer, game, "paid", 41, "Gameserver: 4 GB", 1250)
        .await?;
    seed.invoice(
        customer,
        hosting,
        "paid",
        150,
        "Webhosting: Plus, jaar",
        5100,
    )
    .await?;
    seed.invoice(customer, vps, "open", 2, "VPS: Cloud S", 650)
        .await?;

    seed.notification(
        customer,
        "Support heeft gereageerd",
        "Er is een reactie op je ticket over de trage website.",
        "/support/tickets/1",
        26,
    )
    .await?;
    seed.notification(
        customer,
        "Je VPS verloopt binnenkort",
        "Cloud S verloopt over 5 dagen. Verleng hem om onderbreking te voorkomen.",
        "/",
        4,
    )
    .await?;

    seed.transaction.commit().await?;
    tracing::info!("demo data reset");
    Ok(())
}

const DEMO_KEYS: [(&str, &str); 2] = [
    (
        "Laptop Jan",
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFMXMMPuV8/LagCn6t+bZMYKO61iYwgYxyx5ZfuWI0Ws jan@laptop",
    ),
    (
        "Deploy-server",
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIH/YjZrm0YUrDnrhEGhcKTzjs9ntR3PQSIU2Vm93QZxz deploy@ci",
    ),
];

async fn extras(
    seed: &mut Seed<'_>,
    customer: Uuid,
    web: Uuid,
    cloud_m: Uuid,
) -> anyhow::Result<()> {
    let proxmox = seed
        .integration(
            "proxmox",
            "Proxmox Amsterdam",
            json!({
                "node": "pve",
                "gateway": "203.0.113.1",
                "ip_from": "203.0.113.10",
                "ip_to": "203.0.113.99",
                "private_bridge": "vmbr1",
                "images": [
                    { "template": 9000, "name": "Ubuntu 24.04" },
                    { "template": 9001, "name": "Debian 13" },
                    { "template": 9002, "name": "AlmaLinux 9" },
                    { "template": 9003, "name": "Rocky Linux 9" },
                ],
            }),
        )
        .await?;
    let registrar = seed
        .integration("openprovider", "Openprovider", json!({}))
        .await?;
    seed.attach(web, proxmox, "101").await?;

    let database = seed
        .subscription(
            customer,
            "D4VN7K",
            "active",
            "monthly",
            "automatic",
            12,
            ("VPS: Cloud M", "2 vCPU · 4 GB RAM · 40 GB opslag", 1250),
        )
        .await?;
    let db = seed
        .service(
            customer,
            cloud_m,
            database,
            "vps",
            "db-01",
            json!({ "hostname": "db-01.jansenbouw.nl", "ipv4": "203.0.113.11", "image": "Debian 13", "cores": 2, "memory_mb": 4096, "disk_gb": 40 }),
        )
        .await?;
    seed.attach(db, proxmox, "102").await?;

    let volume_product = seed
        .product(
            "volume",
            "Volume",
            "Extra opslag die je aan een server koppelt.",
            json!({ "per_gb": true }),
            10,
        )
        .await?;
    let address_product = seed
        .product(
            "floating_ip",
            "Floating IP",
            "Een extra IPv4-adres dat je tussen servers verplaatst.",
            json!({}),
            300,
        )
        .await?;
    sqlx::query(
        "UPDATE products SET integration_id = $1 WHERE kind IN ('vps', 'volume', 'floating_ip')",
    )
    .bind(proxmox)
    .execute(&mut *seed.transaction)
    .await?;

    let storage = seed
        .subscription(
            customer,
            "V8RS3T",
            "active",
            "monthly",
            "automatic",
            12,
            ("Volume: data-01", "50 GB", 500),
        )
        .await?;
    let volume = seed
        .service(
            customer,
            volume_product,
            storage,
            "volume",
            "data-01",
            json!({ "size_gb": 50, "server_id": db.to_string(), "host": "102" }),
        )
        .await?;
    seed.attach(volume, proxmox, "local-lvm:vm-102-disk-1")
        .await?;

    let address = seed
        .subscription(
            customer,
            "F2KP9W",
            "active",
            "monthly",
            "automatic",
            12,
            ("Floating IP", "203.0.113.50", 300),
        )
        .await?;
    let floating = seed
        .service(
            customer,
            address_product,
            address,
            "floating_ip",
            "203.0.113.50",
            json!({ "ipv4": "203.0.113.50", "server_id": web.to_string() }),
        )
        .await?;
    seed.attach(floating, proxmox, "203.0.113.50").await?;

    let network = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO vps_networks (id, user_id, integration_id, name, cidr, vlan, external_id, created_at)
         VALUES ($1, $2, $3, 'intern', '10.10.0.0/24', 101, '101', $4)",
    )
    .bind(network)
    .bind(customer)
    .bind(proxmox)
    .bind(seed.ago(24 * 30))
    .execute(&mut *seed.transaction)
    .await?;
    for (server, ip) in [(web, "10.10.0.2"), (db, "10.10.0.3")] {
        sqlx::query(
            "INSERT INTO vps_network_servers (network_id, service_id, address, port) VALUES ($1, $2, $3, 'net1')",
        )
        .bind(network)
        .bind(server)
        .bind(ip)
        .execute(&mut *seed.transaction)
        .await?;
    }

    let firewall = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO vps_firewalls (id, user_id, name, auto_assign, created_at) VALUES ($1, $2, 'Webservers', TRUE, $3)",
    )
    .bind(firewall)
    .bind(customer)
    .bind(seed.ago(24 * 30))
    .execute(&mut *seed.transaction)
    .await?;
    for (position, port, source, comment) in [
        (0, "22", Some("198.51.100.0/24"), "SSH vanaf kantoor"),
        (1, "80", None, "Website"),
        (2, "443", None, "Website"),
    ] {
        sqlx::query(
            "INSERT INTO vps_firewall_rules (id, firewall_id, position, direction, action, proto, dport, source, comment)
             VALUES ($1, $2, $3, 'in', 'ACCEPT', 'tcp', $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(firewall)
        .bind(position)
        .bind(port)
        .bind(source)
        .bind(comment)
        .execute(&mut *seed.transaction)
        .await?;
    }
    sqlx::query("INSERT INTO vps_firewall_servers (firewall_id, service_id) VALUES ($1, $2)")
        .bind(firewall)
        .bind(web)
        .execute(&mut *seed.transaction)
        .await?;

    for (name, url, days) in [
        (
            "Debian 13 netinst",
            "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-13-amd64-netinst.iso",
            6,
        ),
        (
            "SystemRescue",
            "https://fastly.example.org/systemrescue-11.03-amd64.iso",
            2,
        ),
    ] {
        sqlx::query(
            "INSERT INTO vps_isos (id, user_id, integration_id, name, url, firmware, status, external_id, last_used_at, created_at)
             VALUES ($1, $2, $3, $4, $5, 'bios', 'ready', $6, $7, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(customer)
        .bind(proxmox)
        .bind(name)
        .bind(url)
        .bind(format!("local:iso/{}.iso", name.to_lowercase().replace(' ', "-")))
        .bind(seed.ago(24 * days))
        .execute(&mut *seed.transaction)
        .await?;
    }

    for (index, (name, key)) in DEMO_KEYS.iter().enumerate() {
        let (public_key, fingerprint) = crate::routes::ssh_keys::parse(key)
            .ok_or_else(|| anyhow::anyhow!("demo ssh key {index} is invalid"))?;
        sqlx::query(
            "INSERT INTO ssh_keys (id, user_id, name, public_key, fingerprint, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(customer)
        .bind(name)
        .bind(public_key)
        .bind(fingerprint)
        .bind(seed.ago(24 * (60 - index as i64 * 20)))
        .execute(&mut *seed.transaction)
        .await?;
    }

    let colleague = seed
        .user("lisa@demo.stacklane.io", "Lisa", "Bakker", "customer", None)
        .await?;
    sqlx::query(
        "INSERT INTO vps_members (id, owner_id, user_id, permissions, created_at, accepted_at)
         VALUES ($1, $2, $3, $4, $5, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(customer)
    .bind(colleague)
    .bind(vec![
        "servers.view",
        "servers.start",
        "servers.stop",
        "servers.console",
        "snapshots.view",
        "snapshots.create",
        "firewalls.view",
    ])
    .bind(seed.ago(24 * 14))
    .execute(&mut *seed.transaction)
    .await?;

    let profile = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO whois_profiles (id, user_id, first_name, last_name, company, email, phone, street, house_number, zipcode, city, country, created_at)
         VALUES ($1, $2, 'Jan', 'Jansen', 'Jansen Bouw', 'jan@jansenbouw.nl', '+31.302345678', 'Stationsplein', '12', '3511ED', 'Utrecht', 'NL', $3)",
    )
    .bind(profile)
    .bind(customer)
    .bind(seed.ago(24 * 80))
    .execute(&mut *seed.transaction)
    .await?;
    for (extension, register, renew, transfer) in [
        ("nl", 799, 799, 0),
        ("com", 1199, 1399, 1199),
        ("be", 899, 899, 0),
        ("eu", 699, 899, 699),
        ("io", 3999, 4999, 3999),
    ] {
        sqlx::query(
            "INSERT INTO domain_extensions (id, integration_id, extension, register_cents, renew_cents, transfer_cents)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(registrar)
        .bind(extension)
        .bind(register)
        .bind(renew)
        .bind(transfer)
        .execute(&mut *seed.transaction)
        .await?;
    }
    for (name, code, days, external) in [
        ("jansenbouw.nl", "N5DM2Q", 160, "9100001"),
        ("jansenbouw.com", "N7CX4H", 40, "9100002"),
    ] {
        let subscription = seed
            .subscription(
                customer,
                code,
                "active",
                "yearly",
                "manual",
                days,
                ("Domein", name, 67),
            )
            .await?;
        let expires = (seed.ahead(days)).date().to_string();
        let domain = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO services (id, user_id, subscription_id, integration_id, kind, name, status, external_id, details, created_at)
             VALUES ($1, $2, $3, $4, 'domain', $5, 'active', $6, $7, $8)",
        )
        .bind(domain)
        .bind(customer)
        .bind(subscription)
        .bind(registrar)
        .bind(name)
        .bind(external)
        .bind(json!({
            "whois_profile_id": profile,
            "operation": "register",
            "expires_on": expires,
            "nameservers": ["ns1.openprovider.nl", "ns2.openprovider.be", "ns3.openprovider.eu"],
        }))
        .bind(seed.ago(24 * 200))
        .execute(&mut *seed.transaction)
        .await?;
    }

    for (vmid, (email, first, last, company, ticket)) in (110..).zip([
        (
            "info@fietsenmakerdewit.nl",
            "Eva",
            "de Wit",
            Some("Fietsenmaker de Wit"),
            "Mail komt niet aan bij Gmail",
        ),
        (
            "hallo@pixelstudio.nl",
            "Noah",
            "Visser",
            Some("Pixelstudio"),
            "Kunnen we upgraden naar Cloud L?",
        ),
        (
            "mila.peters@example.nl",
            "Mila",
            "Peters",
            None,
            "Hoe zet ik een modpack op mijn server?",
        ),
    ]) {
        let other = seed.user(email, first, last, "customer", company).await?;
        seed.ticket(
            other,
            "Algemeen",
            ticket,
            "open",
            &[(
                Some(other),
                false,
                6,
                "<p>Kunnen jullie hiernaar kijken? Alvast bedankt.</p>",
            )],
        )
        .await?;
        let code = format!(
            "{}{}",
            first[..1].to_uppercase(),
            last.replace(' ', "")[..3].to_uppercase()
        );
        let subscription = seed
            .subscription(
                other,
                &format!("{code}91"),
                "active",
                "monthly",
                "automatic",
                9,
                ("VPS: Cloud M", "2 vCPU · 4 GB RAM · 40 GB opslag", 1250),
            )
            .await?;
        let server = seed
            .service(
                other,
                cloud_m,
                subscription,
                "vps",
                &format!("{}-01", last.to_lowercase().replace(' ', "")),
                json!({ "ipv4": format!("203.0.113.{}", vmid - 90), "image": "Ubuntu 24.04", "cores": 2, "memory_mb": 4096, "disk_gb": 40 }),
            )
            .await?;
        seed.attach(server, proxmox, &vmid.to_string()).await?;
        seed.invoice(other, subscription, "paid", 21, "VPS: Cloud M", 1250)
            .await?;
    }

    for (recipient, subject, kind, status, error, hours) in [
        (
            CUSTOMER_EMAIL,
            "Antwoord op je ticket #1",
            "ticket_reply",
            "sent",
            None,
            26,
        ),
        (
            "info@fietsenmakerdewit.nl",
            "Nieuwe factuur 100004",
            "billing",
            "sent",
            None,
            30,
        ),
        (
            "hallo@pixelstudio.nl",
            "Bevestig je e-mailadres",
            "email_verification",
            "failed",
            Some("permanent error (550): mailbox unavailable"),
            50,
        ),
        (
            "mila.peters@example.nl",
            "Je wachtwoord opnieuw instellen",
            "password_reset",
            "sent",
            None,
            72,
        ),
    ] {
        sqlx::query(
            "INSERT INTO mail_log (id, recipient, subject, kind, status, error, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(recipient)
        .bind(subject)
        .bind(kind)
        .bind(status)
        .bind(error)
        .bind(seed.ago(hours))
        .execute(&mut *seed.transaction)
        .await?;
    }

    seed.notification(
        customer,
        "Lisa heeft toegang gekregen",
        "Lisa Bakker kan nu je servers bekijken, aan- en uitzetten en snapshots maken.",
        "/vps/users",
        24 * 14,
    )
    .await?;
    Ok(())
}

pub fn ensure_demo_mode(enabled: bool) -> anyhow::Result<()> {
    anyhow::ensure!(
        enabled,
        "demo-reset wipes all data and only runs with DEMO_MODE=true"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::next_full_hour;
    use time::macros::datetime;

    #[test]
    fn next_reset_is_the_next_full_hour() {
        assert_eq!(
            next_full_hour(datetime!(2026-11-24 07:42:13 UTC)),
            datetime!(2026-11-24 08:00:00 UTC)
        );
        assert_eq!(
            next_full_hour(datetime!(2026-11-24 23:59:59 UTC)),
            datetime!(2026-11-25 00:00:00 UTC)
        );
    }
}
