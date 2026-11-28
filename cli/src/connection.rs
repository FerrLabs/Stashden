use anyhow::{Context, Result};
use stashden_client::Remote;

pub const DEFAULT_SERVER: &str = "http://localhost:3001";
pub const SERVER_HELP: &str = "Server to talk to [default: http://localhost:3001]";

pub fn remote(server: &str, token: Option<&str>) -> Result<Remote> {
    let Some(token) = token.filter(|token| !token.is_empty()) else {
        anyhow::bail!(
            "not signed in: set STASHDEN_TOKEN from `stashden login`, or STASHDEN_EMAIL and \
             STASHDEN_APP_PASSWORD from `stashden login --create-app-password <name>`"
        );
    };
    Remote::new(server, token.to_owned()).context("building the API client")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_token_says_how_to_sign_in() {
        let message = remote("http://localhost:3001", None)
            .err()
            .expect("there is no token")
            .to_string();
        assert!(message.contains("STASHDEN_TOKEN"));
        assert!(message.contains("--create-app-password"));
    }

    #[test]
    fn an_empty_token_counts_as_none() {
        assert!(remote("http://localhost:3001", Some("")).is_err());
    }

    #[test]
    fn a_token_builds_a_client() {
        assert!(remote("http://localhost:3001", Some("secret")).is_ok());
    }

    #[test]
    fn the_help_text_names_the_default_server() {
        assert!(SERVER_HELP.contains(DEFAULT_SERVER));
    }
}
