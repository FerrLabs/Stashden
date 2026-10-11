use std::process::{Command, Stdio};
use std::time::Duration;

use percent_encoding::percent_decode_str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use uuid::Uuid;

use crate::remote::{Remote, RemoteError, Session};

const WAIT: Duration = Duration::from_secs(300);
const REQUEST_LIMIT: usize = 16 * 1024;

const DONE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Stashden</title>\
<p>Signed in. You can close this tab and go back to Stashden.</p>";

struct Handoff {
    listener: TcpListener,
    nonce: String,
}

impl Handoff {
    async fn bind() -> Result<Self, RemoteError> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|error| browser(&error))?;
        Ok(Self {
            listener,
            nonce: Uuid::now_v7().simple().to_string(),
        })
    }

    fn address(&self, base: &str) -> Result<String, RemoteError> {
        let port = self
            .listener
            .local_addr()
            .map_err(|error| browser(&error))?
            .port();
        Ok(format!(
            "{base}/?loopback_port={port}&loopback_nonce={}",
            self.nonce
        ))
    }

    async fn session(self) -> Result<Session, RemoteError> {
        timeout(WAIT, self.accept())
            .await
            .map_err(|_| RemoteError::Browser("nothing came back from the browser".to_owned()))?
    }

    async fn accept(&self) -> Result<Session, RemoteError> {
        loop {
            let (mut stream, _) = self
                .listener
                .accept()
                .await
                .map_err(|error| browser(&error))?;
            let Some(line) = request_line(&mut stream).await else {
                continue;
            };
            match session_in(&line, &self.nonce) {
                Some(session) => {
                    respond(&mut stream, "200 OK", DONE).await;
                    return Ok(session);
                }
                None => respond(&mut stream, "404 Not Found", "").await,
            }
        }
    }
}

async fn request_line(stream: &mut TcpStream) -> Option<String> {
    let mut received = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !received.contains(&b'\n') && received.len() < REQUEST_LIMIT {
        let read = timeout(Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .ok()?
            .ok()?;
        if read == 0 {
            break;
        }
        received.extend_from_slice(&chunk[..read]);
    }
    let end = received.iter().position(|byte| *byte == b'\n')?;
    String::from_utf8(received[..end].to_vec())
        .ok()
        .map(|line| line.trim_end().to_owned())
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let message = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(message.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn session_in(line: &str, nonce: &str) -> Option<Session> {
    let mut parts = line.split(' ');
    if parts.next()? != "GET" {
        return None;
    }
    let (_, query) = parts.next()?.split_once('?')?;

    let mut token = None;
    let mut expires_in = None;
    let mut carried = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        let value = percent_decode_str(value).decode_utf8().ok()?.into_owned();
        match key {
            "token" => token = Some(value),
            "expires_in" => expires_in = value.parse::<i64>().ok(),
            "nonce" => carried = Some(value),
            _ => {}
        }
    }

    if carried.as_deref() != Some(nonce) {
        return None;
    }
    Some(Session {
        token: token.filter(|token| !token.is_empty())?,
        expires_in: expires_in?,
    })
}

fn browser(source: &std::io::Error) -> RemoteError {
    RemoteError::Browser(source.to_string())
}

fn open_in_browser(address: &str) -> bool {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("rundll32");
        command.args(["url.dll,FileProtocolHandler", address]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(address);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(address);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

impl Remote {
    pub async fn login_in_browser(
        base_url: &str,
        announce: impl FnOnce(&str),
    ) -> Result<(Self, Session), RemoteError> {
        let base = base_url.trim_end_matches('/');
        let handoff = Handoff::bind().await?;
        let address = handoff.address(base)?;
        announce(&address);
        open_in_browser(&address);

        let session = handoff.session().await?;
        let remote = Self::new(base, session.token.clone())?;
        Ok((remote, session))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: &str = "0123456789abcdef";

    #[test]
    fn a_request_carrying_the_nonce_yields_the_session() {
        let line = format!("GET /?token=abc.def.ghi&expires_in=3600&nonce={NONCE} HTTP/1.1");
        let session = session_in(&line, NONCE).expect("a session");
        assert_eq!(session.token, "abc.def.ghi");
        assert_eq!(session.expires_in, 3600);
    }

    #[test]
    fn a_request_with_another_nonce_is_not_a_session() {
        let line = "GET /?token=abc&expires_in=60&nonce=somebody-elses HTTP/1.1";
        assert!(session_in(line, NONCE).is_none());
    }

    #[test]
    fn a_request_without_a_nonce_is_not_a_session() {
        assert!(session_in("GET /?token=abc&expires_in=60 HTTP/1.1", NONCE).is_none());
    }

    #[test]
    fn a_favicon_request_is_not_a_session() {
        assert!(session_in("GET /favicon.ico HTTP/1.1", NONCE).is_none());
    }

    #[test]
    fn an_empty_token_is_not_a_session() {
        let line = format!("GET /?token=&expires_in=60&nonce={NONCE} HTTP/1.1");
        assert!(session_in(&line, NONCE).is_none());
    }

    #[test]
    fn only_a_get_counts() {
        let line = format!("POST /?token=abc&expires_in=60&nonce={NONCE} HTTP/1.1");
        assert!(session_in(&line, NONCE).is_none());
    }

    #[tokio::test]
    async fn the_listener_waits_past_a_stray_request_for_the_real_one() {
        let handoff = Handoff::bind().await.expect("bound");
        let port = handoff.listener.local_addr().expect("address").port();
        let nonce = handoff.nonce.clone();

        let waiting = tokio::spawn(handoff.session());

        let mut stray = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connected");
        stray
            .write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n")
            .await
            .expect("sent");
        let mut answer = String::new();
        stray.read_to_string(&mut answer).await.expect("answered");
        assert!(answer.starts_with("HTTP/1.1 404"));

        let mut real = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connected");
        let request = format!(
            "GET /?token=the-token&expires_in=900&nonce={nonce} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
        );
        real.write_all(request.as_bytes()).await.expect("sent");
        let mut answer = String::new();
        real.read_to_string(&mut answer).await.expect("answered");
        assert!(answer.starts_with("HTTP/1.1 200"));

        let session = waiting.await.expect("joined").expect("a session");
        assert_eq!(session.token, "the-token");
        assert_eq!(session.expires_in, 900);
    }
}
