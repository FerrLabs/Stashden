use std::fmt;

use reqwest::Url;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    #[serde(deserialize_with = "normalized")]
    server: String,
    user_id: Uuid,
}

fn normalized<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    String::deserialize(deserializer).map(|server| normalize(&server))
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
    let mut current = canonical(server);
    loop {
        let next = canonical(&current);
        if next == current {
            return current;
        }
        current = next;
    }
}

fn canonical(server: &str) -> String {
    let bare = strip(server);
    let parsed = Url::parse(bare).map_or_else(|_| bare.to_owned(), String::from);
    strip(&parsed).to_owned()
}

fn strip(text: &str) -> &str {
    text.trim_start()
        .trim_end_matches(|c: char| c == '/' || c.is_whitespace())
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
    fn what_is_written_reads_back_as_the_same_account() {
        for server in [
            "files.example.com /",
            "https://files.example.com:443 /",
            "not a url / /",
            "",
            "/",
            "https://Files.Example.com/stash/ ",
        ] {
            let account = Account::new(server, USER);
            let written = serde_json::to_string(&account).expect("writes");
            let read: Account = serde_json::from_str(&written).expect("reads");
            assert_eq!(read, account, "{server:?}");
        }
    }

    #[test]
    fn a_stored_server_is_normalized_when_read_back() {
        let stored = format!(r#"{{"server":"https://Files.Example.com/","user_id":"{USER}"}}"#);
        let read: Account = serde_json::from_str(&stored).expect("reads");
        assert_eq!(read, Account::new("https://files.example.com", USER));
    }

    #[test]
    fn dot_segments_after_a_hostless_scheme_settle_in_one_step() {
        let account = Account::new("foo:/.//x/..", USER);
        let written = serde_json::to_string(&account).expect("writes");
        let read: Account = serde_json::from_str(&written).expect("reads");
        assert_eq!(read, account);
        assert_eq!(Account::new(&account.server, USER), account);
    }

    #[test]
    fn normalizing_twice_is_normalizing_once() {
        const PIECES: &[&str] = &[
            "foo:", "file:", "https://", "HTTP://", "/", "//", ".", "..", "/./", "/../", "x", "A",
            " ", "%2e", "%2F", ":443", "@", "?", "#", "[::1]", "\\", "\u{3000}", "a b",
        ];
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move |bound: usize| {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            usize::try_from(state.wrapping_mul(0x2545_F491_4F6C_DD1D) % bound as u64).unwrap()
        };
        for _ in 0..20_000 {
            let server: String = (0..=next(8)).map(|_| PIECES[next(PIECES.len())]).collect();
            let once = normalize(&server);
            assert_eq!(normalize(&once), once, "{server:?}");
        }
    }

    #[test]
    fn the_display_names_both_the_user_and_the_server() {
        let shown = Account::new("https://files.example.com/", USER).to_string();
        assert!(shown.contains(&USER.to_string()), "{shown}");
        assert!(shown.contains("https://files.example.com"), "{shown}");
    }
}
