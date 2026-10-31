#![no_main]

use std::path::{Component, Path};

use libfuzzer_sys::fuzz_target;
use stashden_client::sync::path::RelPath;

fuzz_target!(|input: &str| {
    let Ok(path) = RelPath::parse(input) else {
        return;
    };
    assert_eq!(RelPath::parse(path.as_str()).as_ref().ok(), Some(&path));
    assert!(path.depth() >= 1);
    assert!(!path.file_name().is_empty());

    let root = Path::new("/root");
    let resolved = path.to_path(root);
    assert!(resolved.starts_with(root));
    assert!(
        resolved
            .strip_prefix(root)
            .is_ok_and(|rest| rest.components().all(|c| matches!(c, Component::Normal(_))))
    );

    if let Some(parent) = path.parent() {
        assert!(path.is_inside(&parent));
        assert_eq!(parent.depth() + 1, path.depth());
    }
});
