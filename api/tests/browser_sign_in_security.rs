mod common;

use axum::http::StatusCode;
use serde_json::json;
use stashden_api::{app_passwords, browser_sign_ins};
use stashden_core::role::Role;

use common::browser::{
    Auth, PORT, VERIFIER, approve, approved_code, challenge_for, deny, read, redeem, send, start,
    start_with, unknown_id,
};
use common::capture::capture_logs;
use common::{Harness, PASSWORD};

const NEW_PASSWORD: &str = "another-twelve-characters";

async fn minted_for(harness: &Harness, user: uuid::Uuid) -> usize {
    app_passwords::list(&harness.state.db, user)
        .await
        .expect("listing")
        .len()
}

async fn rows(harness: &Harness) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM browser_sign_ins")
        .fetch_one(&harness.state.db)
        .await
        .expect("counting the requests")
}

async fn approved_rows(harness: &Harness) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM browser_sign_ins WHERE user_id IS NOT NULL")
        .fetch_one(&harness.state.db)
        .await
        .expect("counting the approved requests")
}

async fn fill_to_the_cap(harness: &Harness) {
    sqlx::query(
        "INSERT INTO browser_sign_ins
             (id, challenge, device_name, loopback_port, confirmation, expires_at)
         SELECT gen_random_uuid(), repeat('a', 43), 'filler', 4000, 'BBBBBB',
                now() + INTERVAL '10 minutes'
         FROM generate_series(1, $1::int)",
    )
    .bind(i32::try_from(browser_sign_ins::MOST_LIVE).expect("a small cap"))
    .execute(&harness.state.db)
    .await
    .expect("filling the table");
}

database_test!(
    a_second_approval_neither_replaces_the_code_nor_the_account,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let eve = harness.account("eve@example.com", Role::Member).await;
        let ada_session = harness.session(ada.id).await;
        let eve_session = harness.session(eve.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &ada_session).await;

        let again = approve(&harness.state, &id, &ada_session).await;
        let hijack = approve(&harness.state, &id, &eve_session).await;

        assert_eq!(again.status, StatusCode::CONFLICT);
        assert!(again.body.get("callback").is_none());
        assert_eq!(hijack.status, StatusCode::NOT_FOUND);
        let redeemed = redeem(&harness.state, &id, &code, VERIFIER).await;
        assert_eq!(redeemed.status, StatusCode::OK, "{}", redeemed.body);
        assert_eq!(redeemed.body["user"]["email"], "ada@example.com");
        assert_eq!(minted_for(&harness, eve.id).await, 0);
    }
);

database_test!(
    a_redeemed_code_is_dead_even_with_the_right_verifier,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &session).await;
        assert_eq!(
            redeem(&harness.state, &id, &code, VERIFIER).await.status,
            StatusCode::OK
        );

        for _ in 0..3 {
            let replay = redeem(&harness.state, &id, &code, VERIFIER).await;
            assert_eq!(replay.status, StatusCode::UNAUTHORIZED);
        }

        assert_eq!(minted_for(&harness, ada.id).await, 1);
    }
);

database_test!(
    a_code_approved_for_one_request_does_not_redeem_another,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let first = start(&harness.state).await;
        let second = start(&harness.state).await;
        let first_code = approved_code(&harness.state, &first, &session).await;
        let second_code = approved_code(&harness.state, &second, &session).await;

        let crossed = redeem(&harness.state, &second, &first_code, VERIFIER).await;

        assert_eq!(crossed.status, StatusCode::UNAUTHORIZED);
        assert_eq!(minted_for(&harness, ada.id).await, 0);
        let own = redeem(&harness.state, &second, &second_code, VERIFIER).await;
        assert_eq!(own.status, StatusCode::OK);
    }
);

database_test!(
    a_verifier_that_is_not_behind_the_challenge_does_not_redeem,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let started = start_with(
            &harness.state,
            json!({
                "device_name": "Ada's laptop",
                "challenge": challenge_for("a-verifier-only-the-client-knows-0123456789"),
                "loopback_port": PORT,
            }),
        )
        .await;
        let id = started.body["id"].as_str().expect("an id");
        let code = approved_code(&harness.state, id, &session).await;

        let stolen = redeem(&harness.state, id, &code, VERIFIER).await;
        let echoed = redeem(&harness.state, id, &code, &challenge_for(VERIFIER)).await;

        assert_eq!(stolen.status, StatusCode::UNAUTHORIZED);
        assert_eq!(echoed.status, StatusCode::UNAUTHORIZED);
        assert_eq!(minted_for(&harness, ada.id).await, 0);
    }
);

database_test!(
    a_code_approved_before_expiry_does_not_outlive_the_request,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &session).await;
        sqlx::query("UPDATE browser_sign_ins SET expires_at = now() - INTERVAL '1 second'")
            .execute(&harness.state.db)
            .await
            .expect("expiring the request");

        let late = redeem(&harness.state, &id, &code, VERIFIER).await;

        assert_eq!(late.status, StatusCode::UNAUTHORIZED);
        assert_eq!(minted_for(&harness, ada.id).await, 0);
    }
);

