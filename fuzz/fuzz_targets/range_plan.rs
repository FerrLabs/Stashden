#![no_main]

use axum::http::{HeaderMap, HeaderName, HeaderValue, header};
use libfuzzer_sys::fuzz_target;
use stashden_api::ranges::{Delivery, plan};

const ETAG: &str = "\"abc\"";

fn insert(headers: &mut HeaderMap, name: HeaderName, raw: &[u8]) {
    if let Ok(value) = HeaderValue::from_bytes(raw) {
        headers.insert(name, value);
    }
}

fuzz_target!(|input: (u64, &[u8], &[u8], &[u8])| {
    let (size, range, if_range, if_none_match) = input;
    let mut headers = HeaderMap::new();
    insert(&mut headers, header::RANGE, range);
    insert(&mut headers, header::IF_RANGE, if_range);
    insert(&mut headers, header::IF_NONE_MATCH, if_none_match);

    if let Delivery::Partial { start, end } = plan(&headers, ETAG, size) {
        assert!(start <= end);
        assert!(end < size);
    }
});
