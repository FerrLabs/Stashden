use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionCaller;
use crate::browser_sign_ins::{self, NewRequest, Redeemed, Waiting};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Start {
    device_name: String,
    challenge: String,
    loopback_port: u32,
}

#[derive(Serialize)]
pub struct Started {
    id: Uuid,
    confirmation: String,
    expires_in: i32,
}

#[derive(Serialize)]
pub struct Callback {
    callback: String,
}

#[derive(Deserialize)]
pub struct Redemption {
    code: String,
    verifier: String,
}

pub async fn start(
    State(state): State<AppState>,
    Json(request): Json<Start>,
) -> Result<(StatusCode, Json<Started>), ApiError> {
    let started = browser_sign_ins::start(
        &state.db,
        &NewRequest {
            device_name: &request.device_name,
            challenge: &request.challenge,
            loopback_port: request.loopback_port,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Started {
            id: started.id,
            confirmation: started.confirmation,
            expires_in: browser_sign_ins::LIFETIME_SECONDS,
        }),
    ))
}

pub async fn read(
    State(state): State<AppState>,
    caller: SessionCaller,
    Path(id): Path<Uuid>,
) -> Result<Json<Waiting>, ApiError> {
    Ok(Json(
        browser_sign_ins::waiting(&state.db, id, caller.user_id()).await?,
    ))
}

pub async fn approve(
    State(state): State<AppState>,
    caller: SessionCaller,
    Path(id): Path<Uuid>,
) -> Result<Json<Callback>, ApiError> {
    let callback =
        browser_sign_ins::approve(&state.db, id, caller.user_id(), caller.session_id).await?;
    Ok(Json(Callback { callback }))
}

pub async fn deny(
    State(state): State<AppState>,
    caller: SessionCaller,
    Path(id): Path<Uuid>,
) -> Result<Json<Callback>, ApiError> {
    let callback = browser_sign_ins::deny(&state.db, id, caller.user_id()).await?;
    Ok(Json(Callback { callback }))
}

pub async fn redeem(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<Redemption>,
) -> Result<Json<Redeemed>, ApiError> {
    Ok(Json(
        browser_sign_ins::redeem(&state.db, id, &request.code, &request.verifier).await?,
    ))
}
