use std::io::BufRead;

use uuid::Uuid;

use crate::auth;
use crate::config::Config;

pub async fn create(args: &[String]) -> anyhow::Result<()> {
    let mut email = None;
    let mut name = String::new();
    let mut promote = false;
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--email" => email = iter.next().cloned(),
            "--name" => name = iter.next().cloned().unwrap_or_default(),
            "--promote" => promote = true,
            other => anyhow::bail!("onbekende optie: {other}"),
        }
    }
    let email = email
        .map(|value| value.trim().to_lowercase())
        .filter(|value| value.contains('@'))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "gebruik: stacklane admin create --email <adres> [--name <naam>] [--promote]"
            )
        })?;

    let password = if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        rpassword::prompt_password("  Wachtwoord: ")?
    } else {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        line.trim_end_matches(['\r', '\n']).to_owned()
    };
    anyhow::ensure!(
        password.chars().count() >= 10,
        "het wachtwoord moet minstens 10 tekens lang zijn"
    );

    let (first_name, last_name) = match name.trim().split_once(' ') {
        Some((first, last)) => (first.to_owned(), last.trim().to_owned()),
        None => (name.trim().to_owned(), String::new()),
    };

    let config = Config::from_env()?;
    let pool = crate::connect(&config).await?;
    let hash = auth::hash_password(&password)?;
    let mut transaction = pool.begin().await?;
    let existing: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, role FROM users WHERE email = $1 FOR UPDATE")
            .bind(&email)
            .fetch_optional(&mut *transaction)
            .await?;
    match action(existing.as_ref().map(|(_, role)| role.as_str()), promote)? {
        Action::Create => {
            sqlx::query(
                "INSERT INTO users (id, email, password_hash, first_name, last_name, role)
                 VALUES ($1, $2, $3, $4, $5, 'admin')",
            )
            .bind(Uuid::new_v4())
            .bind(&email)
            .bind(&hash)
            .bind(first_name)
            .bind(last_name)
            .execute(&mut *transaction)
            .await?;
        }
        Action::Reset => {
            let id = existing.map(|(id, _)| id).unwrap_or_default();
            crate::auth::forget_unproven_holder(&mut transaction, id).await?;
            sqlx::query(
                "UPDATE users SET password_hash = $2, role = 'admin', totp_secret = NULL, totp_pending = NULL,
                     totp_last_step = NULL, two_factor_enabled_at = NULL,
                     email_verified_at = COALESCE(email_verified_at, now()), updated_at = now()
                 WHERE id = $1",
            )
            .bind(id)
            .bind(&hash)
            .execute(&mut *transaction)
            .await?;
            for table in [
                "sessions",
                "api_tokens",
                "password_resets",
                "recovery_codes",
            ] {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DELETE FROM {table} WHERE user_id = $1"
                )))
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            }
        }
    }
    transaction.commit().await?;
    println!("  beheerder {email} staat klaar");
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Action {
    Create,
    Reset,
}

fn action(role: Option<&str>, promote: bool) -> anyhow::Result<Action> {
    match role {
        None => Ok(Action::Create),
        Some("admin") => Ok(Action::Reset),
        Some(_) if promote => Ok(Action::Reset),
        Some(_) => anyhow::bail!(
            "er bestaat al een klantaccount met dit adres. Gebruik --promote als dat account van jou is; \
             het krijgt dan een nieuw wachtwoord en alle sessies, tokens en 2FA worden gewist"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_adopts_a_customer_account_silently() {
        assert_eq!(action(None, false).unwrap(), Action::Create);
        assert_eq!(action(Some("admin"), false).unwrap(), Action::Reset);
        assert!(action(Some("customer"), false).is_err());
        assert_eq!(action(Some("customer"), true).unwrap(), Action::Reset);
    }
}
