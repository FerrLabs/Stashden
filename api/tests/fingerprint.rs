mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use stashden_api::build_router;
use stashden_core::role::Role;
use tower::ServiceExt;

use common::Harness;

async fn call(
    harness: &Harness,
    method: &str,
    uri: &str,
    bearer: &str,
    body: &str,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("a request");
    let response = build_router(harness.state.clone(), &[], None)
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn fingerprint(harness: &Harness, bearer: &str) -> String {
    let (status, body) = call(harness, "GET", "/v1/tree/fingerprint", bearer, "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["fingerprint"]
        .as_str()
        .expect("a fingerprint")
        .to_owned()
}

database_test!(an_untouched_tree_keeps_its_fingerprint, harness, {
    let owner = harness.account("owner@example.com", Role::Member).await;
    let bearer = harness.session(owner.id).await;
    harness.write(owner.id, "a/one.txt", b"1").await;

    let first = fingerprint(&harness, &bearer).await;
    let second = fingerprint(&harness, &bearer).await;

    assert_eq!(first, second);
});

database_test!(every_kind_of_change_moves_the_fingerprint, harness, {
    let owner = harness.account("owner@example.com", Role::Member).await;
    let bearer = harness.session(owner.id).await;
    harness.write(owner.id, "a/one.txt", b"1").await;
    harness.write(owner.id, "a/two.txt", b"2").await;
    let mut seen = vec![fingerprint(&harness, &bearer).await];

    async fn moved(harness: &Harness, bearer: &str, seen: &mut Vec<String>, what: &str) {
        let now = fingerprint(harness, bearer).await;
        assert!(
            !seen.contains(&now),
            "{what} left the fingerprint where it had been"
        );
        seen.push(now);
    }

    harness.write(owner.id, "a/three.txt", b"3").await;
    moved(&harness, &bearer, &mut seen, "adding a file").await;

    harness
        .write(owner.id, "a/one.txt", b"one, rewritten")
        .await;
    moved(&harness, &bearer, &mut seen, "overwriting a file").await;

    let (status, _) = call(
        &harness,
        "POST",
        "/v1/move",
        &bearer,
        r#"{"from":"a/two.txt","to":"a/renamed.txt"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    moved(&harness, &bearer, &mut seen, "renaming a file").await;

    let (status, _) = call(&harness, "DELETE", "/v1/files/a/three.txt", &bearer, "").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    moved(&harness, &bearer, &mut seen, "trashing a file").await;

    let (status, trash) = call(&harness, "GET", "/v1/trash", &bearer, "").await;
    assert_eq!(status, StatusCode::OK);
    let id = trash[0]["id"].as_str().expect("a trash id").to_owned();
    let (status, _) = call(
        &harness,
        "POST",
        &format!("/v1/trash/{id}/restore"),
        &bearer,
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    moved(&harness, &bearer, &mut seen, "restoring a file").await;

    let (status, _) = call(&harness, "DELETE", "/v1/files/a/renamed.txt", &bearer, "").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    seen.push(fingerprint(&harness, &bearer).await);
    let (status, _) = call(&harness, "DELETE", "/v1/trash", &bearer, "").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    moved(&harness, &bearer, &mut seen, "emptying the trash").await;
});

database_test!(
    a_change_inside_a_shared_folder_moves_the_recipients_fingerprint,
    harness,
    {
        let owner = harness.account("owner@example.com", Role::Member).await;
        let owner_bearer = harness.session(owner.id).await;
        let guest = harness.account("guest@example.com", Role::Member).await;
        let guest_bearer = harness.session(guest.id).await;
        harness.write(owner.id, "shared/a.txt", b"a").await;

        let before_grant = fingerprint(&harness, &guest_bearer).await;
        let (status, _) = call(
            &harness,
            "POST",
            "/v1/grants",
            &owner_bearer,
            r#"{"path":"shared","email":"guest@example.com","access":"read"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let granted = fingerprint(&harness, &guest_bearer).await;
        harness.write(owner.id, "shared/deeper/b.txt", b"b").await;
        let changed = fingerprint(&harness, &guest_bearer).await;

        assert_ne!(before_grant, granted);
        assert_ne!(granted, changed);
    }
);

database_test!(
    another_accounts_changes_do_not_move_this_fingerprint,
    harness,
    {
        let one = harness.account("one@example.com", Role::Member).await;
        let one_bearer = harness.session(one.id).await;
        let two = harness.account("two@example.com", Role::Member).await;
        let before = fingerprint(&harness, &one_bearer).await;

        harness.write(two.id, "theirs.txt", b"x").await;

        assert_eq!(before, fingerprint(&harness, &one_bearer).await);
    }
);
