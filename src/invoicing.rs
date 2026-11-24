use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::AppResult;
use crate::settings::Billing;
use crate::state::AppState;

pub struct Line {
    pub subscription_id: Option<Uuid>,
    pub description: String,
    pub period_start: Option<Date>,
    pub period_end: Option<Date>,
    pub amount_cents: i64,
}

pub fn tax_included(total_cents: i64, rate_basis_points: i32) -> i64 {
    let rate = i64::from(rate_basis_points);
    total_cents - (total_cents * 10_000 + (10_000 + rate) / 2) / (10_000 + rate)
}

fn cents(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub async fn issue(
    state: &AppState,
    user_id: Uuid,
    lines: &[Line],
    due_at: Date,
    billing: &Billing,
) -> AppResult<(i64, i64)> {
    let total: i64 = lines.iter().map(|line| line.amount_cents).sum();
    let snapshot: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object(
            'name', COALESCE(NULLIF(trim(first_name || ' ' || last_name), ''), email::text),
            'company_name', company_name, 'address_line1', address_line1, 'address_line2', address_line2,
            'postal_code', postal_code, 'city', city, 'country', country::text)
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let mut transaction = state.db.begin().await?;
    let invoice_id: i64 = sqlx::query_scalar(
        "INSERT INTO invoices (user_id, status, issued_at, due_at, tax_rate_basis_points, total_cents, tax_cents, billing_snapshot)
         VALUES ($1, 'open', $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(user_id)
    .bind(OffsetDateTime::now_utc().date())
    .bind(due_at)
    .bind(billing.tax_rate_basis_points)
    .bind(cents(total))
    .bind(cents(tax_included(total, billing.tax_rate_basis_points)))
    .bind(snapshot)
    .fetch_one(&mut *transaction)
    .await?;
    for line in lines {
        sqlx::query(
            "INSERT INTO invoice_lines (id, invoice_id, subscription_id, description, period_start, period_end, amount_cents)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(invoice_id)
        .bind(line.subscription_id)
        .bind(&line.description)
        .bind(line.period_start)
        .bind(line.period_end)
        .bind(cents(line.amount_cents))
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok((invoice_id, total))
}

#[cfg(test)]
mod tests {
    use super::tax_included;

    #[test]
    fn splits_vat_out_of_gross_prices() {
        assert_eq!(tax_included(650, 2100), 113);
        assert_eq!(tax_included(1250, 2100), 217);
        assert_eq!(tax_included(5100, 2100), 885);
        assert_eq!(tax_included(1000, 0), 0);
    }
}
