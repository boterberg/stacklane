use std::collections::HashMap;

use anyhow::Context;
use serde_json::json;
use sqlx::{MySqlPool, PgPool, Postgres, Transaction};
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use super::Report;
use super::text::{plain_to_html, slugify};
use crate::auth;

type Pg<'a> = Transaction<'a, Postgres>;

fn date(value: Option<String>) -> Option<Date> {
    Date::parse(
        value?.get(..10)?,
        format_description!("[year]-[month]-[day]"),
    )
    .ok()
}

fn datetime(value: Option<String>) -> Option<OffsetDateTime> {
    let value = value?;
    PrimitiveDateTime::parse(
        &value,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
    )
    .map(PrimitiveDateTime::assume_utc)
    .ok()
    .or_else(|| date(Some(value)).map(|date| date.midnight().assume_utc()))
}

pub fn interval(cycle: &str) -> (&'static str, i32) {
    match cycle {
        "Monthly" => ("monthly", 1),
        "Quarterly" => ("quarterly", 3),
        "Semi-Annually" => ("half_yearly", 6),
        "Annually" => ("yearly", 12),
        "Biennially" => ("yearly", 24),
        "Triennially" => ("yearly", 36),
        _ => ("monthly", 0),
    }
}

pub fn service_kind(product_type: &str, module: &str, name: &str) -> &'static str {
    let module = module.to_lowercase();
    let name = name.to_lowercase();
    if ["pterodactyl", "pelican", "multicraft", "gamecp"]
        .iter()
        .any(|game| module.contains(game))
        || name.contains("minecraft")
        || name.contains("gameserver")
    {
        "gameserver"
    } else if product_type == "server"
        || [
            "proxmox",
            "virtualizor",
            "virtfusion",
            "solusvm",
            "convoy",
            "hetzner",
            "openstack",
        ]
        .iter()
        .any(|platform| module.contains(platform))
    {
        "vps"
    } else if matches!(product_type, "hostingaccount" | "reselleraccount") {
        "webhosting"
    } else {
        "other"
    }
}

pub fn service_status(status: &str) -> (&'static str, &'static str) {
    match status {
        "Active" => ("active", "active"),
        "Suspended" => ("suspended", "suspended"),
        "Pending" => ("pending", "pending"),
        _ => ("cancelled", "terminated"),
    }
}

pub fn invoice_status(status: &str) -> &'static str {
    match status {
        "Paid" => "paid",
        "Cancelled" => "cancelled",
        "Refunded" => "refunded",
        _ => "open",
    }
}

pub fn ticket_status(status: &str) -> &'static str {
    match status {
        "Answered" => "answered",
        "Closed" => "closed",
        "In Progress" | "On Hold" => "in_progress",
        _ => "open",
    }
}

async fn has_table(source: &MySqlPool, table: &str) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
    )
    .bind(table)
    .fetch_one(source)
    .await?
        > 0)
}

async fn has_column(source: &MySqlPool, table: &str, column: &str) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ? AND column_name = ?",
    )
    .bind(table)
    .bind(column)
    .fetch_one(source)
    .await?
        > 0)
}

async fn existing_ref(
    pg: &mut Pg<'_>,
    table: &str,
    reference: &str,
) -> anyhow::Result<Option<Uuid>> {
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT id FROM {table} WHERE import_ref = $1"
    )))
    .bind(reference)
    .fetch_optional(&mut **pg)
    .await?)
}

type ClientRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);

