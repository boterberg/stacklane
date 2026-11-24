use std::collections::HashMap;

use anyhow::Context;
use serde_json::json;
use sqlx::{MySqlPool, PgPool, Postgres, Transaction};
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use super::Report;
use super::text::{plain_to_html, slugify};
use super::whmcs::service_kind;
use crate::auth;

type Pg<'a> = Transaction<'a, Postgres>;

fn datetime(value: Option<String>) -> Option<OffsetDateTime> {
    let value = value?;
    PrimitiveDateTime::parse(
        &value,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
    )
    .map(PrimitiveDateTime::assume_utc)
    .ok()
    .or_else(|| {
        Date::parse(
            value.get(..10)?,
            format_description!("[year]-[month]-[day]"),
        )
        .ok()
        .map(|date| date.midnight().assume_utc())
    })
}

pub fn billing_interval(unit: &str, period: i64) -> (&'static str, i64) {
    let months = match unit {
        "year" => period * 12,
        "month" => period,
        _ => 1,
    };
    let interval = match months {
        3 => "quarterly",
        6 => "half_yearly",
        12.. => "yearly",
        _ => "monthly",
    };
    (interval, months.max(1))
}

fn service_status(status: &str) -> (&'static str, &'static str) {
    match status {
        "active" => ("active", "active"),
        "suspended" => ("suspended", "suspended"),
        "pending" => ("pending", "pending"),
        _ => ("cancelled", "terminated"),
    }
}

fn invoice_status(status: &str) -> &'static str {
    match status {
        "paid" => "paid",
        "cancelled" => "cancelled",
        _ => "open",
    }
}

fn ticket_status(status: &str) -> &'static str {
    match status {
        "replied" => "answered",
        "closed" => "closed",
        _ => "open",
    }
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

async fn user_properties(
    source: &MySqlPool,
) -> anyhow::Result<HashMap<i64, HashMap<String, String>>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT CAST(model_id AS SIGNED), LOWER(CAST(`key` AS CHAR)), CAST(value AS CHAR)
         FROM properties WHERE model_type LIKE '%User'",
    )
    .fetch_all(source)
    .await
    .unwrap_or_default();
    let mut map: HashMap<i64, HashMap<String, String>> = HashMap::new();
    for (user, key, value) in rows {
        map.entry(user).or_default().insert(key, value);
    }
    Ok(map)
}

fn property<'a>(properties: Option<&'a HashMap<String, String>>, keys: &[&str]) -> Option<&'a str> {
    let properties = properties?;
    keys.iter()
        .find_map(|key| properties.get(*key))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

async fn users(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    report: &mut Report,
) -> anyhow::Result<(HashMap<i64, Uuid>, std::collections::HashSet<i64>)> {
    let properties = user_properties(source).await?;
    let rows: Vec<(i64, String, String, String, String, i64)> = sqlx::query_as(
        "SELECT CAST(id AS SIGNED), CAST(email AS CHAR), CAST(COALESCE(first_name, '') AS CHAR), CAST(COALESCE(last_name, '') AS CHAR),
                CAST(password AS CHAR), CAST(role_id IS NOT NULL AS SIGNED)
         FROM users ORDER BY id",
    )
    .fetch_all(source)
    .await?;

    let mut map = HashMap::new();
    let mut staff = std::collections::HashSet::new();
    for (id, email, first, last, hash, is_admin) in rows {
        if is_admin == 1 {
            staff.insert(id);
            continue;
        }
        let email = email.trim().to_lowercase();
        match super::matching_account(&mut **pg, &email).await? {
            super::Account::Usable(existing) => {
                report.existing("Klanten");
                map.insert(id, existing);
                continue;
            }
            super::Account::Unconfirmed => {
                report.unconfirmed(&email);
                continue;
            }
            super::Account::Missing => {}
        }
        let user_properties = properties.get(&id);
        let company = property(user_properties, &["company_name", "company"]);
        let country = property(user_properties, &["country"])
            .map(|country| country.to_uppercase())
            .filter(|country| country.len() == 2)
            .unwrap_or_else(|| "NL".into());
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, first_name, last_name, company_name, is_business, phone, country,
                                address_line1, address_line2, city, postal_code, province, role, imported_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'customer', now())",
        )
        .bind(user_id)
        .bind(&email)
        .bind(if hash.starts_with("$2") { hash } else { auth::hash_password(&hex::encode(rand::random::<[u8; 32]>()))? })
        .bind(first.trim())
        .bind(last.trim())
        .bind(company)
        .bind(company.is_some())
        .bind(property(user_properties, &["phone"]))
        .bind(country)
        .bind(property(user_properties, &["address", "address1"]))
        .bind(property(user_properties, &["address2"]))
        .bind(property(user_properties, &["city"]))
        .bind(property(user_properties, &["zip", "postcode", "postal_code"]))
        .bind(property(user_properties, &["state", "province"]))
        .execute(&mut **pg)
        .await?;
        report.created("Klanten");
        map.insert(id, user_id);
    }
    Ok((map, staff))
}

