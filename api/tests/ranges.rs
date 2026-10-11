mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use stashden_api::build_router;
use stashden_core::role::Role;
use tower::ServiceExt;

use common::Harness;

struct Answer {
    status: StatusCode,
    headers: header::HeaderMap,
    body: Vec<u8>,
}

async fn fetch(harness: &Harness, token: &str, path: &str, extra: &[(&str, &str)]) -> Answer {
    let mut request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    for (name, value) in extra {
        request = request.header(*name, *value);
    }
    let response = build_router(harness.state.clone(), &[], None)
        .oneshot(request.body(Body::empty()).expect("a request"))
        .await
        .expect("the router answers");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    Answer {
        status,
        headers,
        body,
    }
}

const CONTENT: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

async fn stored(harness: &Harness) -> String {
    let owner = harness.account("owner@example.com", Role::Member).await;
    harness.write(owner.id, "digits.txt", CONTENT).await;
    harness.session(owner.id).await
}

database_test!(a_whole_read_advertises_ranges, harness, {
    let token = stored(&harness).await;

    let answer = fetch(&harness, &token, "/v1/files/digits.txt", &[]).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.headers[header::ACCEPT_RANGES], "bytes");
    assert_eq!(answer.body, CONTENT);
});

database_test!(a_range_answers_just_those_bytes, harness, {
    let token = stored(&harness).await;

    let answer = fetch(
        &harness,
        &token,
        "/v1/files/digits.txt",
        &[("range", "bytes=10-15")],
    )
    .await;

    assert_eq!(answer.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(answer.body, &CONTENT[10..=15]);
    assert_eq!(answer.headers[header::CONTENT_RANGE], "bytes 10-15/36");
    assert_eq!(answer.headers[header::CONTENT_LENGTH], "6");
});

database_test!(an_open_and_a_suffix_range_reach_the_end, harness, {
    let token = stored(&harness).await;

    let open = fetch(
        &harness,
        &token,
        "/v1/files/digits.txt",
        &[("range", "bytes=30-")],
    )
    .await;
    let suffix = fetch(
        &harness,
        &token,
        "/v1/files/digits.txt",
        &[("range", "bytes=-4")],
    )
    .await;

    assert_eq!(open.body, &CONTENT[30..]);
    assert_eq!(suffix.body, &CONTENT[32..]);
    assert_eq!(suffix.headers[header::CONTENT_RANGE], "bytes 32-35/36");
});

database_test!(a_range_past_the_end_is_refused_with_the_size, harness, {
    let token = stored(&harness).await;

    let answer = fetch(
        &harness,
        &token,
        "/v1/files/digits.txt",
        &[("range", "bytes=40-")],
    )
    .await;

    assert_eq!(answer.status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(answer.headers[header::CONTENT_RANGE], "bytes */36");
});

database_test!(a_current_etag_is_answered_not_modified, harness, {
    let token = stored(&harness).await;
    let first = fetch(&harness, &token, "/v1/files/digits.txt", &[]).await;
    let etag = first.headers[header::ETAG]
        .to_str()
        .expect("text")
        .to_owned();

    let again = fetch(
        &harness,
        &token,
        "/v1/files/digits.txt",
        &[("if-none-match", &etag)],
    )
    .await;

    assert_eq!(again.status, StatusCode::NOT_MODIFIED);
    assert_eq!(again.body, b"");
});

database_test!(
    a_range_guarded_by_a_stale_etag_gets_the_whole_file,
    harness,
    {
        let token = stored(&harness).await;

        let answer = fetch(
            &harness,
            &token,
            "/v1/files/digits.txt",
            &[("range", "bytes=0-3"), ("if-range", "\"stale\"")],
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.body, CONTENT);
    }
);
