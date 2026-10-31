#![no_main]

use libfuzzer_sys::fuzz_target;
use stashden_api::dav::propfind::parse;

fuzz_target!(|body: &[u8]| {
    let requested = parse(body);
    if requested.names_only {
        assert!(requested.unknown.is_empty());
    }
    for unknown in &requested.unknown {
        assert!(!unknown.name.is_empty());
    }
});