struct ImportedProduct {
    id: Uuid,
    name: String,
    kind: &'static str,
}

async fn products(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    report: &mut Report,
) -> anyhow::Result<HashMap<i64, ImportedProduct>> {
    let rows: Vec<(i64, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT CAST(p.id AS SIGNED), CAST(p.name AS CHAR), CAST(COALESCE(p.description, '') AS CHAR),
                CAST(COALESCE(e.extension, '') AS CHAR),
                (SELECT CAST(MIN(pr.price / IF(pl.billing_unit = 'year', 12 * COALESCE(pl.billing_period, 1), COALESCE(pl.billing_period, 1)) * 100) AS SIGNED)
                 FROM plans pl JOIN prices pr ON pr.plan_id = pl.id
                 WHERE pl.priceable_id = p.id AND pl.priceable_type LIKE '%Product' AND pl.type = 'recurring')
         FROM products p LEFT JOIN extensions e ON e.id = p.server_id ORDER BY p.id",
    )
    .fetch_all(source)
    .await?;

    let mut map = HashMap::new();
    for (id, name, description, extension, monthly) in rows {
        let kind = service_kind("other", &extension, &name);
        let reference = format!("paymenter:product:{id}");
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
        let mut slug = slugify(&name);
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM products WHERE slug = $1)")
                .bind(&slug)
                .fetch_one(&mut **pg)
                .await?;
        if taken || slug.is_empty() {
            slug = format!("{slug}-paymenter-{id}")
                .trim_start_matches('-')
                .to_owned();
        }
        let product_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO products (id, kind, slug, name, description, monthly_price_cents, import_ref) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(product_id)
        .bind(kind)
        .bind(&slug)
        .bind(name.trim())
        .bind(Some(description.trim()).filter(|value| !value.is_empty()))
        .bind(i32::try_from(monthly.unwrap_or(0)).unwrap_or(0))
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

type ServiceRow = (
    i64,
    i64,
    Option<i64>,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    String,
    String,
);

async fn services(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    users: &HashMap<i64, Uuid>,
    products: &HashMap<i64, ImportedProduct>,
    report: &mut Report,
) -> anyhow::Result<HashMap<i64, Uuid>> {
    let label = if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'services' AND column_name = 'label'",
    )
    .fetch_one(source)
    .await?
        > 0
    {
        "CAST(COALESCE(s.label, '') AS CHAR)"
    } else {
        "''"
    };
    let rows: Vec<ServiceRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT CAST(s.id AS SIGNED), CAST(s.user_id AS SIGNED), CAST(s.product_id AS SIGNED), CAST(s.status AS CHAR),
                CAST(s.price * s.quantity * 100 AS SIGNED), CAST(COALESCE(pl.billing_unit, 'month') AS CHAR),
                DATE_FORMAT(s.expires_at, '%Y-%m-%d %H:%i:%s'), DATE_FORMAT(s.created_at, '%Y-%m-%d %H:%i:%s'),
                CAST(pl.billing_period AS SIGNED), {label}, CAST(s.currency_code AS CHAR)
         FROM services s LEFT JOIN plans pl ON pl.id = s.plan_id ORDER BY s.id"
    )))
    .fetch_all(source)
    .await?;

    let now = OffsetDateTime::now_utc();
    let mut map = HashMap::new();
    let mut currencies = std::collections::BTreeSet::new();
    for (id, user, product_id, status, amount, unit, expires, created, period, label, currency) in
        rows
    {
        currencies.insert(currency);
        let reference = format!("paymenter:service:{id}");
        if let Some(existing) = existing_ref(pg, "subscriptions", &reference).await? {
            report.existing("Abonnementen");
            map.insert(id, existing);
            continue;
        }
        let Some(user_id) = users.get(&user).copied() else {
            report.skipped.push(format!(
                "Dienst {id} hoort bij een klant die niet is overgenomen"
            ));
            continue;
        };
        let product = product_id.and_then(|product_id| products.get(&product_id));
        let (interval, months) = billing_interval(&unit, period.unwrap_or(1));
        let (subscription_status, service_status) = service_status(&status);
        let created_at = datetime(created).unwrap_or(now);
        let expires_at = datetime(expires).unwrap_or(now);
        let name = if label.trim().is_empty() {
            product
                .map(|product| product.name.clone())
                .unwrap_or_else(|| format!("Dienst {id}"))
        } else {
            label.trim().to_owned()
        };
        let subscription_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, payment_mode, started_at, expires_at, cancelled_at, import_ref)
             VALUES ($1, $2, $3, $4, $5, 'manual', $6, $7, CASE WHEN $4 = 'cancelled' THEN $7 END, $8)",
        )
        .bind(subscription_id)
        .bind(format!("PM{id}"))
        .bind(user_id)
        .bind(subscription_status)
        .bind(interval)
        .bind(created_at)
        .bind(expires_at)
        .bind(&reference)
        .execute(&mut **pg)
        .await?;
        sqlx::query(
            "INSERT INTO subscription_items (id, subscription_id, product_id, description, monthly_price_cents) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(subscription_id)
        .bind(product.map(|product| product.id))
        .bind(product.map(|product| product.name.clone()).unwrap_or_else(|| name.clone()))
        .bind(i32::try_from(amount / months).unwrap_or(0))
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
        .bind(&name)
        .bind(service_status)
        .bind(json!({ "imported_from": "paymenter", "paymenter_id": id }))
        .bind(&reference)
        .execute(&mut **pg)
        .await?;
        report.created("Abonnementen");
        map.insert(id, subscription_id);
    }
    let foreign: Vec<String> = currencies
        .into_iter()
        .filter(|code| code.to_uppercase() != "EUR")
        .collect();
    if !foreign.is_empty() {
        report.notes.push(format!(
            "Er zijn diensten in {}; bedragen zijn overgenomen zonder omrekening",
            foreign.join(", ")
        ));
    }
    Ok(map)
}

