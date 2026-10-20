use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use stashden_client::sync::watch::{Command, DEFAULT_POLL, Session, Status, watch};
use stashden_client::{Debounce, Engine, Report};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

use crate::Desktop;
use crate::remembered::Remembered;

const STATUS_EVENT: &str = "sync:status";

#[derive(Clone, Serialize)]
pub struct Syncing {
    folder: PathBuf,
    status: Status,
    last: Option<Report>,
}

struct Running {
    session: Session,
    generation: u64,
    latest: Syncing,
}

#[derive(Default)]
pub struct Folders {
    generation: u64,
    running: HashMap<PathBuf, Running>,
}

impl Folders {
    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    fn insert(&mut self, folder: PathBuf, session: Session) -> (u64, Option<Session>) {
        self.generation += 1;
        let generation = self.generation;
        let latest = Syncing {
            folder: folder.clone(),
            status: Status::Idle,
            last: None,
        };
        let replaced = self.running.insert(
            folder,
            Running {
                session,
                generation,
                latest,
            },
        );
        (generation, replaced.map(|replaced| replaced.session))
    }

    fn record(&mut self, folder: &Path, generation: u64, update: Status) -> Option<Syncing> {
        let running = self.running.get_mut(folder)?;
        if running.generation != generation {
            return None;
        }
        let last = match &update {
            Status::Synced(report) => Some(report.clone()),
            _ => running.latest.last.clone(),
        };
        running.latest = Syncing {
            folder: folder.to_path_buf(),
            status: update,
            last,
        };
        Some(running.latest.clone())
    }

    fn take(&mut self, folder: &Path) -> Option<Session> {
        self.running.remove(folder).map(|running| running.session)
    }

    fn drain(&mut self) -> Vec<Session> {
        self.running
            .drain()
            .map(|(_, running)| running.session)
            .collect()
    }

    fn snapshot(&self) -> Vec<Syncing> {
        let mut all: Vec<_> = self
            .running
            .values()
            .map(|running| running.latest.clone())
            .collect();
        all.sort_by(|a, b| a.folder.cmp(&b.folder));
        all
    }
}

pub async fn stop_all(desktop: &Desktop) {
    let sessions = desktop.folders.lock().await.drain();
    for session in sessions {
        session.stop().await;
    }
}

pub async fn resume_saved(app: &AppHandle, desktop: &Desktop) {
    let Some(account) = account(desktop).await else {
        return;
    };
    let Ok(directory) = app.path().app_config_dir() else {
        return;
    };
    for folder in Remembered::load(&directory).of(&account) {
        let running = desktop.folders.lock().await.running.contains_key(folder);
        if !running {
            let _ = begin(app, desktop, folder.clone()).await;
        }
    }
}

async fn account(desktop: &Desktop) -> Option<String> {
    desktop
        .credentials
        .lock()
        .await
        .as_ref()
        .map(|credentials| credentials.email.clone())
}

async fn remember(
    app: &AppHandle,
    desktop: &Desktop,
    folder: PathBuf,
    keep: bool,
) -> Result<(), String> {
    let account = account(desktop).await.ok_or("not connected to a server")?;
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    let mut remembered = Remembered::load(&directory);
    if keep {
        remembered.add(&account, folder);
    } else {
        remembered.remove(&account, &folder);
    }
    remembered
        .save(&directory)
        .map_err(|error| format!("the synced folders could not be saved: {error}"))
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<PathBuf>, String> {
    let (chosen, answer) = oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = chosen.send(folder);
    });

    let Some(folder) = answer
        .await
        .map_err(|_| "the folder picker went away".to_owned())?
    else {
        return Ok(None);
    };

    folder
        .into_path()
        .map(Some)
        .map_err(|error| format!("that folder is not one this computer can open: {error}"))
}

async fn begin(app: &AppHandle, desktop: &Desktop, folder: PathBuf) -> Result<(), String> {
    let credentials = desktop
        .credentials
        .lock()
        .await
        .clone()
        .ok_or("not connected to a server")?;

    let remote = crate::remote_for(&credentials).map_err(|error| error.to_string())?;
    let engine = Engine::open(folder.clone(), remote).map_err(|error| error.to_string())?;
    let session =
        watch(engine, Debounce::default(), DEFAULT_POLL).map_err(|error| error.to_string())?;
    let mut status = session.subscribe();

    let (generation, replaced) = desktop.folders.lock().await.insert(folder.clone(), session);
    if let Some(replaced) = replaced {
        replaced.stop().await;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Ok(update) = status.recv().await {
            let stopped = matches!(update, Status::Stopped);
            let desktop = app.state::<Desktop>();
            let recorded = desktop
                .folders
                .lock()
                .await
                .record(&folder, generation, update);
            let Some(syncing) = recorded else {
                break;
            };
            let _ = app.emit(STATUS_EVENT, syncing);
            if stopped {
                break;
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn start_sync(
    app: AppHandle,
    desktop: State<'_, Desktop>,
    folder: PathBuf,
) -> Result<(), String> {
    begin(&app, &desktop, folder.clone()).await?;
    remember(&app, &desktop, folder, true).await
}

#[tauri::command]
pub async fn sync_control(
    app: AppHandle,
    desktop: State<'_, Desktop>,
    folder: PathBuf,
    command: Command,
) -> Result<(), String> {
    if command == Command::Stop {
        let session = desktop.folders.lock().await.take(&folder);
        let session = session.ok_or("that folder is not syncing")?;
        session.stop().await;
        return remember(&app, &desktop, folder, false).await;
    }

    let guard = desktop.folders.lock().await;
    let running = guard
        .running
        .get(&folder)
        .ok_or("that folder is not syncing")?;
    running.session.send(command).await;
    Ok(())
}

#[tauri::command]
pub async fn sync_status(desktop: State<'_, Desktop>) -> Result<Vec<Syncing>, String> {
    Ok(desktop.folders.lock().await.snapshot())
}
