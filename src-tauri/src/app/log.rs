//! The app's own log: `logs\layoutswap.log` in the app root, one line per app-side event
//! in the same shape the generated scripts use (`yyyy-MM-dd HH:mm:ss  text`), so the two
//! logs read alike inside a diagnostics zip.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::App;

pub const LOGS_DIR_NAME: &str = "logs";
pub const APP_LOG_NAME: &str = "layoutswap.log";

impl App {
    pub fn app_log_path(&self) -> PathBuf {
        self.root().join(LOGS_DIR_NAME).join(APP_LOG_NAME)
    }

    /// Appends one line. Best effort: the log is for diagnostics and must never turn one
    /// failure into two.
    pub fn log(&self, text: impl AsRef<str>) {
        append_log_line(&self.app_log_path(), text.as_ref());
    }
}

/// Appends `text` to `path` under a local timestamp, creating the folder and the file.
/// Returns whether the line landed.
pub fn append_log_line(path: &Path, text: &str) -> bool {
    let Some(folder) = path.parent() else {
        return false;
    };
    if fs::create_dir_all(folder).is_err() {
        return false;
    }
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };
    writeln!(file, "{}  {text}", stamp()).is_ok()
}

fn stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_land_under_a_timestamp_in_the_script_log_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs").join("layoutswap.log");
        assert!(append_log_line(&path, "probe: 5 monitors, 4 active"));
        assert!(append_log_line(&path, "switch Desk: started"));
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            // "2026-09-10 18:30:12  text": ten date characters, a space, eight time
            // characters, two spaces.
            assert_eq!(&line[10..11], " ", "{line}");
            assert_eq!(&line[19..21], "  ", "{line}");
            assert!(line[..10].chars().filter(|c| *c == '-').count() == 2, "{line}");
        }
        assert!(lines[1].ends_with("switch Desk: started"));
    }
}
