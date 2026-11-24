use sqlx::PgPool;
use sqlx::migrate::{AppliedMigration, Migrate, MigrateError, Migrator};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error(
        "the database schema is behind this release ({} pending: {}); run `stashden-api migrate` with a role that owns the schema before starting the server",
        .pending.len(),
        join(.pending)
    )]
    Behind { pending: Vec<i64> },
    #[error(
        "the database was migrated by a newer release (unknown versions: {}); downgrading is not supported, restore the backup taken before the upgrade or run the newer release",
        join(.unknown)
    )]
    Ahead { unknown: Vec<i64> },
    #[error(
        "migrations already applied were changed after the fact (versions: {}); this release does not match the database",
        join(.versions)
    )]
    Modified { versions: Vec<i64> },
    #[error(
        "a migration failed halfway (version {0}); restore the backup and run `stashden-api migrate` again"
    )]
    Dirty(i64),
    #[error("reading the applied migrations: {0}")]
    Migrate(#[from] MigrateError),
}

pub async fn apply(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

pub async fn ensure_current(pool: &PgPool) -> Result<(), SchemaError> {
    let tracked: bool = sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(MigrateError::from)?;
    if !tracked {
        return compare(&[]);
    }

    let mut connection = pool.acquire().await.map_err(MigrateError::from)?;
    if let Some(version) = connection.dirty_version(&MIGRATOR.table_name).await? {
        return Err(SchemaError::Dirty(version));
    }
    let applied = connection
        .list_applied_migrations(&MIGRATOR.table_name)
        .await?;
    compare(&applied)
}

fn compare(applied: &[AppliedMigration]) -> Result<(), SchemaError> {
    let embedded: Vec<_> = MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .collect();

    let unknown: Vec<i64> = applied
        .iter()
        .map(|done| done.version)
        .filter(|version| !embedded.iter().any(|known| known.version == *version))
        .collect();
    if !unknown.is_empty() {
        return Err(SchemaError::Ahead { unknown });
    }

    let versions: Vec<i64> = embedded
        .iter()
        .filter(|known| {
            applied
                .iter()
                .any(|done| done.version == known.version && done.checksum != known.checksum)
        })
        .map(|known| known.version)
        .collect();
    if !versions.is_empty() {
        return Err(SchemaError::Modified { versions });
    }

    let pending: Vec<i64> = embedded
        .iter()
        .map(|known| known.version)
        .filter(|version| !applied.iter().any(|done| done.version == *version))
        .collect();
    if pending.is_empty() {
        Ok(())
    } else {
        Err(SchemaError::Behind { pending })
    }
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

    fn embedded() -> Vec<AppliedMigration> {
        MIGRATOR
            .iter()
            .map(|migration| AppliedMigration {
                version: migration.version,
                checksum: migration.checksum.clone(),
            })
            .collect()
    }

    fn pending_of(result: Result<(), SchemaError>) -> Vec<i64> {
        match result {
            Err(SchemaError::Behind { pending }) => pending,
            other => panic!("expected Behind, got {other:?}"),
        }
    }

    #[test]
    fn nothing_applied_leaves_every_migration_pending() {
        let all: Vec<i64> = embedded().iter().map(|done| done.version).collect();
        assert_eq!(pending_of(compare(&[])), all);
    }

    #[test]
    fn everything_applied_is_current() {
        assert!(compare(&embedded()).is_ok());
    }

    #[test]
    fn only_the_migrations_not_applied_are_pending() {
        let all = embedded();
        let (applied, behind) = all.split_at(all.len() - 1);
        assert_eq!(pending_of(compare(applied)), vec![behind[0].version]);
    }

    #[test]
    fn a_version_this_release_does_not_know_is_ahead() {
        let mut applied = embedded();
        applied.push(AppliedMigration {
            version: i64::MAX,
            checksum: Vec::new().into(),
        });
        match compare(&applied) {
            Err(SchemaError::Ahead { unknown }) => assert_eq!(unknown, vec![i64::MAX]),
            other => panic!("expected Ahead, got {other:?}"),
        }
    }

    #[test]
    fn a_changed_checksum_is_modified() {
        let mut applied = embedded();
        applied[0].checksum = vec![0; 4].into();
        match compare(&applied) {
            Err(SchemaError::Modified { versions }) => {
                assert_eq!(versions, vec![applied[0].version]);
            }
            other => panic!("expected Modified, got {other:?}"),
        }
    }

    #[test]
    fn the_behind_error_names_the_versions_and_the_command() {
        let message = SchemaError::Behind {
            pending: vec![1, 2],
        }
        .to_string();
        assert!(message.contains("1, 2"));
        assert!(message.contains("stashden-api migrate"));
    }

    #[test]
    fn the_ahead_error_says_downgrading_is_not_supported() {
        let message = SchemaError::Ahead { unknown: vec![7] }.to_string();
        assert!(message.contains('7'));
        assert!(message.contains("downgrading is not supported"));
    }
}
