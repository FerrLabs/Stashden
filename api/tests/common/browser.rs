use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use stashden_api::build_router;
use stashden_api::state::AppState;
use tower::ServiceExt;
use uuid::Uuid;

pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub const PORT: u16 = 41234;

pub enum Auth<'a> {
    Nobody,
    Session(&'a str),
    AppPassword(&'a str, &'a str),
}

pub struct Reply {
    pub status: StatusCode,
    pub body: Value,
}

pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub async fn send(
    state: &AppState,
    method: &str,
    uri: &str,
    auth: &Auth<'_>,
    body: Option<Value>,
) -> Reply {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    match auth {
        Auth::Nobody => {}
        Auth::Session(token) => {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        Auth::AppPassword(email, secret) => {
            let encoded = STANDARD.encode(format!("{email}:{secret}"));
            request = request.header(header::AUTHORIZATION, format!("Basic {encoded}"));
        }
    }
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
    let response = build_router(state.clone(), &[], None)
        .oneshot(request.body(body).expect("a well formed request"))
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("reading the body")
        .to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

pub async fn start_with(state: &AppState, body: Value) -> Reply {
    send(state, "POST", "/v1/auth/browser", &Auth::Nobody, Some(body)).await
}

pub async fn start(state: &AppState) -> String {
    let reply = start_with(
        state,
        json!({
            "device_name": "Ada's laptop",
            "challenge": challenge_for(VERIFIER),
            "loopback_port": PORT,
        }),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_str().expect("an id").to_owned()
}

pub async fn approve(state: &AppState, id: &str, session: &str) -> Reply {
    send(
        state,
        "POST",
        &format!("/v1/auth/browser/{id}/approve"),
        &Auth::Session(session),
        None,
    )
    .await
}

pub async fn approved_code(state: &AppState, id: &str, session: &str) -> String {
    let reply = approve(state, id, session).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    code_in(reply.body["callback"].as_str().expect("a callback"))
}

pub fn code_in(callback: &str) -> String {
    callback
        .split_once("code=")
        .expect("a code in the callback")
        .1
        .to_owned()
}

pub async fn redeem(state: &AppState, id: &str, code: &str, verifier: &str) -> Reply {
    send(
        state,
        "POST",
        &format!("/v1/auth/browser/{id}/redeem"),
        &Auth::Nobody,
        Some(json!({ "code": code, "verifier": verifier })),
    )
    .await
}

pub async fn read(state: &AppState, id: &str, session: &str) -> Reply {
    send(
        state,
        "GET",
        &format!("/v1/auth/browser/{id}"),
        &Auth::Session(session),
        None,
    )
    .await
}

pub async fn deny(state: &AppState, id: &str, session: &str) -> Reply {
    send(
        state,
        "DELETE",
        &format!("/v1/auth/browser/{id}"),
        &Auth::Session(session),
        None,
    )
    .await
}

pub fn unknown_id() -> String {
    Uuid::now_v7().to_string()
}
