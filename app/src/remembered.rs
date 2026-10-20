use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE: &str = "synced-folders.json";

#[derive(Default, Serialize, Deserialize)]
pub struct Remembered {
    folders: BTreeMap<String, Vec<PathBuf>>,
}

impl Remembered {
    pub fn load(directory: &Path) -> Self {
        fs::read(directory.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, directory: &Path) -> io::Result<()> {
        fs::create_dir_all(directory)?;
        let encoded = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let target = directory.join(FILE);
        let temporary = directory.join(format!("{FILE}.tmp"));
        fs::write(&temporary, encoded)?;
        fs::rename(temporary, target)
    }

    pub fn of(&self, account: &str) -> &[PathBuf] {
        self.folders.get(account).map_or(&[], Vec::as_slice)
    }

    pub fn add(&mut self, account: &str, folder: PathBuf) {
        let folders = self.folders.entry(account.to_owned()).or_default();
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }

    pub fn remove(&mut self, account: &str, folder: &Path) {
        if let Some(folders) = self.folders.get_mut(account) {
            folders.retain(|kept| kept != folder);
            if folders.is_empty() {
                self.folders.remove(account);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("stashden-remembered-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn adding_twice_keeps_one_entry_per_account() {
        let mut remembered = Remembered::default();
        remembered.add("a@x", PathBuf::from("/data/one"));
        remembered.add("a@x", PathBuf::from("/data/one"));
        remembered.add("b@x", PathBuf::from("/data/one"));

        assert_eq!(remembered.of("a@x"), [PathBuf::from("/data/one")]);
        assert_eq!(remembered.of("b@x"), [PathBuf::from("/data/one")]);
    }

    #[test]
    fn removing_only_touches_the_named_account_and_folder() {
        let mut remembered = Remembered::default();
        remembered.add("a@x", PathBuf::from("/one"));
        remembered.add("a@x", PathBuf::from("/two"));
        remembered.add("b@x", PathBuf::from("/one"));

        remembered.remove("a@x", Path::new("/one"));

        assert_eq!(remembered.of("a@x"), [PathBuf::from("/two")]);
        assert_eq!(remembered.of("b@x"), [PathBuf::from("/one")]);
        assert_eq!(remembered.of("nobody"), Vec::<PathBuf>::new());
    }

    #[test]
    fn survives_a_round_trip_through_disk() {
        let directory = scratch("round-trip");
        let mut remembered = Remembered::default();
        remembered.add("a@x", PathBuf::from("/one"));
        remembered.save(&directory).unwrap();

        assert_eq!(
            Remembered::load(&directory).of("a@x"),
            [PathBuf::from("/one")]
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_missing_or_corrupt_file_loads_as_empty() {
        let directory = scratch("corrupt");
        assert_eq!(
            Remembered::load(&directory).of("a@x"),
            Vec::<PathBuf>::new()
        );

        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(FILE), b"{not json").unwrap();
        assert_eq!(
            Remembered::load(&directory).of("a@x"),
            Vec::<PathBuf>::new()
        );
        let _ = fs::remove_dir_all(&directory);
    }
}
