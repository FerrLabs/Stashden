mod common;

use std::borrow::Cow;
use std::process::Stdio;
use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::{AssertSqlSafe, Executor, PgPool, postgres::PgPoolOptions};
use stashden_api::config::BootstrapAdmin;
use stashden_api::migrations::{self, SchemaError};
use stashden_api::state::AppState;
use stashden_api::sweeper::sweep;
use stashden_api::{bootstrap, search};
use stashden_core::blob::BlobHash;
use stashden_core::role::Role;
use tokio::process::Command;
use uuid::Uuid;

use common::Harness;

macro_rules! unmigrated_database_test {
    ($name:ident, $harness:ident, $body:block) => {
        #[tokio::test]
        async fn $name() {
            let Some($harness) = Harness::start_unmigrated().await else {
                eprintln!(
                    "skipping {}: DATABASE_URL is not set",
                    stringify!($name)
                );
                return;
            };
            $body
            $harness.stop().await;
        }
    };
}

const SERVER: &str = env!("CARGO_BIN_EXE_stashden-api");
const FIRST_MIGRATIONS: usize = 3;

async fn tracked(pool: &PgPool) -> bool {
    sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
        .fetch_one(pool)
        .await
        .expect("looking for the migrations table")
}

async fn applied(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
        .expect("counting the applied migrations")
}

async fn apply_first_migrations(pool: &PgPool) {
    let all = sqlx::migrate!("./migrations");
    let first = Migrator {
        migrations: Cow::Owned(all.migrations[..FIRST_MIGRATIONS].to_vec()),
        ..Migrator::DEFAULT
    };
    first
        .run(pool)
        .await
        .expect("applying the first migrations");
}

fn embedded() -> usize {
    sqlx::migrate!("./migrations").migrations.len()
}

fn expect_behind(result: Result<(), SchemaError>) -> Vec<i64> {
    match result {
        Err(SchemaError::Behind { pending }) => pending,
        other => panic!("expected a schema that is behind, got {other:?}"),
    }
}

fn server(harness: &Harness) -> Command {
    let mut command = Command::new(SERVER);
    command
        .env_clear()
        .env("DATABASE_URL", &harness.database_url)
        .env("JWT_SECRET", "x".repeat(40))
        .env("BLOB_ROOT", &harness.blob_root)
        .env("PORT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

async fn run(mut command: Command) -> std::process::Output {
    tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .expect("the process to exit on its own")
        .expect("running the process")
}

unmigrated_database_test!(migrate_applies_everything_to_an_empty_database, harness, {
    assert!(!tracked(&harness.state.db).await);

    migrations::apply(&harness.state.db)
        .await
        .expect("migrating");

    migrations::ensure_current(&harness.state.db)
        .await
        .expect("current");
    assert_eq!(harness.account_count().await, 0);
});

unmigrated_database_test!(migrate_twice_changes_nothing, harness, {
    migrations::apply(&harness.state.db).await.expect("first");
    let before = applied(&harness.state.db).await;

    migrations::apply(&harness.state.db).await.expect("second");

    assert_eq!(applied(&harness.state.db).await, before);
});

unmigrated_database_test!(migrate_finishes_a_partially_migrated_database, harness, {
    apply_first_migrations(&harness.state.db).await;
    let partial = applied(&harness.state.db).await;
    assert_eq!(partial, i64::try_from(FIRST_MIGRATIONS).expect("small"));

    migrations::apply(&harness.state.db)
        .await
        .expect("finishing");

    assert!(applied(&harness.state.db).await > partial);
    migrations::ensure_current(&harness.state.db)
        .await
        .expect("current");
});

unmigrated_database_test!(an_empty_database_is_behind_and_stays_untouched, harness, {
    let pending = expect_behind(migrations::ensure_current(&harness.state.db).await);

    assert_eq!(pending.len(), embedded());
    assert!(
        !tracked(&harness.state.db).await,
        "checking the schema must not create anything"
    );
});

unmigrated_database_test!(
    a_partially_migrated_database_is_behind_by_the_rest,
    harness,
    {
        apply_first_migrations(&harness.state.db).await;
        let before = applied(&harness.state.db).await;

        let pending = expect_behind(migrations::ensure_current(&harness.state.db).await);

        assert_eq!(pending.len(), embedded() - FIRST_MIGRATIONS);
        assert_eq!(applied(&harness.state.db).await, before);
    }
);

database_test!(a_schema_from_a_newer_release_is_refused, harness, {
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99991231000001, 'from the future', true, '\\x00', 0)",
    )
    .execute(&harness.state.db)
    .await
    .expect("recording a migration this release does not carry");

    match migrations::ensure_current(&harness.state.db).await {
        Err(SchemaError::Ahead { unknown }) => assert_eq!(unknown, vec![99_991_231_000_001]),
        other => panic!("expected Ahead, got {other:?}"),
    }
});