async fn clients(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    report: &mut Report,
) -> anyhow::Result<HashMap<i64, Uuid>> {
    let password = if has_table(source, "tblusers").await?
        && has_table(source, "tblusers_clients").await?
    {
        "(SELECT CAST(u.password AS CHAR) FROM tblusers u JOIN tblusers_clients uc ON uc.auth_user_id = u.id
          WHERE uc.client_id = c.id AND uc.owner = 1 LIMIT 1)"
    } else if has_column(source, "tblclients", "password").await? {
        "CAST(c.password AS CHAR)"
    } else {
        "NULL"
    };
    let rows: Vec<ClientRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT CAST(c.id AS SIGNED), CAST(c.email AS CHAR), CAST(c.firstname AS CHAR), CAST(c.lastname AS CHAR),
                CAST(c.companyname AS CHAR), CAST(c.address1 AS CHAR), CAST(c.address2 AS CHAR), CAST(c.city AS CHAR),
                CAST(c.state AS CHAR), CAST(c.postcode AS CHAR), CAST(c.country AS CHAR), CAST(c.phonenumber AS CHAR),
                {password}
         FROM tblclients c ORDER BY c.id"
    )))
    .fetch_all(source)
    .await?;

    let mut map = HashMap::new();
    for (
        id,
        email,
        first,
        last,
        company,
        address1,
        address2,
        city,
        state,
        postcode,
        country,
        phone,
        hash,
    ) in rows
    {
        let email = email.trim().to_lowercase();
        if !email.contains('@') {
            report
                .skipped
                .push(format!("Klant {id} heeft geen geldig e-mailadres"));
            continue;
        }
        if let Some(existing) =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
                .bind(&email)
                .fetch_optional(&mut **pg)
                .await?
        {
            report.existing("Klanten");
            map.insert(id, existing);
            continue;
        }
        let password_hash = match hash.filter(|hash| hash.starts_with("$2")) {
            Some(hash) => hash,
            None => auth::hash_password(&hex::encode(rand::random::<[u8; 32]>()))?,
        };
        let user_id = Uuid::new_v4();
        let country = country.trim().to_uppercase();
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, first_name, last_name, company_name, is_business, phone,
                                country, address_line1, address_line2, city, postal_code, province, role)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'customer')",
        )
        .bind(user_id)
        .bind(&email)
        .bind(password_hash)
        .bind(first.trim())
        .bind(last.trim())
        .bind(Some(company.trim()).filter(|value| !value.is_empty()))
        .bind(!company.trim().is_empty())
        .bind(Some(phone.trim()).filter(|value| !value.is_empty()))
        .bind(if country.len() == 2 { country } else { "NL".into() })
        .bind(Some(address1.trim()).filter(|value| !value.is_empty()))
        .bind(Some(address2.trim()).filter(|value| !value.is_empty()))
        .bind(Some(city.trim()).filter(|value| !value.is_empty()))
        .bind(Some(postcode.trim()).filter(|value| !value.is_empty()))
        .bind(Some(state.trim()).filter(|value| !value.is_empty()))
        .execute(&mut **pg)
        .await?;
        report.created("Klanten");
        map.insert(id, user_id);
    }
    Ok(map)
}

type ProductRow = (
    i64,
    String,
    String,
    String,
    String,
    i64,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
);

struct ImportedProduct {
    id: Uuid,
    name: String,
    kind: &'static str,
}

async fn products(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    currency: i64,
    report: &mut Report,
) -> anyhow::Result<HashMap<i64, ImportedProduct>> {
    let retired = if has_column(source, "tblproducts", "retired").await? {
        "p.retired"
    } else {
        "0"
    };
    let rows: Vec<ProductRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT CAST(p.id AS SIGNED), CAST(p.type AS CHAR), CAST(p.name AS CHAR), CAST(COALESCE(p.description, '') AS CHAR),
                CAST(COALESCE(p.servertype, '') AS CHAR), CAST(p.hidden OR {retired} AS SIGNED),
                CAST(pr.monthly * 100 AS SIGNED), CAST(pr.quarterly * 100 AS SIGNED),
                CAST(pr.semiannually * 100 AS SIGNED), CAST(pr.annually * 100 AS SIGNED)
         FROM tblproducts p
         LEFT JOIN tblpricing pr ON pr.type = 'product' AND pr.relid = p.id AND pr.currency = ?
         ORDER BY p.id"
    )))
    .bind(currency)
    .fetch_all(source)
    .await?;

    let mut map = HashMap::new();
    for (
        id,
        product_type,
        name,
        description,
        module,
        hidden,
        monthly,
        quarterly,
        semiannually,
        annually,
    ) in rows
    {
        let kind = service_kind(&product_type, &module, &name);
        let reference = format!("whmcs:product:{id}");
        if let Some(existing) = existing_ref(pg, "products", &reference).await? {
            report.existing("Producten");
            map.insert(
                id,
                ImportedProduct {
                    id: existing,
                    name,
                    kind,
                },
            );
            continue;
        }
        let monthly_price = [
            (monthly, 1),
            (quarterly, 3),
            (semiannually, 6),
            (annually, 12),
        ]
        .into_iter()
        .find_map(|(price, months)| {
            price
                .filter(|price| *price >= 0)
                .map(|price| price / months)
        })
        .unwrap_or(0);
        let mut slug = slugify(&name);
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM products WHERE slug = $1)")
                .bind(&slug)
                .fetch_one(&mut **pg)
                .await?;
        if taken || slug.is_empty() {
            slug = format!("{slug}-whmcs-{id}")
                .trim_start_matches('-')
                .to_owned();
        }
        let product_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO products (id, kind, slug, name, description, monthly_price_cents, is_active, import_ref)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(product_id)
        .bind(kind)
        .bind(&slug)
        .bind(name.trim())
        .bind(Some(description.trim()).filter(|value| !value.is_empty()))
        .bind(i32::try_from(monthly_price).unwrap_or(i32::MAX))
        .bind(hidden == 0)
        .bind(&reference)
        .execute(&mut **pg)
        .await?;
        report.created("Producten");
        map.insert(
            id,
            ImportedProduct {
                id: product_id,
                name,
                kind,
            },
        );
    }
    Ok(map)
}

