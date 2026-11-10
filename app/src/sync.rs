use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use stashden_client::sync::watch::{Command, DEFAULT_POLL, Session, Status, watch};
use stashden_client::{Debounce, Engine, Report};

use crate::desktop::Desktop;
use crate::failure::Failure;
use crate::remembered::Remembered;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Syncing {
    pub folder: PathBuf,
    pub status: Status,
    pub last: Option<Report>,
}

struct Running<S> {
    session: S,
    generation: u64,
    latest: Syncing,
}

pub struct Folders<S = Session> {
    generation: u64,
    running: HashMap<PathBuf, Running<S>>,
    active: Arc<AtomicBool>,
}

impl<S> Default for Folders<S> {
    fn default() -> Self {
        Self {
            generation: 0,
            running: HashMap::new(),
            active: Arc::default(),
        }
    }
}

impl<S> Folders<S> {
    pub fn active(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.active)
    }

    fn publish(&self) {
        self.active
            .store(!self.running.is_empty(), Ordering::Relaxed);
    }

    fn contains(&self, folder: &Path) -> bool {
        self.running.contains_key(folder)
    }

    fn session(&self, folder: &Path) -> Option<&S> {
        self.running.get(folder).map(|running| &running.session)
    }

    fn insert(&mut self, folder: PathBuf, session: S) -> (u64, Option<S>) {
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
        self.publish();
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
        let syncing = running.latest.clone();
        if matches!(syncing.status, Status::Stopped) {
            self.running.remove(folder);
            self.publish();
        }
        Some(syncing)
    }

    fn take(&mut self, folder: &Path) -> Option<S> {
        let taken = self.running.remove(folder).map(|running| running.session);
        self.publish();
        taken
    }

    fn drain(&mut self) -> Vec<S> {
        let drained = self
            .running
            .drain()
            .map(|(_, running)| running.session)
            .collect();
        self.publish();
        drained
    }

    pub fn snapshot(&self) -> Vec<Syncing> {
        let mut all: Vec<_> = self
            .running
            .values()
            .map(|running| running.latest.clone())
            .collect();
        all.sort_by(|a, b| a.folder.cmp(&b.folder));
        all
    }
}

impl Desktop {
    pub async fn stop_all_syncs(&self) {
        let sessions = self.folders.lock().await.drain();
        for session in sessions {
            session.stop().await;
        }
    }

    pub async fn resume_saved_syncs(self: &Arc<Self>) {
        let Some(account) = self.account_email() else {
            return;
        };
        for folder in Remembered::load(&self.config).of(&account) {
            if !self.folders.lock().await.contains(folder) {
                let _ = self.begin(folder.clone()).await;
            }
        }
    }

    pub async fn start_sync(self: &Arc<Self>, folder: PathBuf) -> Result<(), Failure> {
        self.begin(folder.clone()).await?;
        self.remember(folder, true)
    }

    pub async fn control_sync(&self, folder: PathBuf, command: Command) -> Result<(), Failure> {
        if command == Command::Stop {
            let session = self.folders.lock().await.take(&folder);
            let session = session.ok_or("that folder is not syncing")?;
            session.stop().await;
            self.changes
                .send(Syncing {
                    folder: folder.clone(),
                    status: Status::Stopped,
                    last: None,
                })
                .ok();
            return self.remember(folder, false);
        }

        let folders = self.folders.lock().await;
        let session = folders
            .session(&folder)
            .ok_or("that folder is not syncing")?;
        session.send(command).await;
        Ok(())
    }

    pub async fn syncing(&self) -> Vec<Syncing> {
        self.folders.lock().await.snapshot()
    }

