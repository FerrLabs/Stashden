use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use stashden_client::{Account, Methods, Remote, RemoteError, free_path};
use stashden_core::node::Node;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::failure::Failure;
use crate::keychain::{self, Credentials};
use crate::sync::{Folders, Syncing};

pub const IDENTIFIER: &str = "com.ferrlabs.roxycloud";
const LAST_SERVER: &str = "server";
const CHANGES_BUFFER: usize = 64;

pub enum Resumed {
    NoSession,
    Opened { notice: Option<String> },
}

#[derive(Clone)]
struct Connection {
    remote: Arc<Remote>,
    credentials: Credentials,
}

pub struct Desktop {
    connection: Mutex<Option<Connection>>,
    pub(crate) folders: tokio::sync::Mutex<Folders>,
    pub(crate) active: Arc<AtomicBool>,
    pub(crate) changes: broadcast::Sender<Syncing>,
    pub(crate) config: PathBuf,
}

impl Desktop {
    pub fn new(config: PathBuf) -> Arc<Self> {
        let folders = Folders::default();
        Arc::new(Self {
            connection: Mutex::new(None),
            active: folders.active(),
            folders: tokio::sync::Mutex::new(folders),
            changes: broadcast::channel(CHANGES_BUFFER).0,
            config,
        })
    }

