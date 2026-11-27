mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use stashden_api::build_router;
use stashden_core::role::Role;
use tower::ServiceExt;
use uuid::Uuid;

use common::Harness;

async fn signed_in(harness: &Harness, email: &str) -> (Uuid, String) {
    let user = harness.account(email, Role::Member).await;
    let token = harness.session(user.id).await;
    (user.id, format!("Bearer {token}"))
}

async fn put(
    harness: &Harness,
    path: &str,
    bearer: &str,
    condition: Option<(header::HeaderName, &str)>,
    contents: &[u8],
) -> StatusCode {
    let mut request = Request::builder()
        .method("PUT")
        .uri(format!("/v1/files/{path}"))
        .header(header::AUTHORIZATION, bearer);
    if let Some((name, value)) = condition {
        request = request.header(name, value);
    }
    build_router(harness.state.clone(), &[], None)
        .oneshot(
            request
                .body(Body::from(contents.to_vec()))
                .expect("a well formed request"),
        )
        .await
        .expect("the router answers")
        .status()
}

async fn holds(harness: &Harness, owner: Uuid, path: &str, expected: &[u8]) -> bool {
    harness.resolve(owner, path).await.blob_hash == Some(blake3::hash(expected).into())
}

database_test!(a_write_naming_the_current_etag_lands, harness, {
    let (owner, bearer) = signed_in(&harness, "current@example.com").await;
    let seen = harness.write(owner, "a.txt", b"seen").await;

    let status = put(
        &harness,
        "a.txt",
        &bearer,
        Some((header::IF_MATCH, &seen.etag)),
        b"edited",
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(holds(&harness, owner, "a.txt", b"edited").await);
});

database_test!(a_write_over_a_newer_version_is_refused, harness, {
    let (owner, bearer) = signed_in(&harness, "stale@example.com").await;
    let seen = harness.write(owner, "a.txt", b"seen").await;
    harness.write(owner, "a.txt", b"edited elsewhere").await;

    let status = put(
        &harness,
        "a.txt",
        &bearer,
        Some((header::IF_MATCH, &seen.etag)),
        b"edited here",
    )
    .await;

    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert!(
        holds(&harness, owner, "a.txt", b"edited elsewhere").await,
        "the edit the client never saw is not replaced"
    );
});

database_test!(
    a_file_deleted_since_it_was_seen_is_not_recreated,
    harness,
    {
        let (owner, bearer) = signed_in(&harness, "deleted@example.com").await;
        let seen = harness.write(owner, "a.txt", b"seen").await;
        harness.trash(&seen).await;

        let status = put(
            &harness,
            "a.txt",
            &bearer,
            Some((header::IF_MATCH, &seen.etag)),
            b"edited here",
        )
        .await;

        assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    }
);

database_test!(
    a_new_file_does_not_replace_one_created_meanwhile,
    harness,
    {
        let (owner, bearer) = signed_in(&harness, "fresh@example.com").await;
        harness.write(owner, "a.txt", b"created elsewhere").await;

        let status = put(
            &harness,
            "a.txt",
            &bearer,
            Some((header::IF_NONE_MATCH, "*")),
            b"created here",
        )
        .await;

        assert_eq!(status, StatusCode::PRECONDITION_FAILED);
        assert!(holds(&harness, owner, "a.txt", b"created elsewhere").await);
    }
);

database_test!(a_new_file_lands_where_nothing_is, harness, {
    let (owner, bearer) = signed_in(&harness, "empty@example.com").await;

    let status = put(
        &harness,
        "a.txt",
        &bearer,
        Some((header::IF_NONE_MATCH, "*")),
        b"created here",
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(holds(&harness, owner, "a.txt", b"created here").await);
});

database_test!(a_refused_write_never_reaches_the_disk, harness, {
    let (owner, bearer) = signed_in(&harness, "unread@example.com").await;
    harness.write(owner, "a.txt", b"there").await;
    let refused = b"bytes that lost the race";

    put(
        &harness,
        "a.txt",
        &bearer,
        Some((header::IF_NONE_MATCH, "*")),
        refused,
    )
    .await;

    assert_eq!(harness.blob(blake3::hash(refused).into()).await, None);
});

async fn refusal_etag(
    harness: &Harness,
    path: &str,
    bearer: &str,
    condition: (header::HeaderName, &str),
) -> (StatusCode, Option<String>) {
    let response = build_router(harness.state.clone(), &[], None)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/files/{path}"))
                .header(header::AUTHORIZATION, bearer)
                .header(condition.0, condition.1)
                .body(Body::from("refused"))
                .expect("a well formed request"),
        )
        .await
        .expect("the router answers");
    let etag = response
        .headers()
        .get(header::ETAG)
        .map(|value| value.to_str().expect("ascii").to_owned());
    (response.status(), etag)
}

database_test!(a_stale_if_match_answers_with_the_current_etag, harness, {
    let (owner, bearer) = signed_in(&harness, "etag-stale@example.com").await;
    let seen = harness.write(owner, "a.txt", b"seen").await;
    let current = harness.write(owner, "a.txt", b"edited elsewhere").await;

    let (status, etag) =
        refusal_etag(&harness, "a.txt", &bearer, (header::IF_MATCH, &seen.etag)).await;

    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_ne!(seen.etag, current.etag);
    assert_eq!(etag, Some(current.etag));
});

database_test!(a_missing_file_answers_without_an_etag, harness, {
    let (owner, bearer) = signed_in(&harness, "etag-missing@example.com").await;
    let seen = harness.write(owner, "a.txt", b"seen").await;
    harness.trash(&seen).await;

    let (status, etag) =
        refusal_etag(&harness, "a.txt", &bearer, (header::IF_MATCH, &seen.etag)).await;
    let (never_existed, never_etag) =
        refusal_etag(&harness, "b.txt", &bearer, (header::IF_MATCH, "*")).await;

    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(etag, None);
    assert_eq!(never_existed, StatusCode::PRECONDITION_FAILED);
    assert_eq!(never_etag, None);
});

database_test!(
    if_none_match_star_answers_with_the_etag_of_the_file,
    harness,
    {
        let (owner, bearer) = signed_in(&harness, "etag-star@example.com").await;
        let existing = harness.write(owner, "a.txt", b"already here").await;

        let (status, etag) =
            refusal_etag(&harness, "a.txt", &bearer, (header::IF_NONE_MATCH, "*")).await;

        assert_eq!(status, StatusCode::PRECONDITION_FAILED);
        assert_eq!(etag, Some(existing.etag));
    }
);