type HostingRow = (
    i64,
    i64,
    i64,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
);

async fn services(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    clients: &HashMap<i64, Uuid>,
    products: &HashMap<i64, ImportedProduct>,
    report: &mut Report,
) -> anyhow::Result<HashMap<i64, Uuid>> {
    let rows: Vec<HostingRow> = sqlx::query_as(
        "SELECT CAST(h.id AS SIGNED), CAST(h.userid AS SIGNED), CAST(h.packageid AS SIGNED), CAST(COALESCE(h.domain, '') AS CHAR),
                CAST(h.billingcycle AS CHAR), CAST(h.amount * 100 AS SIGNED), CAST(h.domainstatus AS CHAR),
                IF(h.nextduedate < '1971-01-01', NULL, DATE_FORMAT(h.nextduedate, '%Y-%m-%d')),
                IF(h.regdate < '1971-01-01', NULL, DATE_FORMAT(h.regdate, '%Y-%m-%d')),
                CAST(COALESCE(h.username, '') AS CHAR), CAST(COALESCE(h.dedicatedip, '') AS CHAR)
         FROM tblhosting h ORDER BY h.id",
    )
    .fetch_all(source)
    .await?;

    let now = OffsetDateTime::now_utc();
    let mut map = HashMap::new();
    let mut long_cycles = 0;
    for (id, client, package, domain, cycle, amount, status, next_due, registered, username, ip) in
        rows
    {
        let reference = format!("whmcs:hosting:{id}");
        if let Some(existing) = existing_ref(pg, "subscriptions", &reference).await? {
            report.existing("Abonnementen");
            map.insert(id, existing);
            continue;
        }
        let Some(user_id) = clients.get(&client).copied() else {
            report.skipped.push(format!(
                "Dienst {id} hoort bij een klant die niet is overgenomen"
            ));
            continue;
        };
        let product = products.get(&package);
        let (interval, months) = interval(&cycle);
        if months > 12 {
            long_cycles += 1;
        }
        let (subscription_status, service_status) = service_status(&status);
        let started_at = datetime(registered).unwrap_or(now);
        let expires_at = datetime(next_due).unwrap_or(now);
        let product_name = product
            .map(|product| product.name.clone())
            .unwrap_or_else(|| format!("Product {package}"));
        let subscription_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, payment_mode, started_at, expires_at,
                                        cancelled_at, import_ref)
             VALUES ($1, $2, $3, $4, $5, 'manual', $6, $7, CASE WHEN $4 = 'cancelled' THEN $7 END, $8)",
        )
        .bind(subscription_id)
        .bind(format!("WH{id}"))
        .bind(user_id)
        .bind(subscription_status)
        .bind(interval)
        .bind(started_at)
        .bind(expires_at)
        .bind(&reference)
        .execute(&mut **pg)
        .await?;
        sqlx::query(
            "INSERT INTO subscription_items (id, subscription_id, product_id, description, specs_line, monthly_price_cents)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(subscription_id)
        .bind(product.map(|product| product.id))
        .bind(&product_name)
        .bind(Some(domain.trim()).filter(|value| !value.is_empty()))
        .bind(if months > 0 { i32::try_from(amount / i64::from(months)).unwrap_or(0) } else { 0 })
        .execute(&mut **pg)
        .await?;
        sqlx::query(
            "INSERT INTO services (id, user_id, product_id, subscription_id, kind, name, status, details, import_ref)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(product.map(|product| product.id))
        .bind(subscription_id)
        .bind(product.map(|product| product.kind).unwrap_or("other"))
        .bind(if domain.trim().is_empty() { product_name.clone() } else { domain.trim().to_owned() })
        .bind(service_status)
        .bind(json!({
            "imported_from": "whmcs",
            "whmcs_id": id,
            "domain": Some(domain.trim()).filter(|value| !value.is_empty()),
            "username": Some(username.trim()).filter(|value| !value.is_empty()),
            "address": Some(ip.trim()).filter(|value| !value.is_empty()),
        }))
        .bind(&reference)
        .execute(&mut **pg)
        .await?;
        report.created("Abonnementen");
        map.insert(id, subscription_id);
    }
    if long_cycles > 0 {
        report.notes.push(format!(
            "{long_cycles} diensten hebben een cyclus van twee of drie jaar; die staan in Stacklane als jaarlijks"
        ));
    }
    Ok(map)
}

