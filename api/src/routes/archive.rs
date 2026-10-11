use axum::extract::{Path, State};
use axum::response::Response;

use crate::access::{self, Place};
use crate::archive;
use crate::auth::Caller;
use crate::error::ApiError;
use crate::state::AppState;
use stashden_core::name::parse_path;

pub async fn folder(
    State(state): State<AppState>,
    caller: Caller,
    Path(path): Path<String>,
) -> Result<Response, ApiError> {
    let segments = parse_path(&path)?;
    let mut tx = state.db.begin().await?;
    let place = access::locate(&mut tx, &caller.user, &segments, state.default_quota_bytes).await?;
    tx.commit().await?;

    let root = match place {
        Place::SharedWithMe => {
            return Err(ApiError::WrongKind { expected: "folder" });
        }
        place => place.into_node()?,
    };
    let members = archive::members(&state.db, &root).await?;
    Ok(archive::respond(&state, &root, members))
}
