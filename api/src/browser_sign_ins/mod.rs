mod proof;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use stashden_core::user::User;
use uuid::Uuid;

use crate::app_passwords::{self, Minted, fingerprint, secret};
use crate::error::ApiError;
use crate::metrics::same_token;
use crate::{sessions, users};

pub const LIFETIME_SECONDS: i32 = 15 * 60;
pub const MOST_LIVE: i64 = 1000;
pub const MOST_FAILURES: i32 = 5;
const FIRST_UNPRIVILEGED_PORT: u32 = 1024;
const LAST_PORT: u32 = 65535;

pub struct NewRequest<'a> {
    pub device_name: &'a str,
    pub challenge: &'a str,
    pub loopback_port: u32,
}

pub struct Started {
    pub id: Uuid,
    pub confirmation: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Waiting {
    pub device_name: String,
    pub confirmation: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Redeemed {
    #[serde(flatten)]
    pub minted: Minted,
    pub user: User,
}

#[derive(sqlx::FromRow)]
struct Pending {
    challenge: String,
    device_name: String,
    code_hash: Option<String>,
    failures: i32,
}

impl Pending {
    fn is_answered_by(&self, code: &str, verifier: &str) -> bool {
        let verifier_matches = proof::answers(&self.challenge, verifier);
        let code_matches = self
            .code_hash
            .as_deref()
            .is_some_and(|expected| same_token(&fingerprint(code), expected));
        verifier_matches & code_matches
    }
}

fn is_hidden(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'
        )
}

fn callback(port: i32, id: Uuid, outcome: &str) -> String {
    format!("http://127.0.0.1:{port}/callback?id={id}&{outcome}")
}

pub async fn start(pool: &PgPool, request: &NewRequest<'_>) -> Result<Started, ApiError> {
    let device_name = app_passwords::clean_name(request.device_name)?;
    if device_name.chars().any(is_hidden) {
        return Err(ApiError::WrongKind {
            expected: "device name without control or invisible formatting characters",
        });
    }
    proof::valid_challenge(request.challenge)?;
    if !(FIRST_UNPRIVILEGED_PORT..=LAST_PORT).contains(&request.loopback_port) {
        return Err(ApiError::WrongKind {
            expected: "loopback port between 1024 and 65535",
        });
    }

    let started = Started {
        id: Uuid::now_v7(),
        confirmation: proof::confirmation(),
    };
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('browser_sign_ins', 0))")
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO browser_sign_ins
             (id, challenge, device_name, loopback_port, confirmation, expires_at)
         SELECT $1::uuid, $2::text, $3::text, $4::integer, $5::text,
                now() + make_interval(secs => $6::float8)
         WHERE (SELECT count(*) FROM browser_sign_ins WHERE expires_at > now()) < $7::bigint",
    )
    .bind(started.id)
    .bind(request.challenge)
    .bind(device_name)
    .bind(i32::try_from(request.loopback_port).map_err(|_| ApiError::Credential)?)
    .bind(&started.confirmation)
    .bind(f64::from(LIFETIME_SECONDS))
    .bind(MOST_LIVE)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    if inserted.rows_affected() == 0 {
        return Err(ApiError::Busy);
    }
    tracing::info!(id = %started.id, "browser sign-in started");
    Ok(started)
}

