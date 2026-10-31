#![no_main]

use libfuzzer_sys::fuzz_target;
use stashden_core::name::{MAX_NAME_LEN, NodeName, parse_path};

fuzz_target!(|input: &str| {
    if let Ok(segments) = parse_path(input) {
        for segment in &segments {
            let name = segment.as_str();
            assert!(!name.is_empty() && name.len() <= MAX_NAME_LEN);
            assert!(name != "." && name != "..");
            assert!(!name.contains(['/', '\\']));
            assert!(!name.chars().any(char::is_control));
        }
        let joined = segments
            .iter()
            .map(NodeName::as_str)
            .collect::<Vec<_>>()
            .join("/");
        assert_eq!(parse_path(&joined).as_ref(), Ok(&segments));
    }
});
