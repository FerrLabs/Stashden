use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;

use chrono::Utc;

use crate::desktop::Desktop;

const FILE: &str = "stashden.log";

pub fn record(message: &str) {
    eprintln!("{message}");
    let _ = append(&Desktop::config_directory(), message);
}

fn append(directory: &Path, message: &str) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join(FILE))?;
    writeln!(file, "{} {message}", Utc::now().to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let directory =
            std::env::temp_dir().join(format!("stashden-log-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn messages_are_appended_one_per_line_and_the_folder_is_created() {
        let directory = scratch("append").join("not-yet-there");

        append(&directory, "first").unwrap();
        append(&directory, "second").unwrap();

        let written = fs::read_to_string(directory.join(FILE)).unwrap();
        let lines: Vec<_> = written.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with(" first"));
        assert!(lines[1].ends_with(" second"));
    }

    #[test]
    fn an_unwritable_folder_is_an_error_not_a_panic() {
        let directory = scratch("blocked");
        fs::create_dir_all(&directory).unwrap();
        let blocker = directory.join("file");
        fs::write(&blocker, "").unwrap();

        assert!(append(&blocker, "lost").is_err());
    }
}
