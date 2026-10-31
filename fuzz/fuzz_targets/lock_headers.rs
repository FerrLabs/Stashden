#![no_main]

use libfuzzer_sys::fuzz_target;
use stashden_api::dav::locks::{lock_token, requested_seconds, submitted_tokens};

const SCHEME: &str = "opaquelocktoken:";

fuzz_target!(|header: &str| {
    for token in submitted_tokens(Some(header)) {
        assert!(token.starts_with(SCHEME));
        assert!(header.contains(&format!("<{token}>")));
    }
    if let Some(token) = lock_token(Some(header)) {
        assert!(token.starts_with(SCHEME));
    }
    assert!(requested_seconds(Some(header)) > 0);
});
