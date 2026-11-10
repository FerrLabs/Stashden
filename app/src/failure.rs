use std::fmt;

use stashden_client::RemoteError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    status: Option<u16>,
    message: String,
}

impl Failure {
    pub fn about(self, what: &str) -> Self {
        Self {
            message: format!("{what}: {}", self.message),
            ..self
        }
    }

    pub fn ends_the_session(&self) -> bool {
        matches!(self.status, Some(401 | 403))
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<RemoteError> for Failure {
    fn from(error: RemoteError) -> Self {
        Self {
            status: error.status().map(|status| status.as_u16()),
            message: error.to_string(),
        }
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self {
            status: None,
            message,
        }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_unauthorized_or_forbidden_answer_ends_the_session() {
        assert!(Failure::from(RemoteError::Unauthenticated).ends_the_session());
        assert!(!Failure::from("the network is down").ends_the_session());
    }

    #[test]
    fn naming_what_failed_keeps_the_status() {
        let failure = Failure::from(RemoteError::Unauthenticated).about("docs/a.txt");
        assert!(failure.ends_the_session());
        assert!(failure.to_string().starts_with("docs/a.txt: "));
    }
}
