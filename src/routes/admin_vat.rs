use axum::Json;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use time::{Date, Month};

use crate::auth::CurrentAdmin;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Period {
    year: i32,
    quarter: u8,
    #[serde(default)]
    format: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Row {
    scheme: String,
    country: Option<String>,
    rate_basis_points: i32,
    invoices: i64,
    net_cents: i64,
    vat_cents: i64,
    gross_cents: i64,
}

pub fn quarter_bounds(year: i32, quarter: u8) -> Option<(Date, Date)> {
    let first = Month::try_from(quarter.checked_sub(1)? * 3 + 1).ok()?;
    let start = Date::from_calendar_date(year, first, 1).ok()?;
    let end = if quarter == 4 {
        Date::from_calendar_date(year + 1, Month::January, 1).ok()?
    } else {
        Date::from_calendar_date(year, first.nth_next(3), 1).ok()?
    };
    Some((start, end))
}

fn euros(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, (cents % 100).abs())
}

pub async fn report(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Query(period): Query<Period>,
) -> AppResult<Response> {
    let Some((start, end)) = quarter_bounds(period.year, period.quarter) else {
        return Err(AppError::field(
            "quarter",
            "Kies een kwartaal van 1 tot en met 4.",
        ));
    };
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT vat_scheme AS scheme, vat_country::text AS country, tax_rate_basis_points AS rate_basis_points,
                count(*) AS invoices, sum(total_cents - tax_cents)::bigint AS net_cents,
                sum(tax_cents)::bigint AS vat_cents, sum(total_cents)::bigint AS gross_cents
         FROM invoices
         WHERE status <> 'cancelled' AND issued_at >= $1 AND issued_at < $2
         GROUP BY vat_scheme, vat_country, tax_rate_basis_points
         ORDER BY vat_scheme, vat_country, tax_rate_basis_points",
    )
    .bind(start)
    .bind(end)
    .fetch_all(&state.db)
    .await?;
    if period.format.as_deref() != Some("csv") {
        return Ok(Json(rows).into_response());
    }
    let mut csv = String::from("scheme,country,vat_rate,invoices,net_eur,vat_eur,gross_eur\n");
    for row in &rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            row.scheme,
            row.country.as_deref().unwrap_or(""),
            euros(i64::from(row.rate_basis_points)),
            row.invoices,
            euros(row.net_cents),
            euros(row.vat_cents),
            euros(row.gross_cents),
        ));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"btw-{}-q{}.csv\"",
                    period.year, period.quarter
                ),
            ),
            (header::CACHE_CONTROL, "private, no-store".to_owned()),
        ],
        csv,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::{euros, quarter_bounds};
    use time::macros::date;

    #[test]
    fn computes_quarters() {
        assert_eq!(
            quarter_bounds(2026, 1),
            Some((date!(2026 - 01 - 01), date!(2026 - 04 - 01)))
        );
        assert_eq!(
            quarter_bounds(2026, 4),
            Some((date!(2026 - 10 - 01), date!(2027 - 01 - 01)))
        );
        assert_eq!(quarter_bounds(2026, 0), None);
        assert_eq!(quarter_bounds(2026, 5), None);
        assert_eq!(euros(12_345), "123.45");
        assert_eq!(euros(2100), "21.00");
    }
}