type InvoiceRow = (
    i64,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
);

async fn invoices(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    users: &HashMap<i64, Uuid>,
    subscriptions: &HashMap<i64, Uuid>,
    report: &mut Report,
) -> anyhow::Result<()> {
    let rows: Vec<InvoiceRow> = sqlx::query_as(
        "SELECT CAST(i.id AS SIGNED), CAST(COALESCE(i.number, '') AS CHAR), CAST(i.user_id AS SIGNED), CAST(i.status AS CHAR),
                DATE_FORMAT(i.created_at, '%Y-%m-%d %H:%i:%s'), DATE_FORMAT(i.due_at, '%Y-%m-%d'),
                (SELECT CAST(SUM(it.price * it.quantity) * 100 AS SIGNED) FROM invoice_items it WHERE it.invoice_id = i.id)
         FROM invoices i ORDER BY i.id",
    )
    .fetch_all(source)
    .await?;

    let now = OffsetDateTime::now_utc();
    for (id, number, user, status, created, due, total) in rows {
        let reference = format!("paymenter:invoice:{id}");
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM invoices WHERE import_ref = $1)")
                .bind(&reference)
                .fetch_one(&mut **pg)
                .await?;
        if exists {
            report.existing("Facturen");
            continue;
        }
        if status == "draft" {
            continue;
        }
        let Some(user_id) = users.get(&user).copied() else {
            report.skipped.push(format!(
                "Factuur {id} hoort bij een klant die niet is overgenomen"
            ));
            continue;
        };
        let issued_at = datetime(created).unwrap_or(now);
        let due_at = datetime(due).unwrap_or(issued_at);
        let invoice_id: i64 = sqlx::query_scalar(
            "INSERT INTO invoices (user_id, status, issued_at, due_at, paid_at, total_cents, tax_cents, billing_snapshot, import_ref)
             VALUES ($1, $2, $3, $4, CASE WHEN $2 = 'paid' THEN $5::timestamptz END, $6, 0, $7, $8) RETURNING id",
        )
        .bind(user_id)
        .bind(invoice_status(&status))
        .bind(issued_at.date())
        .bind(due_at.date())
        .bind(issued_at)
        .bind(i32::try_from(total.unwrap_or(0)).unwrap_or(0))
        .bind(json!({ "imported_from": "paymenter", "paymenter_invoice_id": id, "paymenter_number": Some(number).filter(|value| !value.is_empty()) }))
        .bind(&reference)
        .fetch_one(&mut **pg)
        .await?;

        let items: Vec<(String, i64, String, Option<i64>)> = sqlx::query_as(
            "SELECT CAST(COALESCE(description, '') AS CHAR), CAST(price * quantity * 100 AS SIGNED),
                    CAST(COALESCE(reference_type, '') AS CHAR), CAST(reference_id AS SIGNED)
             FROM invoice_items WHERE invoice_id = ? ORDER BY id",
        )
        .bind(id)
        .fetch_all(source)
        .await?;
        for (description, amount, reference_type, reference_id) in items {
            let subscription = if reference_type.ends_with("Service") {
                reference_id.and_then(|service| subscriptions.get(&service).copied())
            } else {
                None
            };
            sqlx::query("INSERT INTO invoice_lines (id, invoice_id, subscription_id, description, amount_cents) VALUES ($1, $2, $3, $4, $5)")
                .bind(Uuid::new_v4())
                .bind(invoice_id)
                .bind(subscription)
                .bind(if description.trim().is_empty() { "Regel".to_owned() } else { description.chars().take(500).collect() })
                .bind(i32::try_from(amount).unwrap_or(0))
                .execute(&mut **pg)
                .await?;
        }
        report.created("Facturen");
    }
    Ok(())
}

