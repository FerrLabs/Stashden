use sqlx::PgPool;
use stashden_core::user::{Email, User};
use uuid::Uuid;

use crate::app_passwords::{fingerprint, secret};
use crate::attempts::{self, Scope};
use crate::error::ApiError;
use crate::mail::Letter;
use crate::{sessions, users};

pub const VALID_FOR_SECONDS: f64 = 60.0 * 60.0;

pub async fn issue(pool: &PgPool, user_id: Uuid) -> Result<String, ApiError> {
    let secret = secret();
    sqlx::query(
        "INSERT INTO password_resets (hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(secs => $3))
         ON CONFLICT (user_id) DO UPDATE
         SET hash = excluded.hash, expires_at = excluded.expires_at",
    )
    .bind(fingerprint(&secret))
    .bind(user_id)
    .bind(VALID_FOR_SECONDS)
    .execute(pool)
    .await?;
    Ok(secret)
}

pub async fn redeem(pool: &PgPool, secret: &str, plaintext: &str) -> Result<User, ApiError> {
    let hash = fingerprint(secret);
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM password_resets WHERE hash = $1 AND expires_at > now()",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::ResetLinkInvalid)?;

    let mut tx = pool.begin().await?;
    let user = users::by_id_in(&mut tx, user_id).await?;
    let consumed = sqlx::query(
        "DELETE FROM password_resets WHERE hash = $1 AND user_id = $2 AND expires_at > now()",
    )
    .bind(&hash)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    if consumed.rows_affected() == 0 || !user.is_active() || !user.has_password {
        return Err(ApiError::ResetLinkInvalid);
    }

    users::set_password(&mut *tx, user.id, plaintext).await?;
    sessions::revoke_all(&mut *tx, user.id, None).await?;
    tx.commit().await?;

    attempts::forget(pool, Scope::Login, user.email.as_str()).await?;
    Ok(user)
}

pub async fn purge_expired(pool: &PgPool) -> Result<u64, ApiError> {
    let removed = sqlx::query("DELETE FROM password_resets WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(removed.rows_affected())
}

#[must_use]
pub fn letter(public_url: &str, to: Email, secret: &str) -> Letter {
    let body = format!(
        "Someone asked to reset the password of the Stashden account for {to}.\n\n\
         To choose a new one, open this link within the hour:\n\n\
         {public_url}/#/reset-password/{secret}\n\n\
         If it was not you, ignore this message: your password stays as it is.\n"
    );
    Letter {
        to,
        subject: "Reset your Stashden password".to_owned(),
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_letter_carries_the_link_in_the_fragment_so_the_server_never_logs_it() {
        let to: Email = "ada@example.com".parse().expect("valid");
        let letter = letter("https://files.example.com", to.clone(), "abc123");

        assert_eq!(letter.to, to);
        assert!(
            letter
                .body
                .contains("https://files.example.com/#/reset-password/abc123"),
            "{}",
            letter.body
        );
    }
}