    pub fn config_directory() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(IDENTIFIER)
    }

    fn connection(&self) -> MutexGuard<'_, Option<Connection>> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn remote(&self) -> Result<Arc<Remote>, Failure> {
        self.connection()
            .as_ref()
            .map(|connection| Arc::clone(&connection.remote))
            .ok_or_else(|| Failure::from("not connected to a server"))
    }

    pub fn server(&self) -> Option<String> {
        self.connection()
            .as_ref()
            .map(|connection| connection.credentials.server.clone())
    }

    pub(crate) fn account_email(&self) -> Option<String> {
        self.connection()
            .as_ref()
            .map(|connection| connection.credentials.email.clone())
    }

    pub(crate) async fn account(&self) -> Result<Account, Failure> {
        let known = self.connection().as_ref().and_then(|connection| {
            let credentials = &connection.credentials;
            credentials
                .user_id
                .map(|id| Account::new(&credentials.server, id))
        });
        if let Some(account) = known {
            return Ok(account);
        }
        let server = self.server().ok_or("not connected to a server")?;
        let user = self.remote()?.me().await?;
        Ok(Account::new(&server, user.id))
    }

    fn adopt_user(&self, user_id: Uuid) -> Option<Credentials> {
        let mut connection = self.connection();
        let credentials = &mut connection.as_mut()?.credentials;
        if credentials.user_id.is_some() {
            return None;
        }
        credentials.user_id = Some(user_id);
        Some(credentials.clone())
    }

    pub async fn learn_account(self: &Arc<Self>, user_id: Uuid) -> Option<String> {
        let credentials = self.adopt_user(user_id)?;
        let _ = keychain::keep(credentials).await;
        self.resume_saved_syncs().await
    }

    pub(crate) fn fresh_remote(&self) -> Result<Remote, Failure> {
        let credentials = self
            .connection()
            .as_ref()
            .map(|connection| connection.credentials.clone())
            .ok_or("not connected to a server")?;
        Ok(remote_for(&credentials)?)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Syncing> {
        self.changes.subscribe()
    }

    pub fn last_server(&self) -> Option<String> {
        fs::read_to_string(self.config.join(LAST_SERVER))
            .ok()
            .map(|server| server.trim().to_owned())
            .filter(|server| !server.is_empty())
    }

    fn keep_server(&self, server: &str) {
        let _ = fs::create_dir_all(&self.config)
            .and_then(|()| fs::write(self.config.join(LAST_SERVER), server));
    }

    pub async fn methods(server: String) -> Result<Methods, Failure> {
        Ok(Remote::methods(&server).await?)
    }

    pub fn forgot_password(server: &str) -> Result<(), Failure> {
        Ok(Remote::reset_password_in_browser(server)?)
    }

    pub async fn sign_in(
        self: Arc<Self>,
        server: String,
        email: String,
        password: String,
    ) -> Result<Option<String>, Failure> {
        let (session, _) = Remote::login(&server, &email, &password).await?;
        self.settle(server, session).await
    }

    pub async fn sign_in_with_provider(
        self: Arc<Self>,
        server: String,
    ) -> Result<Option<String>, Failure> {
        let (session, _) = Remote::login_in_browser(&server, |_| {}).await?;
        self.settle(server, session).await
    }

    async fn settle(
        self: Arc<Self>,
        server: String,
        session: Remote,
    ) -> Result<Option<String>, Failure> {
        let user = session.me().await?;
        let minted = session
            .mint_app_password(&format!("Stashden desktop on {}", this_computer()))
            .await?;
        let _ = session.logout().await;

        let credentials = Credentials {
            server,
            email: user.email,
            secret: minted.secret,
            user_id: Some(user.id),
        };
        self.keep_server(&credentials.server);
        let refused = self.connect(credentials.clone()).await?;
        let unsaved = keychain::keep(credentials).await.err().map(|reason| {
            format!("Signed in until the app closes, then it will ask again: {reason}")
        });
        Ok([refused, unsaved]
            .into_iter()
            .flatten()
            .reduce(|first, second| format!("{first}\n{second}")))
    }

    pub async fn resume(self: Arc<Self>) -> Result<Resumed, Failure> {
        let Some(credentials) = keychain::kept().await? else {
            return Ok(Resumed::NoSession);
        };
        let notice = self.connect(credentials).await?;
        Ok(Resumed::Opened { notice })
    }

    async fn connect(
        self: &Arc<Self>,
        mut credentials: Credentials,
    ) -> Result<Option<String>, Failure> {
        let remote = Arc::new(remote_for(&credentials)?);
        if credentials.user_id.is_none()
            && let Ok(user) = remote.me().await
        {
            credentials.user_id = Some(user.id);
            let _ = keychain::keep(credentials.clone()).await;
        }
        *self.connection() = Some(Connection {
            remote,
            credentials,
        });
        Ok(self.resume_saved_syncs().await)
    }

    pub async fn sign_out(&self) -> Result<(), Failure> {
        self.stop_all_syncs().await;
        let connection = self.connection().take();
        let revoked = match connection {
            Some(connection) => match connection.remote.revoke_own_app_password().await {
                Ok(()) | Err(RemoteError::Unauthenticated) => Ok(()),
                Err(error) => Err(Failure::from(format!(
                    "the server did not revoke this computer's app password, revoke it from the web app: {error}"
                ))),
            },
            None => Ok(()),
        };
        keychain::forget().await?;
        revoked
    }

    pub async fn upload(&self, path: String, source: PathBuf) -> Result<Node, Failure> {
        if source.is_dir() {
            return Err(Failure::from(format!(
                "{} is a folder; upload the files inside it, or sync the folder",
                source.display()
            )));
        }
        let remote = self.remote()?;
        remote
            .upload(&path, &source)
            .await
            .map_err(|error| Failure::from(error).about(&path))
    }
}

pub fn into_downloads(name: &str) -> Result<PathBuf, Failure> {
    let directory = dirs::download_dir().ok_or("this computer has no downloads folder")?;
    Ok(destination_in(&directory, name))
}

fn destination_in(directory: &Path, path: &str) -> PathBuf {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    free_path(directory, name)
}

fn remote_for(credentials: &Credentials) -> Result<Remote, RemoteError> {
    Remote::with_app_password(
        &credentials.server,
        credentials.email.as_str(),
        credentials.secret.as_str(),
    )
}

fn this_computer() -> String {
    let name = gethostname::gethostname().to_string_lossy().into_owned();
    if name.is_empty() {
        "this computer".to_owned()
    } else {
        name
    }
}

