use anyhow::{Context, Result};
use std::time::Duration;

use stashden_api::{bootstrap, build_router, config::Config, migrations, state::AppState, sweeper};
use stashden_core::user::Email;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

const RESET_PASSWORD: &str = "reset-password";
const CHECK: &str = "check";
const MIGRATE: &str = "migrate";
const SHUTDOWN_GRACE: Duration = Duration::from_secs(25);

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let mut arguments = std::env::args().skip(1);
    let command = arguments.next();
    if command.as_deref() == Some(MIGRATE) {
        return migrate().await;
    }

    let cfg = Config::from_env().context("loading configuration")?;
    if let Some(command) = command {
        return match command.as_str() {
            RESET_PASSWORD => reset_password(&cfg, arguments.next()).await,
            CHECK => check(&cfg, arguments.collect()).await,
            other => {
                anyhow::bail!(
                    "unknown command {other}; the ones there are: {RESET_PASSWORD}, {CHECK}, {MIGRATE}"
                )
            }
        };
    }

    if stashden_api::config::jwt_secret_is_short(&cfg.jwt_secret) {
        warn!(
            minimum = stashden_api::config::MIN_JWT_SECRET_CHARS,
            "JWT_SECRET is shorter than it should be; generate one with `openssl rand -hex 32`"
        );
    }

    let state = AppState::from_config(&cfg).await?;

    migrations::ensure_current(&state.db).await?;

    bootstrap::admin(&state.db, cfg.bootstrap_admin.as_ref()).await?;

    if cfg.blob_sweep_interval_seconds > 0 {
        sweeper::spawn(
            state.clone(),
            Duration::from_secs(cfg.blob_sweep_interval_seconds),
            Duration::from_secs(cfg.blob_grace_period_seconds),
        );
        info!(
            every = cfg.blob_sweep_interval_seconds,
            grace = cfg.blob_grace_period_seconds,
            "collecting orphaned blobs in the background"
        );
    }

    let app = build_router(state, &cfg.cors_allowed_origins, cfg.web_root.as_deref());
    let bind = format!("0.0.0.0:{}", cfg.port);
    info!(%bind, "starting Stashden API");

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;

    let stop = CancellationToken::new();
    tokio::spawn({
        let stop = stop.clone();
        async move {
            terminated().await;
            info!("shutting down, letting running transfers finish");
            stop.cancel();
        }
    });

    let serving = axum::serve(listener, app).with_graceful_shutdown({
        let stop = stop.clone();
        async move { stop.cancelled().await }
    });
    tokio::select! {
        served = std::future::IntoFuture::into_future(serving) => served.context("serving")?,
        () = async {
            stop.cancelled().await;
            tokio::time::sleep(SHUTDOWN_GRACE).await;
        } => warn!(
            seconds = SHUTDOWN_GRACE.as_secs(),
            "transfers were still running, stopping anyway"
        ),
    }
    Ok(())
}

async fn terminated() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let terminate = async {
            match signal(SignalKind::terminate()) {
                Ok(mut stream) => {
                    stream.recv().await;
                }
                Err(_) => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            () = interrupt => {}
            () = terminate => {}
        }
    }

    #[cfg(not(unix))]
    interrupt.await;
}

async fn migrate() -> Result<()> {
    let url = stashden_api::config::database_url().context("loading configuration")?;
    let pool = stashden_api::db::connect(&url, 1).await?;
    migrations::apply(&pool)
        .await
        .context("running database migrations")?;
    info!("database migrations are up to date");
    Ok(())
}

async fn check(cfg: &Config, flags: Vec<String>) -> Result<()> {
    let mut options = stashden_api::check::Options::default();
    for flag in &flags {
        match flag.as_str() {
            "--verify-content" => options.verify_content = true,
            "--repair" => options.repair = true,
            other => anyhow::bail!(
                "unknown option {other}; the ones there are: --verify-content, --repair"
            ),
        }
    }

    let state = AppState::from_config(cfg).await?;
    let findings = stashden_api::check::run(&state.db, state.blobs.as_ref(), options)
        .await
        .context("checking the instance")?;

    for hash in &findings.missing {
        println!("blob {}: referenced, but not in the store", hash.to_hex());
    }
    for hash in &findings.corrupt {
        println!("blob {}: its bytes no longer match its hash", hash.to_hex());
    }
    for wrong in &findings.wrong_counts {
        println!(
            "blob {}: reference count {}, should be {}",
            wrong.hash.to_hex(),
            wrong.stored,
            wrong.expected
        );
    }
    for wrong in &findings.wrong_usage {
        println!(
            "account {}: usage {} bytes, should be {}",
            wrong.owner_id, wrong.stored, wrong.expected
        );
    }

    let content = if options.verify_content {
        "bytes and hashes"
    } else {
        "presence"
    };
    println!("checked {} blobs for {content}", findings.checked_blobs);
    if findings.repaired {
        println!("reference counts and usage corrected");
    }
    if findings.needs_a_person() {
        anyhow::bail!(
            "{} blobs are missing and {} corrupt: restore them from a backup",
            findings.missing.len(),
            findings.corrupt.len()
        );
    }
    if !findings.is_clean() {
        anyhow::bail!("counts or usage are off: run it again with --repair");
    }
    println!("clean");
    Ok(())
}

async fn reset_password(cfg: &Config, email: Option<String>) -> Result<()> {
    let email: Email = email
        .context("usage: stashden-api reset-password <email>, the new password on standard input")?
        .parse()
        .context("the account's email")?;

    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .context("reading the new password from standard input")?;
    let password = password.trim_end_matches(['\r', '\n']);

    let pool = sqlx::PgPool::connect(&cfg.database_url)
        .await
        .context("connecting to Postgres")?;
    let user = stashden_api::recovery::reset_password(&pool, &email, password)
        .await
        .context("resetting the password")?;

    println!(
        "reset the password of {} and signed it out everywhere",
        user.email
    );
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).json())
        .init();
}
