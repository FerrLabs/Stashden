use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::error::ApiError;

pub const FRESH_FOR_SECONDS: i64 = 600;

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
    via_provider: bool,
) -> Result<Uuid, ApiError> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, via_provider) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(user_id)
    .bind(expires_at)
    .bind(via_provider)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn is_live(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, ApiError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
             SELECT 1 FROM sessions WHERE id = $1 AND user_id = $2 AND expires_at > now()
         )",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}

pub async fn signed_in_recently_through_provider(
    pool: &PgPool,
    id: Uuid,
) -> Result<bool, ApiError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
             SELECT 1 FROM sessions
             WHERE id = $1 AND via_provider
               AND created_at > now() - ($2::bigint * interval '1 second')
         )",
    )
    .bind(id)
    .bind(FRESH_FOR_SECONDS)
    .fetch_one(pool)
    .await?)
}

pub async fn revoke(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_all(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    keeping: Option<Uuid>,
) -> Result<u64, ApiError> {
    Ok(
        sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND id IS DISTINCT FROM $2")
            .bind(user_id)
            .bind(keeping)
            .execute(executor)
            .await?
            .rows_affected(),
    )
}

pub async fn purge_expired(pool: &PgPool) -> Result<u64, ApiError> {
    Ok(sqlx::query("DELETE FROM sessions WHERE expires_at < now()")
        .execute(pool)
        .await?
        .rows_affected())
}