type TicketRow = (
    i64,
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

async fn tickets(
    source: &MySqlPool,
    pg: &mut Pg<'_>,
    users: &HashMap<i64, Uuid>,
    staff: &std::collections::HashSet<i64>,
    report: &mut Report,
) -> anyhow::Result<()> {
    let rows: Vec<TicketRow> = sqlx::query_as(
        "SELECT CAST(id AS SIGNED), CAST(user_id AS SIGNED), CAST(subject AS CHAR), CAST(status AS CHAR),
                CAST(COALESCE(department, 'Algemeen') AS CHAR),
                DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s'), DATE_FORMAT(updated_at, '%Y-%m-%d %H:%i:%s')
         FROM tickets ORDER BY id",
    )
    .fetch_all(source)
    .await?;

    let now = OffsetDateTime::now_utc();
    for (id, user, subject, status, department, created, updated) in rows {
        let reference = format!("paymenter:ticket:{id}");
        if existing_ref(pg, "tickets", &reference).await?.is_some() {
            report.existing("Tickets");
            continue;
        }
        let Some(user_id) = users.get(&user).copied() else {
            report.skipped.push(format!(
                "Ticket {id} hoort bij een klant die niet is overgenomen"
            ));
            continue;
        };
        let department = if department.trim().is_empty() {
            "Algemeen".to_owned()
        } else {
            department.trim().to_owned()
        };
        let department_id: Uuid =
            match sqlx::query_scalar("SELECT id FROM departments WHERE name = $1")
                .bind(&department)
                .fetch_optional(&mut **pg)
                .await?
            {
                Some(existing) => existing,
                None => {
                    let created_id = Uuid::new_v4();
                    sqlx::query("INSERT INTO departments (id, name, position) VALUES ($1, $2, 99)")
                        .bind(created_id)
                        .bind(&department)
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
        .bind(subject.trim().chars().take(150).collect::<String>())
        .bind(ticket_status(&status))
        .bind(created_at)
        .bind(datetime(updated).unwrap_or(created_at))
        .bind(&reference)
        .execute(&mut **pg)
        .await?;

        let messages: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT CAST(user_id AS SIGNED), CAST(message AS CHAR), DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')
             FROM ticket_messages WHERE ticket_id = ? ORDER BY created_at, id",
        )
        .bind(id)
        .fetch_all(source)
        .await?;
        for (author, message, date) in messages {
            let is_staff = staff.contains(&author);
            sqlx::query(
                "INSERT INTO ticket_messages (id, ticket_id, author_user_id, is_staff, body_html, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(ticket_id)
            .bind(if is_staff { None } else { users.get(&author).copied() })
            .bind(is_staff)
            .bind(plain_to_html(&message))
            .bind(datetime(date).unwrap_or(created_at))
            .execute(&mut **pg)
            .await?;
        }
        report.created("Tickets");
    }
    Ok(())
}

pub async fn run(source: &MySqlPool, target: &PgPool, dry_run: bool) -> anyhow::Result<Report> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM plans")
        .fetch_one(source)
        .await
        .context("dit lijkt geen Paymenter-database: de tabel plans ontbreekt")?;
    let mut report = Report::default();
    let mut pg = target.begin().await?;
    let (users, staff) = users(source, &mut pg, &mut report).await?;
    let products = products(source, &mut pg, &mut report).await?;
    let subscriptions = services(source, &mut pg, &users, &products, &mut report).await?;
    invoices(source, &mut pg, &users, &subscriptions, &mut report).await?;
    tickets(source, &mut pg, &users, &staff, &mut report).await?;
    if !staff.is_empty() {
        report.notes.push(if staff.len() == 1 {
            "1 beheerder uit Paymenter is niet overgenomen; maak die opnieuw aan met stacklane admin create".to_owned()
        } else {
            format!(
                "{} beheerders uit Paymenter zijn niet overgenomen; maak ze opnieuw aan met stacklane admin create",
                staff.len()
            )
        });
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
    use super::billing_interval;

    #[test]
    fn maps_paymenter_plans_to_intervals() {
        assert_eq!(billing_interval("month", 1), ("monthly", 1));
        assert_eq!(billing_interval("month", 3), ("quarterly", 3));
        assert_eq!(billing_interval("month", 6), ("half_yearly", 6));
        assert_eq!(billing_interval("year", 1), ("yearly", 12));
        assert_eq!(billing_interval("year", 2), ("yearly", 24));
    }
}
