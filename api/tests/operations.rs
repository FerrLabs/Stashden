mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use stashden_api::build_router;
use stashden_api::state::AppState;
use tower::ServiceExt;

async fn get(state: &AppState, path: &str, bearer: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::builder().method("GET").uri(path);
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = build_router(state.clone(), &[], None)
        .oneshot(request.body(Body::empty()).expect("a request"))
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

database_test!(
    readiness_is_ok_when_the_database_and_the_blobs_answer,
    harness,
    {
        let (status, body) = get(&harness.state, "/ready", None).await;

        assert_eq!(status, StatusCode::OK, "{body}");
        let reported: Value = serde_json::from_str(&body).expect("json");
        assert_eq!(reported["database"], true);
        assert_eq!(reported["blobs"], true);
    }
);

database_test!(readiness_fails_when_the_blob_store_has_gone, harness, {
    tokio::fs::remove_dir_all(harness.blob_root.join("tmp"))
        .await
        .expect("removing the staging directory");

    let (status, body) = get(&harness.state, "/ready", None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let reported: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(reported["database"], true);
    assert_eq!(reported["blobs"], false);

    let (alive, _) = get(&harness.state, "/health", None).await;
    assert_eq!(alive, StatusCode::OK);
});

database_test!(readiness_fails_when_the_database_has_gone, harness, {
    harness.state.db.close().await;

    let (status, body) = get(&harness.state, "/ready", None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let reported: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(reported["database"], false);
    assert_eq!(reported["blobs"], true);
});

database_test!(metrics_do_not_exist_until_a_token_is_configured, harness, {
    let (status, _) = get(&harness.state, "/metrics", Some("anything")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
});

database_test!(metrics_need_the_configured_token, harness, {
    let mut state = harness.state.clone();
    state.metrics_token = Some("scrape-secret".into());

    let (without, _) = get(&state, "/metrics", None).await;
    let (wrong, _) = get(&state, "/metrics", Some("not-it")).await;
    let (status, body) = get(&state, "/metrics", Some("scrape-secret")).await;

    assert_eq!(without, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong, StatusCode::UNAUTHORIZED);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("stashden_http_requests_total{class=\"2xx\"}"));
    assert!(body.contains("stashden_db_connections{state=\"idle\"}"));
});
