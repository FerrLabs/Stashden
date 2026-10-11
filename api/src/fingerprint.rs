use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::ApiError;
use crate::grants;

type Totals = (i64, Option<DateTime<Utc>>, Option<DateTime<Utc>>, i64, i64);

pub async fn of(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    email: &str,
) -> Result<String, ApiError> {
    let mut hasher = blake3::Hasher::new();

    let own = sqlx::query_as::<_, Totals>(
        "SELECT count(*), max(updated_at), max(deleted_at),
                count(*) FILTER (WHERE deleted_at IS NOT NULL),
                COALESCE(sum(size), 0)::bigint
         FROM nodes WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&mut **tx)
    .await?;
    feed(&mut hasher, &own);

    for (mount, node) in grants::mounts(tx, email).await? {
        hasher.update(mount.id.as_bytes());
        hasher.update(mount.mount_name.as_bytes());
        hasher.update(&[u8::from(mount.access.may_write())]);

        let below = sqlx::query_as::<_, Totals>(
            "WITH RECURSIVE sub AS (
                 SELECT id, updated_at, deleted_at, size FROM nodes WHERE id = $1
                 UNION ALL
                 SELECT child.id, child.updated_at, child.deleted_at, child.size
                 FROM nodes child JOIN sub ON child.parent_id = sub.id
             )
             SELECT count(*), max(updated_at), max(deleted_at),
                    count(*) FILTER (WHERE deleted_at IS NOT NULL),
                    COALESCE(sum(size), 0)::bigint
             FROM sub",
        )
        .bind(node.id)
        .fetch_one(&mut **tx)
        .await?;
        feed(&mut hasher, &below);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn feed(hasher: &mut blake3::Hasher, totals: &Totals) {
    let (count, updated, deleted, trashed, size) = totals;
    hasher.update(&count.to_le_bytes());
    hasher.update(
        &updated
            .map_or(i64::MIN, |at| at.timestamp_micros())
            .to_le_bytes(),
    );
    hasher.update(
        &deleted
            .map_or(i64::MIN, |at| at.timestamp_micros())
            .to_le_bytes(),
    );
    hasher.update(&trashed.to_le_bytes());
    hasher.update(&size.to_le_bytes());
}
