use std::sync::Arc;

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use stashden_core::user::Email;

use crate::config::MailConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Letter {
    pub to: Email,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("the mail could not be built: {0}")]
    Build(String),
    #[error("the mail server refused the mail: {0}")]
    Send(String),
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, letter: Letter) -> Result<(), MailError>;
}

#[derive(Clone)]
pub struct Mail {
    pub mailer: Arc<dyn Mailer>,
    pub public_url: Arc<str>,
}

impl Mail {
    pub fn from_config(cfg: &MailConfig) -> anyhow::Result<Self> {
        Ok(Self {
            mailer: Arc::new(Smtp::new(&cfg.smtp_url, &cfg.from)?),
            public_url: Arc::from(cfg.public_url.trim_end_matches('/')),
        })
    }
}

pub struct Smtp {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Smtp {
    pub fn new(url: &str, from: &str) -> anyhow::Result<Self> {
        Ok(Self {
            transport: AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
                .map_err(|_| anyhow::anyhow!("SMTP_URL is not a valid smtp:// or smtps:// URL"))?
                .build(),
            from: from
                .parse()
                .map_err(|_| anyhow::anyhow!("MAIL_FROM is not a valid address"))?,
        })
    }
}

#[async_trait]
impl Mailer for Smtp {
    async fn send(&self, letter: Letter) -> Result<(), MailError> {
        let to: Mailbox = letter
            .to
            .as_str()
            .parse()
            .map_err(|error: lettre::address::AddressError| MailError::Build(error.to_string()))?;
        let message = lettre::Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(letter.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(letter.body)
            .map_err(|error| MailError::Build(error.to_string()))?;
        self.transport
            .send(message)
            .await
            .map(drop)
            .map_err(|error| MailError::Send(error.to_string()))
    }
}
