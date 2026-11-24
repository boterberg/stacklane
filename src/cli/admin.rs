use std::io::BufRead;

use uuid::Uuid;

use crate::auth;
use crate::config::Config;

pub async fn create(args: &[String]) -> anyhow::Result<()> {
    let mut email = None;
    let mut name = String::new();
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--email" => email = iter.next().cloned(),
            "--name" => name = iter.next().cloned().unwrap_or_default(),
            other => anyhow::bail!("onbekende optie: {other}"),
        }
    }
    let email = email
        .map(|value| value.trim().to_lowercase())
        .filter(|value| value.contains('@'))
        .ok_or_else(|| {
            anyhow::anyhow!("gebruik: stacklane admin create --email <adres> [--name <naam>]")
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
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, first_name, last_name, role)
         VALUES ($1, $2, $3, $4, $5, 'admin')
         ON CONFLICT (email) DO UPDATE SET password_hash = EXCLUDED.password_hash, role = 'admin', updated_at = now()",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(auth::hash_password(&password)?)
    .bind(first_name)
    .bind(last_name)
    .execute(&pool)
    .await?;
    println!("  beheerder {email} staat klaar");
    Ok(())
}