database_test!(
    a_migration_changed_after_it_was_applied_is_refused,
    harness,
    {
        sqlx::query("UPDATE _sqlx_migrations SET checksum = '\\x00' WHERE version = (SELECT min(version) FROM _sqlx_migrations)")
        .execute(&harness.state.db)
        .await
        .expect("tampering with a checksum");

        assert!(matches!(
            migrations::ensure_current(&harness.state.db).await,
            Err(SchemaError::Modified { .. })
        ));
    }
);

unmigrated_database_test!(the_migrate_command_needs_only_the_database_url, harness, {
    let mut command = Command::new(SERVER);
    command
        .env_clear()
        .env("DATABASE_URL", &harness.database_url)
        .arg("migrate")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let output = run(command).await;

    assert!(output.status.success(), "{output:?}");
    migrations::ensure_current(&harness.state.db)
        .await
        .expect("current");
});

database_test!(
    the_migrate_command_fails_when_the_database_is_unreachable,
    harness,
    {
        let mut command = Command::new(SERVER);
        command
            .env_clear()
            .env("DATABASE_URL", "postgres://nobody@127.0.0.1:1/none")
            .arg("migrate")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = run(command).await;

        assert!(!output.status.success());
    }
);

unmigrated_database_test!(
    the_server_refuses_to_start_on_an_outdated_schema,
    harness,
    {
        let output = run(server(&harness)).await;

        assert!(!output.status.success(), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("stashden-api migrate"), "{stderr}");
        assert!(
            !tracked(&harness.state.db).await,
            "the server must not migrate on start"
        );
    }
);

async fn free_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("binding a free port")
        .local_addr()
        .expect("the bound address")
        .port()
}

async fn healthy(port: u16) -> bool {
    reqwest::get(format!("http://127.0.0.1:{port}/health"))
        .await
        .is_ok_and(|response| response.status().is_success())
}

database_test!(the_server_starts_on_a_migrated_schema, harness, {
    migrations::apply(&harness.state.db)
        .await
        .expect("migrating");
    let port = free_port().await;
    let mut command = server(&harness);
    command.env("PORT", port.to_string());
    let mut child = command.spawn().expect("starting the server");

    let mut serving = false;
    for _ in 0..100 {
        if healthy(port).await {
            serving = true;
            break;
        }
        assert!(
            child.try_wait().expect("polling the server").is_none(),
            "the server exited instead of serving"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    child.kill().await.expect("stopping the server");

    assert!(serving, "the server never answered /health");
});

async fn restricted_state(harness: &Harness) -> AppState {
    let role = format!("stashden_app_{}", Uuid::now_v7().simple());
    for statement in [
        format!("CREATE ROLE \"{role}\" NOLOGIN"),
        format!("GRANT USAGE ON SCHEMA public TO \"{role}\""),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO \"{role}\""
        ),
        format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO \"{role}\""),
        "REVOKE CREATE ON SCHEMA public FROM PUBLIC".to_owned(),
    ] {
        harness
            .state
            .db
            .execute(AssertSqlSafe(statement))
            .await
            .expect("setting up the restricted role");
    }

    harness.drop_role_on_stop(&role);

    let set_role = format!("SET ROLE \"{role}\"");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _| {
            let set_role = set_role.clone();
            Box::pin(async move {
                connection.execute(AssertSqlSafe(set_role)).await?;
                Ok(())
            })
        })
        .connect(&harness.database_url)
        .await
        .expect("connecting as the restricted role");

    AppState {
        db: pool,
        ..harness.state.clone()
    }
}

database_test!(the_server_runs_without_any_ddl_rights, harness, {
    let member = harness.account("member@example.com", Role::Member).await;
    let contents = b"uploaded, deleted, purged";
    let node = harness.write(member.id, "gone.txt", contents).await;
    harness.write(member.id, "kept.txt", b"still here").await;
    harness.trash(&node).await;
    harness.purge(member.id, node.id).await;
    let restricted = restricted_state(&harness).await;

    let ddl = restricted
        .db
        .execute("CREATE TABLE denied (id INT)")
        .await
        .expect_err("the role must not be able to create tables");
    assert!(ddl.to_string().contains("permission denied"), "{ddl}");

    migrations::ensure_current(&restricted.db)
        .await
        .expect("reading the applied migrations");

    let collected = sweep(&restricted, Duration::ZERO).await.expect("sweeping");
    assert_eq!(collected.blobs, 1);
    assert_eq!(
        harness.blob(BlobHash::from(blake3::hash(contents))).await,
        None
    );

    let hits = search::by_name(&restricted.db, &member, "kept", 10, 0)
        .await
        .expect("searching");
    assert_eq!(hits.len(), 1);
});

database_test!(the_bootstrap_administrator_needs_only_dml, harness, {
    let restricted = restricted_state(&harness).await;
    let admin = BootstrapAdmin {
        email: "admin@example.com".to_owned(),
        password: common::PASSWORD.to_owned(),
    };

    bootstrap::admin(&restricted.db, Some(&admin))
        .await
        .expect("bootstrapping");

    let created = harness.account_by_email("admin@example.com").await;
    assert_eq!(created.role, Role::Admin);
});
