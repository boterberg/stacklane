pub mod paymenter;
pub mod pterodactyl;
pub mod text;
pub mod whmcs;

use uuid::Uuid;

pub enum Account {
    Usable(Uuid),
    Unconfirmed,
    Missing,
}

pub async fn matching_account<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    email: &str,
) -> sqlx::Result<Account> {
    let row: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, imported_at IS NOT NULL OR email_verified_at IS NOT NULL FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(executor)
    .await?;
    Ok(match row {
        Some((id, true)) => Account::Usable(id),
        Some((_, false)) => Account::Unconfirmed,
        None => Account::Missing,
    })
}

pub struct Count {
    pub label: &'static str,
    pub created: usize,
    pub existing: usize,
}

#[derive(Default)]
pub struct Report {
    pub counts: Vec<Count>,
    pub skipped: Vec<String>,
    pub notes: Vec<String>,
}

impl Report {
    pub fn count(&mut self, label: &'static str) -> &mut Count {
        if let Some(index) = self.counts.iter().position(|count| count.label == label) {
            return &mut self.counts[index];
        }
        self.counts.push(Count {
            label,
            created: 0,
            existing: 0,
        });
        self.counts.last_mut().expect("just pushed")
    }

    pub fn created(&mut self, label: &'static str) {
        self.count(label).created += 1;
    }

    pub fn existing(&mut self, label: &'static str) {
        self.count(label).existing += 1;
    }

    pub fn unconfirmed(&mut self, email: &str) {
        self.skipped.push(format!(
            "{email}: er bestaat al een account met dit adres dat nog niet is bevestigd. Koppel het met de hand."
        ));
    }
}
