use anyhow::{Context, Result};
use sqlx::PgPool;
use stashden_core::role::Role;
use stashden_core::user::Email;
use tracing::{info, warn};

use crate::config::BootstrapAdmin;
use crate::users;

pub async fn admin(db: &PgPool, admin: Option<&BootstrapAdmin>) -> Result<()> {
    let Some(admin) = admin else {
        if users::count(db).await? == 0 {
            warn!(
                "no accounts exist; set BOOTSTRAP_ADMIN_EMAIL and BOOTSTRAP_ADMIN_PASSWORD to create the first one"
            );
        }
        return Ok(());
    };

    if users::count(db).await? > 0 {
        info!("accounts already exist, skipping bootstrap");
        return Ok(());
    }

    let email: Email = admin.email.parse().context("BOOTSTRAP_ADMIN_EMAIL")?;
    let mut tx = db.begin().await?;
    let user = users::create(
        &mut tx,
        &email,
        "Administrator",
        &admin.password,
        Role::Admin,
    )
    .await
    .context("creating the bootstrap administrator")?;
    tx.commit().await?;

    info!(%email, id = %user.id, "bootstrap administrator created");
    Ok(())
}
