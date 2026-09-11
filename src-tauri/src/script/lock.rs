//! The lock a switch script takes: `switch.lock` in the app root, holding one line
//! `pid=<n>` and one line `layout=<name>`. The script writes it before its first step and
//! removes it on exit, so one switch runs at a time, from the app or from a shortcut.
//! The app reads it to refuse a switch while another holds it, and clears a lock a
//! killed script could not remove itself.

use std::fs;
use std::path::Path;

pub const LOCK_FILE_NAME: &str = "switch.lock";

/// Who holds the lock, as far as the file says. The `pid=` line is for the script's
/// own message; the app names the layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockHolder {
    /// The layout named in the file, or `None` when the file does not say.
    pub layout: Option<String>,
}

impl LockHolder {
    /// The layout name for a message: "another layout" when the file does not say.
    pub fn layout_name(&self) -> String {
        self.layout
            .clone()
            .unwrap_or_else(|| "another layout".to_string())
    }
}

/// The lock's holder when the file exists. An unreadable file still counts as held.
pub fn read_lock(path: &Path) -> Option<LockHolder> {
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut holder = LockHolder { layout: None };
    for line in text.lines() {
        if let Some(layout) = line.strip_prefix("layout=") {
            let layout = layout.trim();
            if !layout.is_empty() {
                holder.layout = Some(layout.to_string());
            }
        }
    }
    Some(holder)
}

/// Removes the lock when it names `layout`, as a killed script leaves it. A lock held
/// by another layout is left alone. Returns whether a file was removed.
pub fn clear_lock_of(path: &Path, layout: &str) -> bool {
    match read_lock(path) {
        Some(holder) if holder.layout.as_deref() == Some(layout) => {
            fs::remove_file(path).is_ok()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_lock_is_nobody() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_lock(&dir.path().join(LOCK_FILE_NAME)), None);
    }

    #[test]
    fn reads_the_layout_the_script_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE_NAME);
        fs::write(&path, "pid=4242\r\nlayout=Desk\r\n").unwrap();
        let holder = read_lock(&path).unwrap();
        assert_eq!(holder.layout.as_deref(), Some("Desk"));
        assert_eq!(holder.layout_name(), "Desk");
    }

    #[test]
    fn an_empty_or_odd_lock_still_counts_as_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE_NAME);
        fs::write(&path, "").unwrap();
        let holder = read_lock(&path).unwrap();
        assert_eq!(holder.layout, None);
        assert_eq!(holder.layout_name(), "another layout");
    }

    #[test]
    fn clears_only_the_lock_of_the_named_layout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE_NAME);
        fs::write(&path, "pid=1\r\nlayout=Film\r\n").unwrap();
        assert!(!clear_lock_of(&path, "Desk"));
        assert!(path.exists());
        assert!(clear_lock_of(&path, "Film"));
        assert!(!path.exists());
        assert!(!clear_lock_of(&path, "Film"));
    }
}
