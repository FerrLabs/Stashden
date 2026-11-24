use sqlx::PgPool;
use sqlx::migrate::{MigrateError, Migrator};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error(
        "the database schema is behind this release ({} pending: {}); run `stashden-api migrate` with a role that owns the schema before starting the server",
        .pending.len(),
        join(.pending)
    )]
    Behind { pending: Vec<i64> },
    #[error("reading the applied migrations: {0}")]
    Database(#[from] sqlx::Error),
}

pub async fn apply(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

pub async fn ensure_current(pool: &PgPool) -> Result<(), SchemaError> {
    let pending = pending(&applied(pool).await?);
    if pending.is_empty() {
        Ok(())
    } else {
        Err(SchemaError::Behind { pending })
    }
}

async fn applied(pool: &PgPool) -> Result<Vec<i64>, sqlx::Error> {
    let tracked: bool = sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
        .fetch_one(pool)
        .await?;
    if !tracked {
        return Ok(Vec::new());
    }
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success")
        .fetch_all(pool)
        .await
}

fn pending(applied: &[i64]) -> Vec<i64> {
    MIGRATOR
        .iter()
        .map(|migration| migration.version)
        .filter(|version| !applied.contains(version))
        .collect()
}

fn join(versions: &[i64]) -> String {
    versions
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedded() -> Vec<i64> {
        MIGRATOR.iter().map(|migration| migration.version).collect()
    }

    #[test]
    fn nothing_applied_leaves_every_migration_pending() {
        assert_eq!(pending(&[]), embedded());
    }

    #[test]
    fn everything_applied_leaves_nothing_pending() {
        assert_eq!(pending(&embedded()), Vec::<i64>::new());
    }

    #[test]
    fn only_the_migrations_not_applied_are_pending() {
        let all = embedded();
        let (applied, behind) = all.split_at(all.len() - 1);
        assert_eq!(pending(applied), behind);
    }

    #[test]
    fn a_version_this_release_does_not_know_does_not_count_as_pending() {
        let mut applied = embedded();
        applied.push(i64::MAX);
        assert_eq!(pending(&applied), Vec::<i64>::new());
    }

    #[test]
    fn the_error_names_the_pending_versions_and_the_command() {
        let message = SchemaError::Behind {
            pending: vec![1, 2],
        }
        .to_string();
        assert!(message.contains("1, 2"));
        assert!(message.contains("stashden-api migrate"));
    }
}