database_test!(
    an_account_deleted_after_approval_leaves_nothing_to_redeem,
    harness,
    {
        let admin = harness.account("root@example.com", Role::Admin).await;
        let ada = harness.account("ada@example.com", Role::Member).await;
        let admin_session = harness.session(admin.id).await;
        let ada_session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &ada_session).await;

        let removed = send(
            &harness.state,
            "DELETE",
            &format!("/v1/users/{}", ada.id),
            &Auth::Session(&admin_session),
            None,
        )
        .await;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);

        let reply = redeem(&harness.state, &id, &code, VERIFIER).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(rows(&harness).await, 0);
    }
);

database_test!(
    changing_the_password_between_approval_and_redeem_leaves_no_app_password,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &session).await;

        let changed = send(
            &harness.state,
            "PUT",
            "/v1/auth/password",
            &Auth::Session(&session),
            Some(json!({ "current": PASSWORD, "password": NEW_PASSWORD })),
        )
        .await;
        assert_eq!(changed.status, StatusCode::NO_CONTENT, "{}", changed.body);

        let reply = redeem(&harness.state, &id, &code, VERIFIER).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(minted_for(&harness, ada.id).await, 0);
    }
);

database_test!(
    an_administrator_resetting_the_password_cancels_pending_requests,
    harness,
    {
        let admin = harness.account("root@example.com", Role::Admin).await;
        let ada = harness.account("ada@example.com", Role::Member).await;
        let admin_session = harness.session(admin.id).await;
        let ada_session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &ada_session).await;

        let reset = send(
            &harness.state,
            "PUT",
            &format!("/v1/users/{}/password", ada.id),
            &Auth::Session(&admin_session),
            Some(json!({ "password": NEW_PASSWORD })),
        )
        .await;
        assert_eq!(reset.status, StatusCode::NO_CONTENT, "{}", reset.body);

        let reply = redeem(&harness.state, &id, &code, VERIFIER).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(minted_for(&harness, ada.id).await, 0);
    }
);

database_test!(only_a_session_reads_approves_or_denies, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let mut tx = harness.state.db.begin().await.expect("begin");
    let minted = app_passwords::mint(&mut tx, ada.id, "a client")
        .await
        .expect("minting");
    tx.commit().await.expect("commit");
    let id = start(&harness.state).await;
    let app = Auth::AppPassword("ada@example.com", &minted.secret);

    for (method, uri) in [
        ("GET", format!("/v1/auth/browser/{id}")),
        ("POST", format!("/v1/auth/browser/{id}/approve")),
        ("DELETE", format!("/v1/auth/browser/{id}")),
    ] {
        let with_app_password = send(&harness.state, method, &uri, &app, None).await;
        assert_eq!(
            with_app_password.status,
            StatusCode::FORBIDDEN,
            "{method} {uri}"
        );
        let with_nothing = send(&harness.state, method, &uri, &Auth::Nobody, None).await;
        assert_eq!(
            with_nothing.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }

    assert_eq!(rows(&harness).await, 1, "none of them deleted the request");
    assert_eq!(approved_rows(&harness).await, 0, "none of them approved it");
});

database_test!(unknown_and_wrong_answers_look_the_same, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = start(&harness.state).await;
    approved_code(&harness.state, &id, &session).await;

    let known = redeem(&harness.state, &id, "wrong", VERIFIER).await;
    let unknown = redeem(&harness.state, &unknown_id(), "wrong", VERIFIER).await;

    assert_eq!(known.status, StatusCode::UNAUTHORIZED);
    assert_eq!(known.status, unknown.status);
    assert_eq!(known.body, unknown.body);
});

database_test!(
    five_wrong_codes_delete_only_the_request_they_were_aimed_at,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let target = start(&harness.state).await;
        let bystander = start(&harness.state).await;
        let bystander_code = approved_code(&harness.state, &bystander, &session).await;

        for _ in 0..browser_sign_ins::MOST_FAILURES {
            redeem(&harness.state, &target, "wrong", VERIFIER).await;
        }

        assert_eq!(
            read(&harness.state, &target, &session).await.status,
            StatusCode::NOT_FOUND
        );
        let own = redeem(&harness.state, &bystander, &bystander_code, VERIFIER).await;
        assert_eq!(own.status, StatusCode::OK);
    }
);

database_test!(guessing_at_unknown_requests_stores_nothing, harness, {
    for _ in 0..10 {
        redeem(&harness.state, &unknown_id(), "wrong", VERIFIER).await;
    }

    assert_eq!(rows(&harness).await, 0);
});