type InvoiceRow = (
    i64,
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    i64,
    String,
);
type InvoiceItemRow = (String, i64, String, i64);

async fn invoices(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    clients: &HashMap<i64, Uuid>,
    subscriptions: &HashMap<i64, Uuid>,
    report: &mut Report,
) -> anyhow::Result<()> {
    let rows: Vec<InvoiceRow> = sqlx::query_as(
        "SELECT CAST(i.id AS SIGNED), CAST(i.userid AS SIGNED), CAST(COALESCE(i.invoicenum, '') AS CHAR),
                IF(i.date < '1971-01-01', NULL, DATE_FORMAT(i.date, '%Y-%m-%d')),
                IF(i.duedate < '1971-01-01', NULL, DATE_FORMAT(i.duedate, '%Y-%m-%d')),
                IF(i.datepaid < '1971-01-01', NULL, DATE_FORMAT(i.datepaid, '%Y-%m-%d %H:%i:%s')),
                CAST(i.total * 100 AS SIGNED), CAST((i.tax + i.tax2) * 100 AS SIGNED), CAST(i.status AS CHAR)
         FROM tblinvoices i ORDER BY i.id",
    )
    .fetch_all(source)
    .await?;

    let today = OffsetDateTime::now_utc().date();
    for (id, client, number, issued, due, paid, total, tax, status) in rows {
        let reference = format!("whmcs:invoice:{id}");
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM invoices WHERE import_ref = $1)")
                .bind(&reference)
                .fetch_one(&mut **pg)
                .await?;
        if exists {
            report.existing("Facturen");
            continue;
        }
        let Some(user_id) = clients.get(&client).copied() else {
            report.skipped.push(format!(
                "Factuur {id} hoort bij een klant die niet is overgenomen"
            ));
            continue;
        };
        let id_free: bool =
            sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM invoices WHERE id = $1)")
                .bind(id)
                .fetch_one(&mut **pg)
                .await?;
        let issued_at = date(issued).unwrap_or(today);
        let snapshot = json!({ "imported_from": "whmcs", "whmcs_invoice_id": id, "whmcs_number": Some(number).filter(|value| !value.is_empty()) });
        let insert = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "INSERT INTO invoices ({}user_id, status, issued_at, due_at, paid_at, total_cents, tax_cents, billing_snapshot, import_ref)
             VALUES ({}$1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
            if id_free { "id, " } else { "" },
            if id_free { "$10, " } else { "" },
        )))
        .bind(user_id)
        .bind(invoice_status(&status))
        .bind(issued_at)
        .bind(date(due).unwrap_or(issued_at))
        .bind(datetime(paid))
        .bind(i32::try_from(total).unwrap_or(i32::MAX))
        .bind(i32::try_from(tax).unwrap_or(0))
        .bind(snapshot)
        .bind(&reference);
        let invoice_id: i64 = if id_free { insert.bind(id) } else { insert }
            .fetch_one(&mut **pg)
            .await?;

        let items: Vec<InvoiceItemRow> = sqlx::query_as(
            "SELECT CAST(COALESCE(type, '') AS CHAR), CAST(relid AS SIGNED), CAST(description AS CHAR), CAST(amount * 100 AS SIGNED)
             FROM tblinvoiceitems WHERE invoiceid = ? ORDER BY id",
        )
        .bind(id)
        .fetch_all(source)
        .await?;
        for (item_type, relid, description, amount) in items {
            sqlx::query(
                "INSERT INTO invoice_lines (id, invoice_id, subscription_id, description, amount_cents) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(Uuid::new_v4())
            .bind(invoice_id)
            .bind(if item_type == "Hosting" { subscriptions.get(&relid).copied() } else { None })
            .bind(description.chars().take(500).collect::<String>())
            .bind(i32::try_from(amount).unwrap_or(0))
            .execute(&mut **pg)
            .await?;
        }
        report.created("Facturen");
    }
    sqlx::query("SELECT setval('invoices_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM invoices), 100000))")
        .execute(&mut **pg)
        .await?;
    Ok(())
}

