use time::Date;

use crate::i18n::Locale;
use crate::pdf::{Font, PAGE_WIDTH, Page, render, text_width, wrap};
use crate::routes::billing::InvoiceDetail;
use crate::settings::Company;

const LEFT: f32 = 50.0;
const RIGHT: f32 = PAGE_WIDTH - 50.0;
const TEXT: f32 = 0.11;
const MUTED: f32 = 0.42;
const RULE: f32 = 0.86;

const COUNTRIES: [(&str, &str, &str); 20] = [
    ("NL", "Nederland", "Netherlands"),
    ("BE", "België", "Belgium"),
    ("DE", "Duitsland", "Germany"),
    ("FR", "Frankrijk", "France"),
    ("LU", "Luxemburg", "Luxembourg"),
    ("GB", "Verenigd Koninkrijk", "United Kingdom"),
    ("IE", "Ierland", "Ireland"),
    ("AT", "Oostenrijk", "Austria"),
    ("CH", "Zwitserland", "Switzerland"),
    ("DK", "Denemarken", "Denmark"),
    ("SE", "Zweden", "Sweden"),
    ("NO", "Noorwegen", "Norway"),
    ("FI", "Finland", "Finland"),
    ("ES", "Spanje", "Spain"),
    ("PT", "Portugal", "Portugal"),
    ("IT", "Italië", "Italy"),
    ("PL", "Polen", "Poland"),
    ("CZ", "Tsjechië", "Czechia"),
    ("US", "Verenigde Staten", "United States"),
    ("CA", "Canada", "Canada"),
];

fn say(locale: Locale, dutch: &'static str, english: &'static str) -> &'static str {
    match locale {
        Locale::Nl => dutch,
        Locale::En => english,
    }
}

pub fn money(cents: i64, locale: Locale) -> String {
    let negative = cents < 0;
    let cents = cents.unsigned_abs();
    let (thousands, decimal) = match locale {
        Locale::Nl => ('.', ','),
        Locale::En => (',', '.'),
    };
    let whole = (cents / 100).to_string();
    let mut grouped = String::new();
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index).is_multiple_of(3) {
            grouped.push(thousands);
        }
        grouped.push(digit);
    }
    let sign = if negative { "-" } else { "" };
    match locale {
        Locale::Nl => format!("€ {sign}{grouped}{decimal}{:02}", cents % 100),
        Locale::En => format!("{sign}€{grouped}{decimal}{:02}", cents % 100),
    }
}

pub fn date(value: Date, locale: Locale) -> String {
    let (day, month, year) = (value.day(), u8::from(value.month()), value.year());
    match locale {
        Locale::Nl => format!("{day:02}-{month:02}-{year}"),
        Locale::En => format!("{day:02}/{month:02}/{year}"),
    }
}

fn country(code: &str, locale: Locale) -> String {
    COUNTRIES
        .iter()
        .find(|(known, _, _)| *known == code)
        .map(|(_, dutch, english)| say(locale, dutch, english).to_owned())
        .unwrap_or_else(|| code.to_owned())
}

