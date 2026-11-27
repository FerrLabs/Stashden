mod common;

use axum::http::StatusCode;
use serde_json::json;
use stashden_api::{app_passwords, browser_sign_ins, users};
use stashden_core::role::Role;

use common::Harness;
use common::browser::{
    Auth, PORT, VERIFIER, approve, approved_code, challenge_for, deny, read, redeem, send, start,
    start_with, unknown_id,
};

async fn rows(harness: &Harness) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM browser_sign_ins")
        .fetch_one(&harness.state.db)
        .await
        .expect("counting the requests")
}

async fn expire(harness: &Harness) {
    sqlx::query("UPDATE browser_sign_ins SET expires_at = now() - INTERVAL '1 second'")
        .execute(&harness.state.db)
        .await
        .expect("expiring the requests");
}

database_test!(
    a_request_ends_in_an_app_password_named_after_the_computer,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;

        let started = start_with(
            &harness.state,
            json!({
                "device_name": "Ada's laptop",
                "challenge": challenge_for(VERIFIER),
                "loopback_port": PORT,
            }),
        )
        .await;
        assert_eq!(started.status, StatusCode::CREATED);
        assert_eq!(started.body["expires_in"], 900);
        assert_eq!(started.body["confirmation"].as_str().map(str::len), Some(6));
        let id = started.body["id"].as_str().expect("an id");

        let waiting = read(&harness.state, id, &session).await;
        assert_eq!(waiting.status, StatusCode::OK);
        assert_eq!(waiting.body["device_name"], "Ada's laptop");
        assert_eq!(waiting.body["confirmation"], started.body["confirmation"]);

        let approved = approve(&harness.state, id, &session).await;
        assert_eq!(approved.status, StatusCode::OK);
        let callback = approved.body["callback"].as_str().expect("a callback");
        assert!(
            callback.starts_with(&format!("http://127.0.0.1:{PORT}/callback?id={id}&code=")),
            "{callback}"
        );
        let code = common::browser::code_in(callback);

        let redeemed = redeem(&harness.state, id, &code, VERIFIER).await;
        assert_eq!(redeemed.status, StatusCode::OK, "{}", redeemed.body);
        assert_eq!(redeemed.body["name"], "Ada's laptop");
        assert_eq!(redeemed.body["user"]["email"], "ada@example.com");
        let secret = redeemed.body["secret"].as_str().expect("a secret");
        assert!(
            redeemed.body.get("token").is_none(),
            "no session token travels"
        );

        let listed = send(
            &harness.state,
            "GET",
            "/v1/app-passwords",
            &Auth::Session(&session),
            None,
        )
        .await;
        assert_eq!(listed.body.as_array().map(Vec::len), Some(1));
        assert_eq!(listed.body[0]["name"], "Ada's laptop");
        assert_eq!(listed.body[0]["id"], redeemed.body["id"]);

        let me = send(
            &harness.state,
            "GET",
            "/v1/auth/me",
            &Auth::AppPassword("ada@example.com", secret),
            None,
        )
        .await;
        assert_eq!(me.status, StatusCode::OK, "the minted secret works");

        let id_of_password = redeemed.body["id"].as_str().expect("an id");
        let revoked = send(
            &harness.state,
            "DELETE",
            &format!("/v1/app-passwords/{id_of_password}"),
            &Auth::Session(&session),
            None,
        )
        .await;
        assert_eq!(revoked.status, StatusCode::NO_CONTENT);
        let me = send(
            &harness.state,
            "GET",
            "/v1/auth/me",
            &Auth::AppPassword("ada@example.com", secret),
            None,
        )
        .await;
        assert_eq!(me.status, StatusCode::UNAUTHORIZED);
    }
);

database_test!(an_unknown_request_is_refused_everywhere, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = unknown_id();

    assert_eq!(
        read(&harness.state, &id, &session).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        approve(&harness.state, &id, &session).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        deny(&harness.state, &id, &session).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        redeem(&harness.state, &id, "code", VERIFIER).await.status,
        StatusCode::UNAUTHORIZED
    );
});

database_test!(an_expired_request_is_gone_for_every_route, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = start(&harness.state).await;
    let code = approved_code(&harness.state, &id, &session).await;
    expire(&harness).await;

    assert_eq!(
        read(&harness.state, &id, &session).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        approve(&harness.state, &id, &session).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        redeem(&harness.state, &id, &code, VERIFIER).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app_passwords::list(&harness.state.db, ada.id)
            .await
            .expect("listing")
            .len(),
        0
    );
});

database_test!(a_request_nobody_approved_cannot_be_redeemed, harness, {
    let id = start(&harness.state).await;

    let reply = redeem(&harness.state, &id, "anything", VERIFIER).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
});