type TicketRow = (
    i64,
    i64,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);
type ReplyRow = (i64, String, String, Option<String>);

async fn tickets(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    clients: &HashMap<i64, Uuid>,
    report: &mut Report,
) -> anyhow::Result<()> {
    let departments: HashMap<i64, String> = sqlx::query_as::<_, (i64, String)>(
        "SELECT CAST(id AS SIGNED), CAST(name AS CHAR) FROM tblticketdepartments",
    )
    .fetch_all(source)
    .await?
    .into_iter()
    .collect();

    let rows: Vec<TicketRow> = sqlx::query_as(
        "SELECT CAST(t.id AS SIGNED), CAST(t.userid AS SIGNED), CAST(COALESCE(t.email, '') AS CHAR),
                CAST(t.title AS CHAR), CAST(t.message AS CHAR), CAST(t.status AS CHAR), CAST(t.did AS CHAR),
                IF(t.date < '1971-01-01', NULL, DATE_FORMAT(t.date, '%Y-%m-%d %H:%i:%s')),
                IF(t.lastreply < '1971-01-01', NULL, DATE_FORMAT(t.lastreply, '%Y-%m-%d %H:%i:%s'))
         FROM tbltickets t ORDER BY t.id",
    )
    .fetch_all(source)
    .await?;

    let now = OffsetDateTime::now_utc();
    for (id, client, email, title, message, status, department, created, last_reply) in rows {
        let reference = format!("whmcs:ticket:{id}");
        if existing_ref(pg, "tickets", &reference).await?.is_some() {
            report.existing("Tickets");
            continue;
        }
        let user_id = match clients.get(&client).copied() {
            Some(user_id) => Some(user_id),
            None => {
                sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
                    .bind(email.trim().to_lowercase())
                    .fetch_optional(&mut **pg)
                    .await?
            }
        };
        let Some(user_id) = user_id else {
            report
                .skipped
                .push(format!("Ticket {id} is van een gast zonder account"));
            continue;
        };
        let department_name = department
            .parse::<i64>()
            .ok()
            .and_then(|key| departments.get(&key).cloned())
            .unwrap_or_else(|| "Algemeen".to_owned());
        let department_id: Uuid =
            match sqlx::query_scalar("SELECT id FROM departments WHERE name = $1")
                .bind(&department_name)
                .fetch_optional(&mut **pg)
                .await?
            {
                Some(existing) => existing,
                None => {
                    let created_id = Uuid::new_v4();
                    sqlx::query("INSERT INTO departments (id, name, position) VALUES ($1, $2, 99)")
                        .bind(created_id)
                        .bind(&department_name)
                        .execute(&mut **pg)
                        .await?;
                    created_id
                }
            };
        let created_at = datetime(created).unwrap_or(now);
        let ticket_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tickets (id, user_id, department_id, subject, status, created_at, last_reply_at, import_ref)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(ticket_id)
        .bind(user_id)
        .bind(department_id)
        .bind(title.trim().chars().take(150).collect::<String>())
        .bind(ticket_status(&status))
        .bind(created_at)
        .bind(datetime(last_reply).unwrap_or(created_at))
        .bind(&reference)
        .execute(&mut **pg)
        .await?;
        sqlx::query(
            "INSERT INTO ticket_messages (id, ticket_id, author_user_id, is_staff, body_html, created_at) VALUES ($1, $2, $3, FALSE, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(ticket_id)
        .bind(user_id)
        .bind(plain_to_html(&message))
        .bind(created_at)
        .execute(&mut **pg)
        .await?;

        let replies: Vec<ReplyRow> = sqlx::query_as(
            "SELECT CAST(userid AS SIGNED), CAST(COALESCE(admin, '') AS CHAR), CAST(message AS CHAR),
                    IF(date < '1971-01-01', NULL, DATE_FORMAT(date, '%Y-%m-%d %H:%i:%s'))
             FROM tblticketreplies WHERE tid = ? ORDER BY date, id",
        )
        .bind(id)
        .fetch_all(source)
        .await?;
        for (reply_client, admin, body, date) in replies {
            let is_staff = !admin.trim().is_empty();
            sqlx::query(
                "INSERT INTO ticket_messages (id, ticket_id, author_user_id, is_staff, body_html, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(ticket_id)
            .bind(if is_staff { None } else { clients.get(&reply_client).copied().or(Some(user_id)) })
            .bind(is_staff)
            .bind(plain_to_html(&body))
            .bind(datetime(date).unwrap_or(created_at))
            .execute(&mut **pg)
            .await?;
        }
        report.created("Tickets");
    }
    Ok(())
}

pub async fn run(source: &MySqlPool, target: &PgPool, dry_run: bool) -> anyhow::Result<Report> {
    let mut report = Report::default();
    let (currency_id, currency_code): (i64, String) = sqlx::query_as(
        "SELECT CAST(id AS SIGNED), CAST(code AS CHAR) FROM tblcurrencies ORDER BY `default` DESC, id LIMIT 1",
    )
    .fetch_one(source)
    .await
    .context("dit lijkt geen WHMCS-database: tblcurrencies ontbreekt")?;
    if currency_code.to_uppercase() != "EUR" {
        report.notes.push(format!(
            "De standaardvaluta in WHMCS is {currency_code}; bedragen zijn overgenomen zonder omrekening"
        ));
    }

    let mut pg = target.begin().await?;
    let clients = clients(source, &mut pg, &mut report).await?;
    let products = products(source, &mut pg, currency_id, &mut report).await?;
    let subscriptions = services(source, &mut pg, &clients, &products, &mut report).await?;
    invoices(source, &mut pg, &clients, &subscriptions, &mut report).await?;
    tickets(source, &mut pg, &clients, &mut report).await?;

    let payments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tblaccounts")
        .fetch_one(source)
        .await
        .unwrap_or(0);
    if payments > 0 {
        report.notes.push(format!(
            "{payments} betalingen zijn niet overgenomen; Stacklane bewaart betalingen nog niet apart. De status van elke factuur is wel overgenomen."
        ));
    }

    if dry_run {
        pg.rollback().await?;
    } else {
        pg.commit().await?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_whmcs_statuses() {
        assert_eq!(service_status("Active"), ("active", "active"));
        assert_eq!(service_status("Fraud"), ("cancelled", "terminated"));
        assert_eq!(invoice_status("Unpaid"), "open");
        assert_eq!(invoice_status("Refunded"), "refunded");
        assert_eq!(ticket_status("Customer-Reply"), "open");
        assert_eq!(ticket_status("On Hold"), "in_progress");
    }

    #[test]
    fn maps_billing_cycles() {
        assert_eq!(interval("Semi-Annually"), ("half_yearly", 6));
        assert_eq!(interval("Triennially"), ("yearly", 36));
        assert_eq!(interval("Free Account"), ("monthly", 0));
    }

    #[test]
    fn guesses_the_service_kind() {
        assert_eq!(
            service_kind("other", "pterodactyl", "Paper 4GB"),
            "gameserver"
        );
        assert_eq!(service_kind("server", "", "VPS M"), "vps");
        assert_eq!(service_kind("other", "proxmoxVE", "Cloud"), "vps");
        assert_eq!(
            service_kind("hostingaccount", "cpanel", "Web Plus"),
            "webhosting"
        );
        assert_eq!(service_kind("other", "", "SSL certificaat"), "other");
    }

    #[test]
    fn parses_mysql_dates() {
        assert_eq!(
            date(Some("2024-05-01".into())).unwrap().to_string(),
            "2024-05-01"
        );
        assert!(date(None).is_none());
        assert_eq!(
            datetime(Some("2024-05-01 13:45:00".into())).unwrap().hour(),
            13
        );
    }
}
