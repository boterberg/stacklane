use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::AppResult;
use crate::settings::Billing;
use crate::state::AppState;
use crate::vat;

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
    let (country, verified_vat_number): (String, Option<String>) = sqlx::query_as(
        "SELECT country::text, CASE WHEN is_business AND tax_id_verified_at IS NOT NULL THEN tax_id END
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    let decision = vat::decide(
        &vat::Seller {
            country: &billing.home_country,
            rate: billing.tax_rate_basis_points,
            oss: billing.oss,
        },
        &vat::Buyer {
            country: &country,
            verified_vat_number: verified_vat_number.is_some(),
        },
    );
    let amounts: Vec<i64> = lines
        .iter()
        .map(|line| {
            vat::convert(
                line.amount_cents,
                billing.tax_rate_basis_points,
                decision.rate,
            )
        })
        .collect();
    let total: i64 = amounts.iter().sum();
    let customer_vat_number =
        verified_vat_number.filter(|_| decision.scheme == vat::Scheme::ReverseCharge);
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
        "INSERT INTO invoices (user_id, status, issued_at, due_at, tax_rate_basis_points, total_cents, tax_cents, billing_snapshot,
                               vat_scheme, vat_country, customer_vat_number, currency)
         VALUES ($1, 'open', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(user_id)
    .bind(OffsetDateTime::now_utc().date())
    .bind(due_at)
    .bind(decision.rate)
    .bind(cents(total))
    .bind(cents(tax_included(total, decision.rate)))
    .bind(snapshot)
    .bind(decision.scheme.as_str())
    .bind(&decision.country)
    .bind(customer_vat_number)
    .bind(&billing.currency)
    .fetch_one(&mut *transaction)
    .await?;
    for (line, amount) in lines.iter().zip(&amounts) {
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
        .bind(cents(*amount))
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