fn field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub fn render_invoice(invoice: &InvoiceDetail, company: &Company, locale: Locale) -> Vec<u8> {
    let mut pages = vec![Page::default()];
    let company_name = if company.name.is_empty() {
        "Stacklane"
    } else {
        company.name.as_str()
    };

    let page = pages.last_mut().expect("one page");
    page.text(LEFT, 62.0, 18.0, Font::Bold, TEXT, company_name);
    let mut y = 80.0;
    for line in company
        .address
        .iter()
        .flat_map(|address| address.split([',', '\n']))
    {
        page.text(LEFT, y, 9.0, Font::Regular, MUTED, line.trim());
        y += 12.0;
    }
    for line in [&company.email, &company.website].into_iter().flatten() {
        page.text(LEFT, y, 9.0, Font::Regular, MUTED, line);
        y += 12.0;
    }
    page.text_right(
        RIGHT,
        62.0,
        18.0,
        Font::Bold,
        TEXT,
        say(locale, "Factuur", "Invoice"),
    );

    let mut customer_lines = Vec::new();
    let customer = &invoice.customer;
    customer_lines.extend(field(customer, "company_name"));
    customer_lines.extend(field(customer, "name"));
    customer_lines.extend(field(customer, "address_line1"));
    customer_lines.extend(field(customer, "address_line2"));
    let city = [field(customer, "postal_code"), field(customer, "city")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if !city.is_empty() {
        customer_lines.push(city);
    }
    customer_lines.extend(field(customer, "country").map(|code| country(&code, locale)));

    let top = y.max(140.0) + 10.0;
    page.text(
        LEFT,
        top,
        9.0,
        Font::Bold,
        TEXT,
        say(locale, "Factuur aan", "Bill to"),
    );
    for (index, line) in customer_lines.iter().enumerate() {
        page.text(
            LEFT,
            top + 15.0 + index as f32 * 13.0,
            10.0,
            Font::Regular,
            TEXT,
            line,
        );
    }

    let status = match invoice.summary.status.as_str() {
        "paid" => say(locale, "Betaald", "Paid"),
        "cancelled" => say(locale, "Geannuleerd", "Cancelled"),
        "refunded" => say(locale, "Terugbetaald", "Refunded"),
        _ => say(locale, "Openstaand", "Unpaid"),
    };
    let meta = [
        (
            say(locale, "Factuurnummer", "Invoice number"),
            invoice.summary.id.to_string(),
        ),
        (
            say(locale, "Factuurdatum", "Invoice date"),
            date(invoice.summary.issued_at, locale),
        ),
        (
            say(locale, "Vervaldatum", "Due date"),
            date(invoice.summary.due_at, locale),
        ),
        (say(locale, "Status", "Status"), status.to_owned()),
    ];
    for (index, (label, value)) in meta.iter().enumerate() {
        let row = top + 15.0 + index as f32 * 13.0;
        page.text(360.0, row, 10.0, Font::Regular, MUTED, label);
        page.text_right(RIGHT, row, 10.0, Font::Regular, TEXT, value);
    }

    y = top + 15.0 + customer_lines.len().max(meta.len()) as f32 * 13.0 + 30.0;
    page.rect(LEFT - 8.0, y - 14.0, RIGHT - LEFT + 16.0, 22.0, 0.96);
    page.text(
        LEFT,
        y,
        9.0,
        Font::Bold,
        TEXT,
        say(locale, "Beschrijving", "Description"),
    );
    page.text_right(
        RIGHT,
        y,
        9.0,
        Font::Bold,
        TEXT,
        say(locale, "Bedrag", "Amount"),
    );
    y += 26.0;

    for line in &invoice.lines {
        let mut details = Vec::new();
        if let (Some(start), Some(end)) = (line.period_start, line.period_end) {
            details.push(format!(
                "{} {} {}",
                date(start, locale),
                say(locale, "t/m", "to"),
                date(end, locale)
            ));
        }
        if let Some(code) = &line.subscription_code {
            details.push(format!(
                "{} #{code}",
                say(locale, "Abonnement", "Subscription")
            ));
        }
        let description = wrap(&line.description, 380.0, 10.0, Font::Regular);
        let height = (description.len() + details.len()) as f32 * 13.0 + 14.0;
        if y + height > 700.0 {
            pages.push(Page::default());
            y = 60.0;
        }
        let page = pages.last_mut().expect("one page");
        for (index, text) in description.iter().enumerate() {
            page.text(
                LEFT,
                y + index as f32 * 13.0,
                10.0,
                Font::Regular,
                TEXT,
                text,
            );
        }
        page.text_right(
            RIGHT,
            y,
            10.0,
            Font::Regular,
            TEXT,
            &money(i64::from(line.amount_cents), locale),
        );
        for (index, text) in details.iter().enumerate() {
            page.text(
                LEFT,
                y + (description.len() + index) as f32 * 13.0,
                9.0,
                Font::Regular,
                MUTED,
                text,
            );
        }
        y += height;
        page.line(LEFT, y - 8.0, RIGHT, y - 8.0, RULE);
    }

    if y > 640.0 {
        pages.push(Page::default());
        y = 60.0;
    }
    let page = pages.last_mut().expect("one page");
    let total = i64::from(invoice.summary.total_cents);
    let tax = i64::from(invoice.tax_cents);
    let rate = invoice.tax_rate_basis_points as f32 / 100.0;
    let rate = if rate.fract() == 0.0 {
        format!("{rate:.0}")
    } else {
        format!("{rate}")
    };
    let totals = [
        (
            say(locale, "Subtotaal", "Subtotal"),
            money(total - tax, locale),
            Font::Regular,
        ),
        (
            match locale {
                Locale::Nl => "Btw",
                Locale::En => "VAT",
            },
            money(tax, locale),
            Font::Regular,
        ),
        (
            say(locale, "Totaal", "Total"),
            money(total, locale),
            Font::Bold,
        ),
    ];
    y += 8.0;
    for (index, (label, value, font)) in totals.iter().enumerate() {
        let label = if index == 1 {
            format!("{label} ({rate}%)")
        } else {
            (*label).to_owned()
        };
        page.text_right(
            430.0,
            y,
            10.0,
            *font,
            if index == 2 { TEXT } else { MUTED },
            &label,
        );
        page.text_right(RIGHT, y, 10.0, *font, TEXT, value);
        y += 16.0;
    }

    y += 20.0;
    let note = match invoice.summary.status.as_str() {
        "paid" => invoice.paid_at.map(|paid| {
            format!(
                "{} {}.",
                say(locale, "Betaald op", "Paid on"),
                date(paid.date(), locale)
            )
        }),
        "open" => Some(match (&company.iban, locale) {
            (Some(iban), Locale::Nl) => format!(
                "Betaal {} voor {} op IBAN {iban} ten name van {company_name}, onder vermelding van factuur {}.",
                money(total, locale),
                date(invoice.summary.due_at, locale),
                invoice.summary.id
            ),
            (Some(iban), Locale::En) => format!(
                "Please pay {} before {} to IBAN {iban} in the name of {company_name}, quoting invoice {}.",
                money(total, locale),
                date(invoice.summary.due_at, locale),
                invoice.summary.id
            ),
            (None, Locale::Nl) => format!(
                "Betaal {} voor {} via het klantpanel.",
                money(total, locale),
                date(invoice.summary.due_at, locale)
            ),
            (None, Locale::En) => format!(
                "Please pay {} before {} in the customer panel.",
                money(total, locale),
                date(invoice.summary.due_at, locale)
            ),
        }),
        _ => None,
    };
    if let Some(note) = note {
        for (index, line) in wrap(&note, RIGHT - LEFT, 9.5, Font::Regular)
            .iter()
            .enumerate()
        {
            page.text(
                LEFT,
                y + index as f32 * 13.0,
                9.5,
                Font::Regular,
                TEXT,
                line,
            );
        }
    }

    let footer: Vec<String> = [
        Some(company_name.to_owned()),
        company
            .coc_number
            .as_ref()
            .map(|number| format!("{} {number}", say(locale, "KVK", "CoC"))),
        company
            .vat_number
            .as_ref()
            .map(|number| format!("{} {number}", say(locale, "BTW", "VAT"))),
        company.iban.as_ref().map(|iban| format!("IBAN {iban}")),
        company.bic.as_ref().map(|bic| format!("BIC {bic}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let footer = footer.join("  ·  ");
    let count = pages.len();
    for (index, page) in pages.iter_mut().enumerate() {
        page.line(LEFT, 790.0, RIGHT, 790.0, RULE);
        page.text(
            (PAGE_WIDTH - text_width(&footer, 8.0, Font::Regular)) / 2.0,
            806.0,
            8.0,
            Font::Regular,
            MUTED,
            &footer,
        );
        if count > 1 {
            page.text_right(
                RIGHT,
                822.0,
                8.0,
                Font::Regular,
                MUTED,
                &format!("{}/{}", index + 1, count),
            );
        }
    }

    render(
        &pages,
        &format!(
            "{} {}",
            say(locale, "Factuur", "Invoice"),
            invoice.summary.id
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn formats_money_per_locale() {
        assert_eq!(money(650, Locale::Nl), "€ 6,50");
        assert_eq!(money(12_345_678, Locale::Nl), "€ 123.456,78");
        assert_eq!(money(12_345_678, Locale::En), "€123,456.78");
        assert_eq!(money(-500, Locale::Nl), "€ -5,00");
    }

    #[test]
    fn formats_dates_per_locale() {
        assert_eq!(date(date!(2026 - 05 - 25), Locale::Nl), "25-05-2026");
        assert_eq!(date(date!(2026 - 05 - 25), Locale::En), "25/05/2026");
    }
}
