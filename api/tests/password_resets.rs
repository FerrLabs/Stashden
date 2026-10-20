mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use stashden_api::build_router;
use stashden_api::mail::Letter;
use stashden_api::state::AppState;
use stashden_api::users;
use stashden_core::role::Role;
use stashden_core::user::Email;
use tokio::sync::mpsc::UnboundedReceiver;
use tower::ServiceExt;

use common::PASSWORD;

const NEW_PASSWORD: &str = "a-password-from-the-link";

async fn call(
    state: &AppState,
    method: &str,
    uri: &str,
    authorization: Option<&str>,
    body: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(authorization) = authorization {
        request = request.header(header::AUTHORIZATION, authorization);
    }
    let response = build_router(state.clone(), &[], None)
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
                .expect("a well formed request"),
        )
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

async fn forgot(state: &AppState, email: &str) -> StatusCode {
    call(
        state,
        "POST",
        "/v1/auth/password/forgot",
        None,
        &format!(r#"{{"email":"{email}"}}"#),
    )
    .await
    .0
}

async fn reset(state: &AppState, token: &str, password: &str) -> StatusCode {
    call(
        state,
        "POST",
        "/v1/auth/password/reset",
        None,
        &format!(r#"{{"token":"{token}","password":"{password}"}}"#),
    )
    .await
    .0
}

async fn login(state: &AppState, email: &str, password: &str) -> StatusCode {
    call(
        state,
        "POST",
        "/v1/auth/login",
        None,
        &format!(r#"{{"email":"{email}","password":"{password}"}}"#),
    )
    .await
    .0
}

async fn delivered(outbox: &mut UnboundedReceiver<Letter>) -> Letter {
    tokio::time::timeout(Duration::from_secs(5), outbox.recv())
        .await
        .expect("the link is mailed")
        .expect("the outbox is open")
}

fn token_in(letter: &Letter) -> String {
    let (_, after) = letter
        .body
        .split_once("https://files.example.com/#/reset-password/")
        .expect("the letter carries the link");
    after
        .split_whitespace()
        .next()
        .expect("a token after the link")
        .to_owned()
}

database_test!(
    a_forgotten_password_is_replaced_through_the_mailed_link,
    harness,
    {
        harness.account("forgot@example.com", Role::Member).await;
        let (state, mut outbox) = harness.with_mail();

        assert_eq!(
            forgot(&state, "forgot@example.com").await,
            StatusCode::ACCEPTED
        );
        let letter = delivered(&mut outbox).await;
        assert_eq!(letter.to.as_str(), "forgot@example.com");

        let token = token_in(&letter);
        assert_eq!(
            reset(&state, &token, NEW_PASSWORD).await,
            StatusCode::NO_CONTENT
        );

        assert_eq!(
            login(&state, "forgot@example.com", NEW_PASSWORD).await,
            StatusCode::OK
        );
        assert_eq!(
            login(&state, "forgot@example.com", PASSWORD).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            reset(&state, &token, "yet-another-password").await,
            StatusCode::BAD_REQUEST,
            "a link opens once"
        );
    }
);

database_test!(
    an_address_without_a_password_account_gets_the_same_answer_and_no_link,
    harness,
    {
        let disabled = harness.account("disabled@example.com", Role::Member).await;
        users::set_disabled(&harness.state.db, disabled.id, true)
            .await
            .expect("disabling");
        let mut tx = harness.state.db.begin().await.expect("begin");
        users::create_provisioned(
            &mut tx,
            &"provider@example.com".parse::<Email>().expect("valid"),
            "Provider",
            "a-random-password-nobody-knows",
            Role::Member,
        )
        .await
        .expect("provisioning");
        tx.commit().await.expect("commit");
        let (state, _outbox) = harness.with_mail();

        for email in [
            "nobody@example.com",
            "disabled@example.com",
            "provider@example.com",
        ] {
            assert_eq!(forgot(&state, email).await, StatusCode::ACCEPTED, "{email}");
        }
        assert_eq!(harness.reset_rows().await, 0);
    }
);

database_test!(a_reset_signs_out_every_session, harness, {
    let user = harness.account("stolen@example.com", Role::Member).await;
    let bearer = format!("Bearer {}", harness.session(user.id).await);
    let (state, mut outbox) = harness.with_mail();

    assert_eq!(
        call(&state, "GET", "/v1/auth/me", Some(&bearer), "")
            .await
            .0,
        StatusCode::OK
    );

    forgot(&state, "stolen@example.com").await;
    let token = token_in(&delivered(&mut outbox).await);
    assert_eq!(
        reset(&state, &token, NEW_PASSWORD).await,
        StatusCode::NO_CONTENT
    );

    assert_eq!(
        call(&state, "GET", "/v1/auth/me", Some(&bearer), "")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
});

database_test!(a_newer_link_voids_the_older_one, harness, {
    harness.account("twice@example.com", Role::Member).await;
    let (state, mut outbox) = harness.with_mail();

    forgot(&state, "twice@example.com").await;
    let first = token_in(&delivered(&mut outbox).await);
    forgot(&state, "twice@example.com").await;
    let second = token_in(&delivered(&mut outbox).await);

    assert_eq!(
        reset(&state, &first, NEW_PASSWORD).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        reset(&state, &second, NEW_PASSWORD).await,
        StatusCode::NO_CONTENT
    );
});

database_test!(an_expired_link_is_refused_and_swept, harness, {
    harness.account("late@example.com", Role::Member).await;
    let (state, mut outbox) = harness.with_mail();

    forgot(&state, "late@example.com").await;
    let token = token_in(&delivered(&mut outbox).await);
    harness.expire_resets().await;

    assert_eq!(
        reset(&state, &token, NEW_PASSWORD).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        stashden_api::password_resets::purge_expired(&harness.state.db)
            .await
            .expect("purging"),
        1
    );
    assert_eq!(harness.reset_rows().await, 0);
});

database_test!(
    a_link_mailed_to_a_previous_address_stops_working,
    harness,
    {
        let user = harness.account("old@example.com", Role::Member).await;
        let (state, mut outbox) = harness.with_mail();

        forgot(&state, "old@example.com").await;
        let token = token_in(&delivered(&mut outbox).await);
        users::set_email(
            &harness.state.db,
            user.id,
            &"new@example.com".parse::<Email>().expect("valid"),
        )
        .await
        .expect("changing the address");

        assert_eq!(
            reset(&state, &token, NEW_PASSWORD).await,
            StatusCode::BAD_REQUEST
        );
    }
);

database_test!(a_password_changed_another_way_voids_the_link, harness, {
    let user = harness.account("changed@example.com", Role::Member).await;
    let (state, mut outbox) = harness.with_mail();

    forgot(&state, "changed@example.com").await;
    let token = token_in(&delivered(&mut outbox).await);
    users::set_password(&harness.state.db, user.id, "changed-by-the-owner")
        .await
        .expect("changing the password");

    assert_eq!(
        reset(&state, &token, NEW_PASSWORD).await,
        StatusCode::BAD_REQUEST
    );
});

database_test!(a_weak_password_leaves_the_link_usable, harness, {
    harness.account("weak@example.com", Role::Member).await;
    let (state, mut outbox) = harness.with_mail();

    forgot(&state, "weak@example.com").await;
    let token = token_in(&delivered(&mut outbox).await);

    assert_eq!(
        reset(&state, &token, "short").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        reset(&state, &token, NEW_PASSWORD).await,
        StatusCode::NO_CONTENT
    );
});

database_test!(a_reset_lifts_a_sign_in_lockout, harness, {
    harness.account("locked@example.com", Role::Member).await;
    let (state, mut outbox) = harness.with_mail();

    let mut status = StatusCode::UNAUTHORIZED;
    while status != StatusCode::TOO_MANY_REQUESTS {
        status = login(&state, "locked@example.com", "a-wrong-password").await;
    }

    forgot(&state, "locked@example.com").await;
    let token = token_in(&delivered(&mut outbox).await);
    assert_eq!(
        reset(&state, &token, NEW_PASSWORD).await,
        StatusCode::NO_CONTENT
    );

    assert_eq!(
        login(&state, "locked@example.com", NEW_PASSWORD).await,
        StatusCode::OK
    );
});

database_test!(asking_for_links_is_rate_limited_per_address, harness, {
    let (state, _outbox) = harness.with_mail();

    let mut answers = Vec::new();
    for _ in 0..11 {
        answers.push(forgot(&state, "flooded@example.com").await);
    }

    assert!(
        answers[..10]
            .iter()
            .all(|status| *status == StatusCode::ACCEPTED)
    );
    assert_eq!(answers[10], StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        forgot(&state, "someone-else@example.com").await,
        StatusCode::ACCEPTED
    );
});

database_test!(without_mail_the_reset_routes_do_not_exist, harness, {
    harness.account("nomail@example.com", Role::Member).await;

    assert_eq!(
        forgot(&harness.state, "nomail@example.com").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        reset(&harness.state, "anything", NEW_PASSWORD).await,
        StatusCode::NOT_FOUND
    );
    let (_, methods) = call(&harness.state, "GET", "/v1/auth/methods", None, "").await;
    assert_eq!(methods["password_reset"], Value::Bool(false));

    let (state, _outbox) = harness.with_mail();
    let (_, methods) = call(&state, "GET", "/v1/auth/methods", None, "").await;
    assert_eq!(methods["password_reset"], Value::Bool(true));
});

database_test!(turning_passwords_off_turns_resets_off, harness, {
    harness.account("off@example.com", Role::Member).await;
    stashden_api::settings::allow_password_login(&harness.state.db, false)
        .await
        .expect("turning passwords off");
    let (state, _outbox) = harness.with_mail();

    assert_eq!(
        forgot(&state, "off@example.com").await,
        StatusCode::NOT_FOUND
    );
    let (_, methods) = call(&state, "GET", "/v1/auth/methods", None, "").await;
    assert_eq!(methods["password_reset"], Value::Bool(false));
});
