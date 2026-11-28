mod connection;
mod legacy;
mod render;
mod sync;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use connection::{DEFAULT_SERVER, SERVER_HELP, remote};
use stashden_client::Remote;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "stashden", version, about = "Command-line client for Stashden")]
struct Cli {
    #[arg(long, env = "STASHDEN_URL", help = SERVER_HELP)]
    server: Option<String>,

    #[arg(long, env = "STASHDEN_TOKEN", hide_env_values = true)]
    token: Option<String>,

    #[arg(
        long,
        env = "STASHDEN_EMAIL",
        help = "Account an app password belongs to"
    )]
    email: Option<String>,

    #[arg(
        long,
        env = "STASHDEN_APP_PASSWORD",
        hide_env_values = true,
        help = "App password, used instead of a session token when set with --email"
    )]
    app_password: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Exchange an email and password for a session token
    Login {
        #[arg(required_unless_present = "browser")]
        email: Option<String>,
        #[arg(
            long,
            conflicts_with_all = ["email", "password"],
            help = "Sign in through the browser with the server's identity provider"
        )]
        browser: bool,
        #[arg(long, env = "STASHDEN_PASSWORD", hide_env_values = true)]
        password: Option<String>,
        #[arg(
            long,
            value_name = "NAME",
            help = "Mint an app password under this name and print it, for a sync that runs unattended"
        )]
        create_app_password: Option<String>,
    },
    /// List a remote directory
    Ls {
        #[arg(default_value = "/")]
        path: String,
    },
    /// Rename a remote node, or move it under another directory
    Mv { from: String, to: String },
    /// Move a remote file to the trash
    Rm { path: String },
    /// List what is in the trash
    Trash,
    /// Bring something back from the trash
    Restore { id: Uuid },
    /// Delete something from the trash for good
    Purge { id: Uuid },
    /// Reconcile a local folder with the server
    Sync {
        folder: PathBuf,
        /// Keep running, syncing the folder as it changes
        #[arg(long)]
        watch: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let server = legacy::settle(cli.server.clone(), "STASHDEN_URL", "ROXYCLOUD_URL")
        .unwrap_or_else(|| DEFAULT_SERVER.to_owned());
    let connect = || {
        if let (Some(email), Some(secret)) = (cli.email.as_deref(), cli.app_password.as_deref())
            && !secret.is_empty()
        {
            return Remote::with_app_password(&server, email, secret)
                .context("building the API client");
        }
        let token = legacy::settle(cli.token.clone(), "STASHDEN_TOKEN", "ROXYCLOUD_TOKEN");
        remote(&server, token.as_deref())
    };

    match &cli.command {
        Command::Login {
            email,
            browser,
            password,
            create_app_password,
        } => {
            let (remote, session) = if *browser {
                Remote::login_in_browser(&server, |address| {
                    eprintln!("Finish signing in in your browser, or open {address}");
                })
                .await
                .context("signing in through the browser")?
            } else {
                let email = email.as_deref().context("an email is needed")?;
                let password =
                    legacy::settle(password.clone(), "STASHDEN_PASSWORD", "ROXYCLOUD_PASSWORD")
                        .context(
                            "a password is needed: pass --password or set STASHDEN_PASSWORD",
                        )?;
                Remote::login(&server, email, &password)
                    .await
                    .context("logging in")?
            };
            match create_app_password {
                Some(name) => {
                    let minted = remote
                        .mint_app_password(name)
                        .await
                        .context("minting the app password")?;
                    let email = match email {
                        Some(email) => email.clone(),
                        None => remote.me().await.context("reading the account")?.email,
                    };
                    if let Err(error) = remote.logout().await {
                        eprintln!(
                            "the session used to mint it stays open until it expires: {error}"
                        );
                    }
                    println!("STASHDEN_EMAIL={email}");
                    println!("STASHDEN_APP_PASSWORD={}", minted.secret);
                }
                None => println!("{}", session.token),
            }
        }
        Command::Ls { path } => {
            for node in connect()?.list(path).await? {
                println!("{}", render::listing_line(&node));
            }
        }
        Command::Mv { from, to } => {
            connect()?.rename(from, to).await?;
        }
        Command::Rm { path } => {
            connect()?.delete(path).await?;
        }
        Command::Trash => {
            for trashed in connect()?.trash().await? {
                println!("{}", render::trash_line(&trashed));
            }
        }
        Command::Restore { id } => {
            connect()?.restore(*id).await?;
        }
        Command::Purge { id } => {
            connect()?.purge(*id).await?;
        }
        Command::Sync { folder, watch } => {
            sync::run(folder, *watch, connect()?, &server).await?;
        }
    }
    Ok(())
}
