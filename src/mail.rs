use lettre::message::{Mailbox, header::ContentType};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

pub enum Mailer {
    Smtp {
        transport: Box<AsyncSmtpTransport<Tokio1Executor>>,
        from: Box<Mailbox>,
    },
    Log,
    Disabled,
}

pub struct Email<'a> {
    pub to: &'a str,
    pub subject: &'a str,
    pub body: String,
}

impl Mailer {
    pub fn from_env() -> anyhow::Result<Self> {
        if std::env::var("MAIL_TRANSPORT").is_ok_and(|value| value == "log") {
            return Ok(Self::Log);
        }
        let Ok(url) = std::env::var("SMTP_URL") else {
            return Ok(Self::Disabled);
        };
        if url.trim().is_empty() {
            return Ok(Self::Disabled);
        }
        let from: Mailbox = std::env::var("MAIL_FROM")
            .map_err(|_| {
                anyhow::anyhow!("MAIL_FROM is not set, e.g. Stacklane <noreply@example.com>")
            })?
            .parse()
            .map_err(|error| anyhow::anyhow!("MAIL_FROM is not a valid address: {error}"))?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url.trim())?.build();
        Ok(Self::Smtp {
            transport: Box::new(transport),
            from: Box::new(from),
        })
    }

    pub async fn send(&self, email: Email<'_>) -> anyhow::Result<bool> {
        match self {
            Self::Smtp { transport, from } => {
                let message = Message::builder()
                    .from(from.as_ref().clone())
                    .to(Mailbox::new(None, email.to.parse::<lettre::Address>()?))
                    .subject(email.subject)
                    .header(ContentType::TEXT_PLAIN)
                    .body(email.body)?;
                transport.send(message).await?;
                Ok(true)
            }
            Self::Log => {
                if cfg!(debug_assertions) {
                    tracing::info!(
                        to = email.to,
                        subject = email.subject,
                        "mail\n{}",
                        email.body
                    );
                    return Ok(true);
                } else {
                    tracing::warn!(
                        to = email.to,
                        subject = email.subject,
                        "MAIL_TRANSPORT=log only prints mail bodies in debug builds, mail not sent"
                    );
                }
                Ok(false)
            }
            Self::Disabled => {
                tracing::warn!(
                    to = email.to,
                    subject = email.subject,
                    "SMTP_URL is not set, mail not sent"
                );
                Ok(false)
            }
        }
    }
}

pub async fn deliver(state: &crate::state::AppState, kind: &str, email: Email<'_>) {
    let recipient = email.to.to_owned();
    let subject = email.subject.to_owned();
    let sent = if state.config.demo_mode {
        Ok(false)
    } else {
        state.mailer.send(email).await
    };
    let (status, error) = match sent {
        Ok(true) => ("sent", None),
        Ok(false) => ("skipped", None),
        Err(error) => {
            tracing::warn!(kind, error = %format!("{error:#}"), "sending mail failed");
            let message: String = format!("{error:#}").chars().take(500).collect();
            ("failed", Some(message))
        }
    };
    if let Err(error) = sqlx::query(
        "INSERT INTO mail_log (id, recipient, subject, kind, status, error) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(&recipient)
    .bind(&subject)
    .bind(kind)
    .bind(status)
    .bind(error)
    .execute(&state.db)
    .await
    {
        tracing::warn!(?error, "writing the mail log failed");
    }
}
