use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::error::ApiError;
use crate::metrics::same_token;
use crate::oidc::sha256;

const CHALLENGE_BYTES: usize = 32;
const LONGEST_VERIFIER: usize = 128;
const CONFIRMATION_LENGTH: usize = 6;
const CONFIRMATION_ALPHABET: &[u8; 20] = b"BCDFGHJKLMNPQRSTVWXZ";
const UNBIASED_BELOW: u8 = 240;

pub fn valid_challenge(challenge: &str) -> Result<(), ApiError> {
    match URL_SAFE_NO_PAD.decode(challenge) {
        Ok(bytes) if bytes.len() == CHALLENGE_BYTES && challenge.len() == 43 => Ok(()),
        _ => Err(ApiError::WrongKind {
            expected: "challenge, the base64url SHA-256 of the verifier",
        }),
    }
}

#[must_use]
pub fn challenge_of(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(sha256(verifier.as_bytes()))
}

#[must_use]
pub fn answers(challenge: &str, verifier: &str) -> bool {
    verifier.len() <= LONGEST_VERIFIER && same_token(&challenge_of(verifier), challenge)
}

#[must_use]
pub fn confirmation() -> String {
    let mut code = String::with_capacity(CONFIRMATION_LENGTH);
    while code.len() < CONFIRMATION_LENGTH {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("the operating system has randomness");
        code.extend(
            bytes
                .into_iter()
                .filter(|byte| *byte < UNBIASED_BELOW)
                .map(|byte| char::from(CONFIRMATION_ALPHABET[usize::from(byte) % 20]))
                .take(CONFIRMATION_LENGTH - code.len()),
        );
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    #[test]
    fn the_challenge_is_the_rfc_7636_transform_of_the_verifier() {
        assert_eq!(
            challenge_of(VERIFIER),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn only_the_verifier_behind_a_challenge_answers_it() {
        let challenge = challenge_of(VERIFIER);
        assert!(answers(&challenge, VERIFIER));
        assert!(!answers(&challenge, "another-verifier"));
        assert!(!answers(&challenge, ""));
        assert!(!answers(&challenge, &challenge));
    }

    #[test]
    fn a_verifier_past_the_rfc_limit_never_answers() {
        let verifier = "a".repeat(LONGEST_VERIFIER + 1);
        assert!(!answers(&challenge_of(&verifier), &verifier));
    }

    #[test]
    fn a_challenge_is_exactly_a_sha256_in_base64url() {
        assert!(valid_challenge(&challenge_of(VERIFIER)).is_ok());
        let padded = format!("{}=", challenge_of(VERIFIER));
        let stretched = format!("{}A", challenge_of(VERIFIER));
        for refused in ["", "short", padded.as_str(), stretched.as_str()] {
            assert!(valid_challenge(refused).is_err(), "{refused}");
        }
        assert!(valid_challenge(&"+".repeat(43)).is_err());
    }

    #[test]
    fn a_confirmation_is_six_characters_of_the_unambiguous_alphabet() {
        for _ in 0..200 {
            let code = confirmation();
            assert_eq!(code.len(), CONFIRMATION_LENGTH);
            assert!(
                code.bytes()
                    .all(|byte| CONFIRMATION_ALPHABET.contains(&byte))
            );
        }
    }
}
