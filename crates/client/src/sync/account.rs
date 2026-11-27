use std::fmt;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "StoredAccount")]
pub struct Account {
    server: String,
    user_id: Uuid,
}

#[derive(Deserialize)]
struct StoredAccount {
    server: String,
    user_id: Uuid,
}

impl From<StoredAccount> for Account {
    fn from(stored: StoredAccount) -> Self {
        Self::new(&stored.server, stored.user_id)
    }
}

impl Account {
    #[must_use]
    pub fn new(server: &str, user_id: Uuid) -> Self {
        Self {
            server: normalize(server),
            user_id,
        }
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "account {} on {}", self.user_id, self.server)
    }
}

fn normalize(server: &str) -> String {
    let server = server.trim();
    let canonical = Url::parse(server).map_or_else(|_| server.to_owned(), String::from);
    canonical.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: Uuid = Uuid::from_u128(1);

    #[test]
    fn a_trailing_slash_and_host_case_do_not_make_another_server() {
        let plain = Account::new("https://files.example.com", USER);
        assert_eq!(plain, Account::new("https://files.example.com/", USER));
        assert_eq!(plain, Account::new("https://Files.Example.COM//", USER));
        assert_eq!(plain, Account::new(" HTTPS://files.example.com ", USER));
    }

    #[test]
    fn another_instance_or_another_path_is_another_server() {
        let base = Account::new("https://files.example.com", USER);
        assert_ne!(base, Account::new("https://other.example.com", USER));
        assert_ne!(base, Account::new("http://files.example.com", USER));
        assert_ne!(
            Account::new("https://example.com/stash", USER),
            Account::new("https://example.com/other", USER)
        );
    }

    #[test]
    fn the_same_server_with_another_user_is_another_account() {
        assert_ne!(
            Account::new("https://files.example.com", USER),
            Account::new("https://files.example.com", Uuid::from_u128(2))
        );
    }

    #[test]
    fn an_address_that_is_not_a_url_still_compares_by_its_trimmed_text() {
        assert_eq!(
            Account::new("not a url/", USER),
            Account::new(" not a url", USER)
        );
    }

    #[test]
    fn a_stored_server_is_normalized_when_read_back() {
        let stored = format!(r#"{{"server":"https://Files.Example.com/","user_id":"{USER}"}}"#);
        let read: Account = serde_json::from_str(&stored).expect("reads");
        assert_eq!(read, Account::new("https://files.example.com", USER));
    }

    #[test]
    fn the_display_names_both_the_user_and_the_server() {
        let shown = Account::new("https://files.example.com/", USER).to_string();
        assert!(shown.contains(&USER.to_string()), "{shown}");
        assert!(shown.contains("https://files.example.com"), "{shown}");
    }
}
