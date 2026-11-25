use crate::i18n::Locale;

pub const CURRENCIES: [&str; 11] = [
    "EUR", "USD", "GBP", "CHF", "SEK", "NOK", "DKK", "PLN", "CZK", "CAD", "AUD",
];

pub fn supported(currency: &str) -> bool {
    CURRENCIES.contains(&currency)
}

fn symbol(currency: &str) -> Option<&'static str> {
    match currency {
        "EUR" => Some("€"),
        "USD" => Some("$"),
        "GBP" => Some("£"),
        _ => None,
    }
}

pub fn format(cents: i64, currency: &str, locale: Locale) -> String {
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
    let amount = format!("{sign}{grouped}{decimal}{:02}", cents % 100);
    match (symbol(currency), locale) {
        (Some(symbol), Locale::Nl) => format!("{symbol} {amount}"),
        (Some(symbol), Locale::En) => format!("{sign}{symbol}{}", &amount[sign.len()..]),
        (None, _) => format!("{currency} {amount}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_follow_the_currency_and_language() {
        assert_eq!(format(650, "EUR", Locale::Nl), "€ 6,50");
        assert_eq!(format(12_345_678, "EUR", Locale::Nl), "€ 123.456,78");
        assert_eq!(format(12_345_678, "EUR", Locale::En), "€123,456.78");
        assert_eq!(format(-500, "EUR", Locale::Nl), "€ -5,00");
        assert_eq!(format(-500, "USD", Locale::En), "-$5.00");
        assert_eq!(format(199_900, "GBP", Locale::En), "£1,999.00");
        assert_eq!(format(1250, "CHF", Locale::Nl), "CHF 12,50");
    }
}
