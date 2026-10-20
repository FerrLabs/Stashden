mod common;

use std::io::{Cursor, Read};

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use stashden_api::build_router;
use stashden_core::role::Role;
use tower::ServiceExt;
use zip::ZipArchive;

use common::Harness;

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

async fn call(
    harness: &Harness,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: &str,
) -> Answer {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = build_router(harness.state.clone(), &[], None)
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
                .expect("a request"),
        )
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

fn entries(answer: &Answer) -> Vec<(String, Vec<u8>)> {
    let mut archive = ZipArchive::new(Cursor::new(answer.body.clone())).expect("a zip");
    let mut found = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("an entry");
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("its bytes");
        found.push((entry.name().expect("a utf-8 name").into_owned(), bytes));
    }
    found.sort();
    found
}

async fn account(harness: &Harness, email: &str, role: Role) -> (uuid::Uuid, String) {
    let user = harness.account(email, role).await;
    let token = harness.session(user.id).await;
    (user.id, token)
}

database_test!(a_folder_comes_back_as_a_zip_with_its_whole_tree, harness, {
    let (owner, bearer) = account(&harness, "owner@example.com", Role::Member).await;
    harness.write(owner, "photos/a.jpg", b"first").await;
    harness.write(owner, "photos/2024/b.jpg", b"second").await;
    harness.write(owner, "photos/2024/empty.txt", b"").await;
    harness
        .write(owner, "elsewhere/c.txt", b"not included")
        .await;

    let answer = call(&harness, "GET", "/v1/archive/photos", Some(&bearer), "").await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.headers[header::CONTENT_TYPE], "application/zip");
    assert!(
        answer.headers[header::CONTENT_DISPOSITION]
            .to_str()
            .expect("text")
            .contains("photos.zip")
    );
    assert_eq!(
        entries(&answer),
        [
            ("photos/".to_owned(), Vec::new()),
            ("photos/2024/".to_owned(), Vec::new()),
            ("photos/2024/b.jpg".to_owned(), b"second".to_vec()),
            ("photos/2024/empty.txt".to_owned(), Vec::new()),
            ("photos/a.jpg".to_owned(), b"first".to_vec()),
        ]
    );
});

database_test!(what_is_in_the_trash_stays_out_of_the_archive, harness, {
    let (owner, bearer) = account(&harness, "owner@example.com", Role::Member).await;
    harness.write(owner, "docs/keep.txt", b"keep").await;
    harness.write(owner, "docs/gone.txt", b"gone").await;
    let deleted = call(
        &harness,
        "DELETE",
        "/v1/files/docs/gone.txt",
        Some(&bearer),
        "",
    )
    .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);

    let answer = call(&harness, "GET", "/v1/archive/docs", Some(&bearer), "").await;

    let names: Vec<String> = entries(&answer).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["docs/", "docs/keep.txt"]);
});

database_test!(a_file_is_not_a_folder_to_archive, harness, {
    let (owner, bearer) = account(&harness, "owner@example.com", Role::Member).await;
    harness.write(owner, "notes.txt", b"hi").await;

    let answer = call(&harness, "GET", "/v1/archive/notes.txt", Some(&bearer), "").await;

    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
});

database_test!(somebody_elses_folder_is_not_found, harness, {
    let (owner, _) = account(&harness, "owner@example.com", Role::Member).await;
    let (_, other) = account(&harness, "other@example.com", Role::Member).await;
    harness.write(owner, "private/a.txt", b"mine").await;

    let answer = call(&harness, "GET", "/v1/archive/private", Some(&other), "").await;

    assert_eq!(answer.status, StatusCode::NOT_FOUND);
});

database_test!(
    a_folder_shared_with_an_account_can_be_archived_by_it,
    harness,
    {
        let (owner, owner_bearer) = account(&harness, "owner@example.com", Role::Member).await;
        let (_, guest) = account(&harness, "guest@example.com", Role::Member).await;
        harness.write(owner, "photos/a.jpg", b"shared").await;
        let shared = call(
            &harness,
            "POST",
            "/v1/grants",
            Some(&owner_bearer),
            r#"{"path":"photos","email":"guest@example.com","access":"read"}"#,
        )
        .await;
        assert_eq!(shared.status, StatusCode::CREATED);

        let answer = call(
            &harness,
            "GET",
            "/v1/archive/Shared%20with%20me/photos",
            Some(&guest),
            "",
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            entries(&answer),
            [
                ("photos/".to_owned(), Vec::new()),
                ("photos/a.jpg".to_owned(), b"shared".to_vec()),
            ]
        );
    }
);

database_test!(
    a_link_to_a_folder_serves_an_archive_to_a_visitor,
    harness,
    {
        let (owner, bearer) = account(&harness, "owner@example.com", Role::Member).await;
        harness.write(owner, "pack/inner/a.txt", b"inside").await;
        let minted = call(
            &harness,
            "POST",
            "/v1/shares",
            Some(&bearer),
            r#"{"path":"pack"}"#,
        )
        .await;
        assert_eq!(minted.status, StatusCode::CREATED);
        let body = String::from_utf8(minted.body).expect("text");
        let token = body
            .split("\"token\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("a token");

        let whole = call(
            &harness,
            "GET",
            &format!("/v1/public/{token}/archive"),
            None,
            "",
        )
        .await;
        let below = call(
            &harness,
            "GET",
            &format!("/v1/public/{token}/archive/inner"),
            None,
            "",
        )
        .await;

        assert_eq!(whole.status, StatusCode::OK);
        assert_eq!(whole.headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            entries(&whole)
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["pack/", "pack/inner/", "pack/inner/a.txt"]
        );
        assert_eq!(
            entries(&below)
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["inner/", "inner/a.txt"]
        );
    }
);
