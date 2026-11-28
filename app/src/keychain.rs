use crate::runtime::blocking;
use keyring::{Entry, Error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SERVICE: &str = "Stashden";
const ACCOUNT: &str = "desktop";

#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub server: String,
    pub email: String,
    pub secret: String,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

fn encode(credentials: &Credentials) -> Result<String, String> {
    serde_json::to_string(credentials).map_err(|error| error.to_string())
}

fn decode(encoded: &str) -> Result<Credentials, String> {
    serde_json::from_str(encoded)
        .map_err(|error| format!("the keychain entry is not one this app wrote: {error}"))
}

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT)
        .map_err(|error| format!("the system keychain is unavailable: {error}"))
}

pub async fn keep(credentials: Credentials) -> Result<(), String> {
    blocking(move || {
        let encoded = encode(&credentials)?;
        entry()?
            .set_password(&encoded)
            .map_err(|error| format!("the system keychain refused the app password: {error}"))
    })
    .await
}

pub async fn kept() -> Result<Option<Credentials>, String> {
    blocking(|| match entry()?.get_password() {
        Ok(encoded) => decode(&encoded).map(Some),
        Err(Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("the system keychain could not be read: {error}")),
    })
    .await
}

pub async fn forget() -> Result<(), String> {
    blocking(|| match entry()?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "the system keychain kept the app password: {error}"
        )),
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(user_id: Option<Uuid>) -> Credentials {
        Credentials {
            server: "https://cloud.example.test".to_owned(),
            email: "a@example.test".to_owned(),
            secret: "app-password".to_owned(),
            user_id,
        }
    }

    #[test]
    fn credentials_survive_a_round_trip() {
        let id = Uuid::from_u128(7);
        let back = decode(&encode(&credentials(Some(id))).unwrap()).unwrap();
        assert_eq!(back.server, "https://cloud.example.test");
        assert_eq!(back.email, "a@example.test");
        assert_eq!(back.secret, "app-password");
        assert_eq!(back.user_id, Some(id));
    }

    #[test]
    fn an_entry_written_before_user_ids_existed_still_reads() {
        let old = r#"{"server":"s","email":"e","secret":"x"}"#;
        let back = decode(old).unwrap();
        assert_eq!(back.user_id, None);
        assert_eq!(back.secret, "x");
    }

    #[test]
    fn an_entry_this_app_did_not_write_is_refused_with_an_explanation() {
        let message = decode("not json").err().unwrap();
        assert!(message.contains("not one this app wrote"));
        assert!(decode(r#"{"server":"s"}"#).is_err());
    }
}
