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

    pub async fn send(&self, email: Email<'_>) -> anyhow::Result<()> {
        match self {
            Self::Smtp { transport, from } => {
                let message = Message::builder()
                    .from(from.as_ref().clone())
                    .to(email.to.parse()?)
                    .subject(email.subject)
                    .header(ContentType::TEXT_PLAIN)
                    .body(email.body)?;
                transport.send(message).await?;
                Ok(())
            }
            Self::Log => {
                tracing::info!(
                    to = email.to,
                    subject = email.subject,
                    "mail\n{}",
                    email.body
                );
                Ok(())
            }
            Self::Disabled => {
                tracing::warn!(
                    to = email.to,
                    subject = email.subject,
                    "SMTP_URL is not set, mail not sent"
                );
                Ok(())
            }
        }
    }
}