pub fn file_name(source: &Path) -> String {
    source.file_name().map_or_else(
        || source.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use stashden_client::sync::watch::Command;

    use super::*;
    use crate::runtime;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("stashden-desktop-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn nothing_reaches_the_server_before_signing_in() {
        let desktop = Desktop::new(scratch("unsigned"));
        assert_eq!(
            desktop.remote().err().map(|failure| failure.to_string()),
            Some("not connected to a server".to_owned())
        );
        assert_eq!(desktop.server(), None);
        assert!(!desktop.is_syncing());

        let started = runtime::block_on(desktop.start_sync(PathBuf::from("/tmp")));
        assert_eq!(
            started.map_err(|failure| failure.to_string()),
            Err("not connected to a server".to_owned())
        );
    }

    fn offline_desktop(name: &str, folder: &Path) -> Arc<Desktop> {
        let desktop = Desktop::new(scratch(name));
        fs::create_dir_all(folder).unwrap();
        let credentials = Credentials {
            server: "http://127.0.0.1:1".to_owned(),
            email: "a@example.com".to_owned(),
            secret: "secret".to_owned(),
            user_id: None,
        };
        *desktop.connection() = Some(Connection {
            remote: Arc::new(remote_for(&credentials).unwrap()),
            credentials,
        });
        let mut remembered = crate::remembered::Remembered::default();
        remembered.add("a@example.com", folder.to_path_buf());
        remembered.save(&desktop.config).unwrap();
        desktop
    }

    #[test]
    fn saved_folders_wait_for_a_verified_account_and_start_once_it_is_known() {
        let folder = scratch("pending-folder");
        let desktop = offline_desktop("pending", &folder);

        let notice = runtime::block_on(desktop.resume_saved_syncs());
        assert!(notice.is_some());
        assert_eq!(
            runtime::block_on(desktop.syncing()),
            Vec::<crate::sync::Syncing>::new()
        );

        assert!(desktop.adopt_user(Uuid::from_u128(7)).is_some());
        assert!(desktop.adopt_user(Uuid::from_u128(8)).is_none());
        assert_eq!(
            runtime::block_on(desktop.account()).unwrap(),
            Account::new("http://127.0.0.1:1", Uuid::from_u128(7))
        );
        assert_eq!(runtime::block_on(desktop.resume_saved_syncs()), None);
        let syncing = runtime::block_on(desktop.syncing());
        assert_eq!(syncing.len(), 1);
        assert_eq!(syncing[0].folder, folder);

        runtime::block_on(desktop.stop_all_syncs());
        let _ = fs::remove_dir_all(&folder);
        let _ = fs::remove_dir_all(&desktop.config);
    }

    #[test]
    fn controlling_a_folder_that_is_not_syncing_is_refused() {
        let desktop = Desktop::new(scratch("control"));
        for command in [Command::SyncNow, Command::Pause, Command::Stop] {
            let outcome = runtime::block_on(desktop.control_sync(PathBuf::from("/nope"), command));
            assert_eq!(
                outcome.map_err(|failure| failure.to_string()),
                Err("that folder is not syncing".to_owned())
            );
        }
        assert_eq!(
            runtime::block_on(desktop.syncing()),
            Vec::<crate::sync::Syncing>::new()
        );
    }

    #[test]
    fn a_folder_is_refused_as_an_upload_before_anything_is_sent() {
        let directory = scratch("upload");
        fs::create_dir_all(&directory).unwrap();
        let desktop = Desktop::new(directory.clone());

        let outcome = runtime::block_on(desktop.upload("x".to_owned(), directory.clone()));
        assert!(outcome.unwrap_err().to_string().contains("is a folder"));
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_last_server_is_remembered_across_launches() {
        let directory = scratch("server");
        let desktop = Desktop::new(directory.clone());
        assert_eq!(desktop.last_server(), None);

        desktop.keep_server("https://files.example");
        assert_eq!(
            Desktop::new(directory.clone()).last_server(),
            Some("https://files.example".to_owned())
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_download_takes_the_last_path_segment_and_never_overwrites() {
        let directory = scratch("downloads");
        fs::create_dir_all(&directory).unwrap();
        assert_eq!(
            destination_in(&directory, "docs/2024/report.pdf"),
            directory.join("report.pdf")
        );
        fs::write(directory.join("report.pdf"), b"taken").unwrap();
        assert_ne!(
            destination_in(&directory, "report.pdf"),
            directory.join("report.pdf")
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_picked_file_is_named_after_its_last_component() {
        assert_eq!(file_name(Path::new("/home/a/notes.md")), "notes.md");
        assert_eq!(file_name(Path::new("/")), "/");
    }
}