database_test!(
    a_request_outside_the_bounds_is_refused_and_stores_nothing,
    harness,
    {
        let challenge = challenge_for(VERIFIER);
        let refused = [
            json!({ "device_name": "", "challenge": challenge, "loopback_port": PORT }),
            json!({ "device_name": "   ", "challenge": challenge, "loopback_port": PORT }),
            json!({ "device_name": "x".repeat(101), "challenge": challenge, "loopback_port": PORT }),
            json!({ "device_name": "line\nbreak", "challenge": challenge, "loopback_port": PORT }),
            json!({ "device_name": "laptop", "challenge": "", "loopback_port": PORT }),
            json!({ "device_name": "laptop", "challenge": "short", "loopback_port": PORT }),
            json!({ "device_name": "laptop", "challenge": format!("{challenge}="), "loopback_port": PORT }),
            json!({ "device_name": "laptop", "challenge": "+".repeat(43), "loopback_port": PORT }),
            json!({ "device_name": "laptop", "challenge": challenge, "loopback_port": 0 }),
            json!({ "device_name": "laptop", "challenge": challenge, "loopback_port": 80 }),
            json!({ "device_name": "laptop", "challenge": challenge, "loopback_port": 1023 }),
            json!({ "device_name": "laptop", "challenge": challenge, "loopback_port": 65536 }),
            json!({ "device_name": "laptop", "challenge": challenge, "loopback_port": -1 }),
        ];

        for body in refused {
            let reply = start_with(&harness.state, body.clone()).await;
            assert!(reply.status.is_client_error(), "{body}: {}", reply.status);
        }

        assert_eq!(rows(&harness).await, 0);
        for port in [1024, 65535] {
            let reply = start_with(
            &harness.state,
            json!({ "device_name": "x".repeat(100), "challenge": challenge, "loopback_port": port }),
        )
        .await;
            assert_eq!(reply.status, StatusCode::CREATED, "port {port}");
        }
    }
);

database_test!(
    the_live_request_cap_answers_503_until_requests_expire,
    harness,
    {
        fill_to_the_cap(&harness).await;

        let refused = start_with(
        &harness.state,
        json!({ "device_name": "laptop", "challenge": challenge_for(VERIFIER), "loopback_port": PORT }),
    )
    .await;

        assert_eq!(refused.status, StatusCode::SERVICE_UNAVAILABLE);
        sqlx::query("UPDATE browser_sign_ins SET expires_at = now() - INTERVAL '1 second'")
            .execute(&harness.state.db)
            .await
            .expect("expiring the requests");
        let accepted = start_with(
        &harness.state,
        json!({ "device_name": "laptop", "challenge": challenge_for(VERIFIER), "loopback_port": PORT }),
    )
    .await;
        assert_eq!(
            accepted.status,
            StatusCode::CREATED,
            "expired requests do not count"
        );
    }
);

database_test!(
    the_callback_only_ever_points_at_loopback_on_the_stored_port,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;

        let reply = send(
        &harness.state,
        "POST",
        &format!("/v1/auth/browser/{id}/approve?loopback_port=9999&host=evil.example&callback=http://evil.example"),
        &Auth::Session(&session),
        Some(json!({ "loopback_port": 9999, "host": "evil.example", "callback": "http://evil.example" })),
    )
    .await;

        assert_eq!(reply.status, StatusCode::OK);
        let callback = reply.body["callback"].as_str().expect("a callback");
        assert!(
            callback.starts_with(&format!("http://127.0.0.1:{PORT}/callback?id={id}&code=")),
            "{callback}"
        );
        let denied = deny(&harness.state, &start(&harness.state).await, &session).await;
        let denied = denied.body["callback"].as_str().expect("a callback");
        assert!(
            denied.starts_with(&format!("http://127.0.0.1:{PORT}/callback?id=")),
            "{denied}"
        );
        assert!(denied.ends_with("&error=denied"), "{denied}");
        assert!(!denied.contains("code="), "{denied}");
    }
);

#[tokio::test]
async fn a_full_flow_logs_neither_the_code_nor_the_verifier_nor_the_secret() {
    let Some(harness) = Harness::start().await else {
        eprintln!("skipping: DATABASE_URL is not set");
        return;
    };
    let (captured, guard) = capture_logs();
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = start(&harness.state).await;
    let code = approved_code(&harness.state, &id, &session).await;
    let wrong = redeem(
        &harness.state,
        &id,
        "a-guess-that-must-not-be-logged",
        VERIFIER,
    )
    .await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    let redeemed = redeem(&harness.state, &id, &code, VERIFIER).await;
    assert_eq!(redeemed.status, StatusCode::OK);
    let secret = redeemed.body["secret"]
        .as_str()
        .expect("a secret")
        .to_owned();
    let app_password_id = redeemed.body["id"].as_str().expect("an id").to_owned();
    drop(guard);

    let logs = captured.text();
    assert!(logs.contains("browser sign-in redeemed"), "{logs}");
    assert!(
        logs.contains(&id) && logs.contains(&app_password_id),
        "{logs}"
    );
    assert!(logs.contains(&ada.id.to_string()), "{logs}");
    for secret_value in [
        code.as_str(),
        VERIFIER,
        secret.as_str(),
        "a-guess-that-must-not-be-logged",
    ] {
        assert!(
            !logs.contains(secret_value),
            "{secret_value} reached the logs"
        );
    }
    harness.stop().await;
}