    pub fn is_syncing(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    fn remember(&self, folder: PathBuf, keep: bool) -> Result<(), Failure> {
        let account = self.account_email().ok_or("not connected to a server")?;
        let mut remembered = Remembered::load(&self.config);
        if keep {
            remembered.add(&account, folder);
        } else {
            remembered.remove(&account, &folder);
        }
        remembered.save(&self.config).map_err(|error| {
            Failure::from(format!("the synced folders could not be saved: {error}"))
        })
    }

    async fn begin(self: &Arc<Self>, folder: PathBuf) -> Result<(), Failure> {
        let remote = self.fresh_remote()?;
        let engine = Engine::open(folder.clone(), remote).map_err(|error| error.to_string())?;
        let session =
            watch(engine, Debounce::default(), DEFAULT_POLL).map_err(|error| error.to_string())?;
        let mut status = session.subscribe();

        let (generation, replaced) = self.folders.lock().await.insert(folder.clone(), session);
        if let Some(replaced) = replaced {
            replaced.stop().await;
        }

        let desktop = Arc::clone(self);
        tokio::spawn(async move {
            while let Ok(update) = status.recv().await {
                let stopped = matches!(update, Status::Stopped);
                let recorded = desktop
                    .folders
                    .lock()
                    .await
                    .record(&folder, generation, update);
                let Some(syncing) = recorded else {
                    break;
                };
                desktop.changes.send(syncing).ok();
                if stopped {
                    break;
                }
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(uploaded: usize) -> Report {
        Report {
            uploaded,
            ..Report::default()
        }
    }

    #[test]
    fn a_new_folder_starts_idle_with_no_last_pass() {
        let mut folders = Folders::<()>::default();
        folders.insert(PathBuf::from("/b"), ());
        folders.insert(PathBuf::from("/a"), ());

        let snapshot = folders.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].folder, PathBuf::from("/a"));
        assert_eq!(snapshot[0].status, Status::Idle);
        assert_eq!(snapshot[0].last, None);
    }

    #[test]
    fn the_last_report_survives_later_statuses_until_another_pass_replaces_it() {
        let mut folders = Folders::<()>::default();
        let folder = PathBuf::from("/a");
        let (generation, _) = folders.insert(folder.clone(), ());

        folders.record(&folder, generation, Status::Synced(report(3)));
        let syncing = folders
            .record(&folder, generation, Status::Syncing)
            .unwrap();
        assert_eq!(syncing.last, Some(report(3)));

        let syncing = folders
            .record(&folder, generation, Status::Synced(report(1)))
            .unwrap();
        assert_eq!(syncing.last, Some(report(1)));
    }

    #[test]
    fn a_replaced_session_hands_back_the_old_one_and_its_statuses_are_ignored() {
        let mut folders = Folders::<&str>::default();
        let folder = PathBuf::from("/a");
        let (old, _) = folders.insert(folder.clone(), "old");
        let (new, replaced) = folders.insert(folder.clone(), "new");

        assert_eq!(replaced, Some("old"));
        assert_ne!(old, new);
        assert_eq!(folders.record(&folder, old, Status::Syncing), None);
        assert_eq!(folders.snapshot()[0].status, Status::Idle);
        assert!(folders.record(&folder, new, Status::Syncing).is_some());
    }

    #[test]
    fn a_stopped_folder_leaves_the_list() {
        let mut folders = Folders::<()>::default();
        let folder = PathBuf::from("/a");
        let (generation, _) = folders.insert(folder.clone(), ());

        let stopped = folders
            .record(&folder, generation, Status::Stopped)
            .unwrap();
        assert_eq!(stopped.status, Status::Stopped);
        assert_eq!(folders.snapshot(), Vec::new());
        assert_eq!(folders.record(&folder, generation, Status::Idle), None);
    }

    #[test]
    fn the_shared_flag_follows_whether_any_folder_is_syncing() {
        let mut folders = Folders::<u8>::default();
        let active = folders.active();
        assert!(!active.load(Ordering::Relaxed));
        let (generation, _) = folders.insert(PathBuf::from("/a"), 1);
        folders.insert(PathBuf::from("/b"), 2);
        assert!(active.load(Ordering::Relaxed));
        folders.take(Path::new("/a"));
        assert!(active.load(Ordering::Relaxed));
        folders.take(Path::new("/b"));
        assert!(!active.load(Ordering::Relaxed));
        folders.insert(PathBuf::from("/a"), 1);
        folders.record(Path::new("/a"), generation + 2, Status::Stopped);
        assert!(!active.load(Ordering::Relaxed));
        folders.insert(PathBuf::from("/a"), 1);
        folders.drain();
        assert!(!active.load(Ordering::Relaxed));
    }

    #[test]
    fn draining_hands_back_every_session_and_empties_the_list() {
        let mut folders = Folders::<u8>::default();
        folders.insert(PathBuf::from("/a"), 1);
        folders.insert(PathBuf::from("/b"), 2);

        let mut drained = folders.drain();
        drained.sort_unstable();
        assert_eq!(drained, [1, 2]);
        assert_eq!(folders.snapshot(), Vec::new());
        assert_eq!(folders.take(Path::new("/a")), None);
    }
}
