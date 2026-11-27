mod common;

use axum::http::StatusCode;
use stashden_core::role::Role;

use common::Harness;
use common::browser::{VERIFIER, approved_code, redeem, start};
use common::capture::capture_logs;

#[tokio::test]
async fn a_full_flow_logs_neither_the_code_nor_the_verifier_nor_the_secret() {
    let Some(harness) = Harness::start().await else {
        eprintln!("skipping: DATABASE_URL is not set");
        return;
    };
    let captured = capture_logs();
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
