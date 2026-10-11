use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::auth::Caller;
use crate::error::ApiError;
use crate::fingerprint;
use crate::state::AppState;

pub async fn fingerprint(
    State(state): State<AppState>,
    caller: Caller,
) -> Result<Json<Value>, ApiError> {
    let mut tx = state.db.begin().await?;
    let value = fingerprint::of(&mut tx, caller.user_id(), &caller.user.email).await?;
    tx.commit().await?;
    Ok(Json(json!({ "fingerprint": value })))
}