database_test!(
    the_wrong_code_or_the_wrong_verifier_gets_nothing,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &session).await;

        let wrong_code = redeem(&harness.state, &id, &"0".repeat(code.len()), VERIFIER).await;
        let wrong_verifier = redeem(&harness.state, &id, &code, "not-the-verifier").await;

        assert_eq!(wrong_code.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong_verifier.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong_code.body, wrong_verifier.body);
        assert_eq!(
            app_passwords::list(&harness.state.db, ada.id)
                .await
                .expect("listing")
                .len(),
            0
        );
        let right = redeem(&harness.state, &id, &code, VERIFIER).await;
        assert_eq!(
            right.status,
            StatusCode::OK,
            "two mistakes do not spend the request"
        );
    }
);

database_test!(an_account_disabled_after_approval_gets_nothing, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = start(&harness.state).await;
    let code = approved_code(&harness.state, &id, &session).await;
    users::set_disabled(&harness.state.db, ada.id, true)
        .await
        .expect("disabling");

    let reply = redeem(&harness.state, &id, &code, VERIFIER).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        app_passwords::list(&harness.state.db, ada.id)
            .await
            .expect("listing")
            .len(),
        0
    );
});

database_test!(an_app_password_cannot_approve, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let mut tx = harness.state.db.begin().await.expect("begin");
    let minted = app_passwords::mint(&mut tx, ada.id, "a client")
        .await
        .expect("minting");
    tx.commit().await.expect("commit");
    let id = start(&harness.state).await;

    let reply = send(
        &harness.state,
        "POST",
        &format!("/v1/auth/browser/{id}/approve"),
        &Auth::AppPassword("ada@example.com", &minted.secret),
        None,
    )
    .await;

    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(rows(&harness).await, 1);
});

database_test!(
    a_request_approved_by_one_account_is_closed_to_another,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let eve = harness.account("eve@example.com", Role::Member).await;
        let ada_session = harness.session(ada.id).await;
        let eve_session = harness.session(eve.id).await;
        let id = start(&harness.state).await;
        approved_code(&harness.state, &id, &ada_session).await;

        assert_eq!(
            approve(&harness.state, &id, &eve_session).await.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            read(&harness.state, &id, &eve_session).await.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            deny(&harness.state, &id, &eve_session).await.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            approve(&harness.state, &id, &ada_session).await.status,
            StatusCode::CONFLICT
        );
    }
);

database_test!(
    the_sixth_attempt_after_five_failures_finds_no_request,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;
        let code = approved_code(&harness.state, &id, &session).await;

        for _ in 0..browser_sign_ins::MOST_FAILURES {
            let reply = redeem(&harness.state, &id, "wrong", VERIFIER).await;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        }

        assert_eq!(rows(&harness).await, 0);
        assert_eq!(
            redeem(&harness.state, &id, &code, VERIFIER).await.status,
            StatusCode::UNAUTHORIZED,
            "the right answer arrives too late"
        );
    }
);

database_test!(a_redeemed_request_cannot_be_redeemed_again, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = start(&harness.state).await;
    let code = approved_code(&harness.state, &id, &session).await;

    let first = redeem(&harness.state, &id, &code, VERIFIER).await;
    let second = redeem(&harness.state, &id, &code, VERIFIER).await;

    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(second.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        app_passwords::list(&harness.state.db, ada.id)
            .await
            .expect("listing")
            .len(),
        1
    );
});

database_test!(two_concurrent_redeems_mint_one_app_password, harness, {
    let ada = harness.account("ada@example.com", Role::Member).await;
    let session = harness.session(ada.id).await;
    let id = start(&harness.state).await;
    let code = approved_code(&harness.state, &id, &session).await;

    let (first, second) = tokio::join!(
        redeem(&harness.state, &id, &code, VERIFIER),
        redeem(&harness.state, &id, &code, VERIFIER),
    );

    let mut statuses = [first.status, second.status];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::UNAUTHORIZED]);
    assert_eq!(
        app_passwords::list(&harness.state.db, ada.id)
            .await
            .expect("listing")
            .len(),
        1
    );
});

database_test!(
    denying_answers_a_callback_without_a_code_and_deletes_the_request,
    harness,
    {
        let ada = harness.account("ada@example.com", Role::Member).await;
        let session = harness.session(ada.id).await;
        let id = start(&harness.state).await;

        let reply = deny(&harness.state, &id, &session).await;

        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(
            reply.body["callback"],
            format!("http://127.0.0.1:{PORT}/callback?id={id}&error=denied")
        );
        assert_eq!(rows(&harness).await, 0);
    }
);

database_test!(sign_in_methods_advertise_the_browser, harness, {
    let reply = send(
        &harness.state,
        "GET",
        "/v1/auth/methods",
        &Auth::Nobody,
        None,
    )
    .await;

    assert_eq!(reply.body["browser"], true);
});

database_test!(
    the_sweep_removes_expired_requests_and_keeps_live_ones,
    harness,
    {
        start(&harness.state).await;
        expire(&harness).await;
        start(&harness.state).await;

        let purged = browser_sign_ins::purge_expired(&harness.state.db)
            .await
            .expect("purging");

        assert_eq!(purged, 1);
        assert_eq!(rows(&harness).await, 1);
    }
);
