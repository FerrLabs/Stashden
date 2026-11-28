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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    async fn capture_one_mail(listener: TcpListener) -> String {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut lines = BufReader::new(read).lines();
        write.write_all(b"220 test ready\r\n").await.unwrap();
        let mut data = String::new();
        let mut in_data = false;
        while let Some(line) = lines.next_line().await.unwrap() {
            if in_data {
                if line == "." {
                    in_data = false;
                    write.write_all(b"250 queued\r\n").await.unwrap();
                } else {
                    data.push_str(&line);
                    data.push('\n');
                }
                continue;
            }
            let reply: &[u8] = match line.split(' ').next().unwrap_or_default() {
                "EHLO" => b"250 test\r\n",
                "DATA" => {
                    in_data = true;
                    b"354 go ahead\r\n"
                }
                "QUIT" => {
                    write.write_all(b"221 bye\r\n").await.unwrap();
                    break;
                }
                _ => b"250 ok\r\n",
            };
            write.write_all(reply).await.unwrap();
        }
        data
    }

    fn letter() -> Letter {
        Letter {
            to: "reader@example.test".parse().unwrap(),
            subject: "Reset your password".to_owned(),
            body: "Follow the link".to_owned(),
        }
    }

    #[test]
    fn an_smtp_url_and_a_sender_address_build_a_transport() {
        assert!(Smtp::new("smtp://localhost:2525", "Stashden <noreply@example.test>").is_ok());
        assert!(Smtp::new("smtps://user:pw@mail.example.test", "noreply@example.test").is_ok());
    }

    #[test]
    fn a_url_that_is_not_smtp_is_refused_with_its_setting_name() {
        let error = Smtp::new("http://localhost", "noreply@example.test")
            .err()
            .expect("an http url is not an smtp url");
        assert!(error.to_string().contains("SMTP_URL"));
    }

    #[test]
    fn a_sender_that_is_not_an_address_is_refused_with_its_setting_name() {
        let error = Smtp::new("smtp://localhost", "not an address")
            .err()
            .expect("a sender needs an address");
        assert!(error.to_string().contains("MAIL_FROM"));
    }

    #[test]
    fn the_public_url_loses_its_trailing_slashes() {
        let mail = Mail::from_config(&MailConfig {
            smtp_url: "smtp://localhost".to_owned(),
            from: "noreply@example.test".to_owned(),
            public_url: "https://cloud.example.test//".to_owned(),
        })
        .unwrap();
        assert_eq!(&*mail.public_url, "https://cloud.example.test");
    }

    #[test]
    fn a_bad_setting_stops_the_mail_from_being_built() {
        let built = Mail::from_config(&MailConfig {
            smtp_url: "nonsense".to_owned(),
            from: "noreply@example.test".to_owned(),
            public_url: "https://cloud.example.test".to_owned(),
        });
        assert!(built.is_err());
    }

    #[tokio::test]
    async fn a_letter_reaches_the_server_with_sender_recipient_subject_and_body() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(capture_one_mail(listener));
        let smtp = Smtp::new(&format!("smtp://127.0.0.1:{port}"), "noreply@example.test").unwrap();

        smtp.send(letter()).await.unwrap();

        let received = server.await.unwrap();
        assert!(
            received.contains("From: noreply@example.test"),
            "{received}"
        );
        assert!(received.contains("To: reader@example.test"), "{received}");
        assert!(
            received.contains("Subject: Reset your password"),
            "{received}"
        );
        assert!(received.contains("text/plain"), "{received}");
        assert!(received.contains("Follow the link"), "{received}");
    }

    #[tokio::test]
    async fn a_server_that_cannot_be_reached_is_a_send_error() {
        let unused = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = unused.local_addr().unwrap().port();
        drop(unused);
        let smtp = Smtp::new(&format!("smtp://127.0.0.1:{port}"), "noreply@example.test").unwrap();

        let error = smtp.send(letter()).await.unwrap_err();

        assert!(matches!(error, MailError::Send(_)), "{error}");
    }
}