pub async fn waiting(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Waiting, ApiError> {
    sqlx::query_as::<_, Waiting>(
        "SELECT device_name, confirmation, created_at, expires_at
         FROM browser_sign_ins
         WHERE id = $1 AND expires_at > now() AND (user_id IS NULL OR user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}

pub async fn approve(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<String, ApiError> {
    let code = secret();
    let mut tx = pool.begin().await?;
    let account = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id = $1 FOR SHARE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
    if account.is_none() || !sessions::is_live(&mut *tx, session_id, user_id).await? {
        return Err(ApiError::Unauthenticated);
    }
    let port = sqlx::query_scalar::<_, i32>(
        "UPDATE browser_sign_ins
         SET user_id = $2, code_hash = $3, approved_at = now()
         WHERE id = $1 AND expires_at > now() AND user_id IS NULL
         RETURNING loopback_port",
    )
    .bind(id)
    .bind(user_id)
    .bind(fingerprint(&code))
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let Some(port) = port else {
        return Err(refused_approval(pool, id, user_id).await?);
    };
    tracing::info!(%id, %user_id, "browser sign-in approved");
    Ok(callback(port, id, &format!("code={code}")))
}

async fn refused_approval(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<ApiError, ApiError> {
    let approved_by_caller = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
             SELECT 1 FROM browser_sign_ins
             WHERE id = $1 AND expires_at > now() AND user_id = $2
         )",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(if approved_by_caller {
        ApiError::Conflict("this sign-in request is already approved".to_owned())
    } else {
        ApiError::NotFound
    })
}

pub async fn deny(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<String, ApiError> {
    let port = sqlx::query_scalar::<_, i32>(
        "DELETE FROM browser_sign_ins
         WHERE id = $1 AND expires_at > now() AND (user_id IS NULL OR user_id = $2)
         RETURNING loopback_port",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    tracing::info!(%id, %user_id, "browser sign-in denied");
    Ok(callback(port, id, "error=denied"))
}

pub async fn redeem(
    pool: &PgPool,
    id: Uuid,
    code: &str,
    verifier: &str,
) -> Result<Redeemed, ApiError> {
    let mut tx = pool.begin().await?;
    let owner = lock_owner(&mut tx, id).await?;
    let pending = sqlx::query_as::<_, Pending>(
        "SELECT challenge, device_name, code_hash, failures
         FROM browser_sign_ins
         WHERE id = $1 AND expires_at > now()
         FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::Unauthenticated)?;

    let account = owner.filter(User::is_active);
    let Some(user) = account.filter(|_| pending.is_answered_by(code, verifier)) else {
        record_failure(&mut tx, id, pending.failures).await?;
        tx.commit().await?;
        return Err(ApiError::Unauthenticated);
    };

    let minted = app_passwords::mint(&mut tx, user.id, &pending.device_name).await?;
    sqlx::query("DELETE FROM browser_sign_ins WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    tracing::info!(%id, user_id = %user.id, app_password_id = %minted.password.id, "browser sign-in redeemed");
    Ok(Redeemed { minted, user })
}

async fn lock_owner(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<User>, ApiError> {
    let owner =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT user_id FROM browser_sign_ins WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .flatten();
    let Some(owner) = owner else {
        return Ok(None);
    };
    match users::by_id_in(tx, owner).await {
        Ok(user) => Ok(Some(user)),
        Err(ApiError::NotFound) => Ok(None),
        Err(other) => Err(other),
    }
}

async fn record_failure(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    before: i32,
) -> Result<(), ApiError> {
    if before + 1 >= MOST_FAILURES {
        sqlx::query("DELETE FROM browser_sign_ins WHERE id = $1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("UPDATE browser_sign_ins SET failures = failures + 1 WHERE id = $1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    tracing::warn!(%id, "browser sign-in redeem refused");
    Ok(())
}

pub async fn cancel_all(connection: &mut PgConnection, user_id: Uuid) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM browser_sign_ins WHERE user_id = $1")
        .bind(user_id)
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn purge_expired(pool: &PgPool) -> Result<u64, ApiError> {
    Ok(
        sqlx::query("DELETE FROM browser_sign_ins WHERE expires_at <= now()")
            .execute(pool)
            .await?
            .rows_affected(),
    )
}

#[cfg(test)]
mod tests {
    use super::is_hidden;

    #[test]
    fn bidi_overrides_and_zero_width_characters_are_hidden() {
        for hidden in [
            '\u{200B}', '\u{200C}', '\u{200D}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202D}',
            '\u{202E}', '\u{2066}', '\u{2069}', '\u{FEFF}', '\n', '\u{7}',
        ] {
            assert!(is_hidden(hidden), "{hidden:?}");
        }
    }

    #[test]
    fn ordinary_names_in_any_script_are_not() {
        assert!(
            "Ada's laptop, \u{4e2d}\u{6587} \u{1F4BB} caf\u{e9}"
                .chars()
                .all(|c| !is_hidden(c))
        );
        assert!(!is_hidden('\u{200A}'));
        assert!(!is_hidden('\u{2070}'));
    }
}
