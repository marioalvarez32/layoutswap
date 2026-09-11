//! Diagnostics: one zip with everything needed to debug a switch (CODING_STANDARDS,
//! "Errors and logging"): the app log, the layout's switch log, the last probe output,
//! the generated script and the config. The zip always has exactly these five members;
//! a file that is not on disk becomes a one-line note under the same name, so the
//! reader learns it was missing rather than wondering whether the app forgot it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::{App, LAST_PROBE_NAME};
use crate::config::layouts::{Layout, SWITCH_LOG_NAME, SWITCH_SCRIPT_NAME};
use crate::config::store::CONFIG_FILE_NAME;
use crate::error::AppError;

/// The member names, in the order they are written.
pub const DIAGNOSTICS_MEMBERS: [&str; 5] = [
    super::log::APP_LOG_NAME,
    SWITCH_LOG_NAME,
    LAST_PROBE_NAME,
    SWITCH_SCRIPT_NAME,
    CONFIG_FILE_NAME,
];

impl App {
    /// `layoutswap-diagnostics-<layout folder>-<stamp>.zip`: the folder is already a
    /// safe file name, and the stamp comes from the caller so tests stay still.
    pub fn diagnostics_file_name(layout: &Layout, stamp: &str) -> String {
        format!("layoutswap-diagnostics-{}-{stamp}.zip", layout.folder)
    }

    /// Save diagnostics: names the file after the layout and the moment, asks `ask`
    /// where to put it, and writes the zip there. `None` when the user cancelled.
    pub fn save_diagnostics(
        &self,
        layout_id: &str,
        ask: impl FnOnce(&str) -> Option<PathBuf>,
    ) -> Result<Option<PathBuf>, AppError> {
        let layout = self.find_layout(layout_id)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        match ask(&Self::diagnostics_file_name(&layout, &stamp)) {
            None => Ok(None),
            Some(target) => self.write_diagnostics(layout_id, &target).map(Some),
        }
    }

    /// Writes the diagnostics zip for `layout_id` to `target` and returns that path.
    pub fn write_diagnostics(&self, layout_id: &str, target: &Path) -> Result<PathBuf, AppError> {
        let layout = self.find_layout(layout_id)?;
        let sources: [(&str, PathBuf); 5] = [
            (DIAGNOSTICS_MEMBERS[0], self.app_log_path()),
            (DIAGNOSTICS_MEMBERS[1], self.switch_log_path(&layout)),
            (DIAGNOSTICS_MEMBERS[2], self.root().join(LAST_PROBE_NAME)),
            (DIAGNOSTICS_MEMBERS[3], self.switch_script_path(&layout)),
            (DIAGNOSTICS_MEMBERS[4], self.store.path().to_path_buf()),
        ];
        let write_error = |source: std::io::Error| AppError::DiagnosticsWrite {
            path: target.to_path_buf(),
            source,
        };
        let zip_error = |source: zip::result::ZipError| write_error(std::io::Error::other(source));

        // Logged first, so the zip's copy of the app log ends with this line.
        self.log(format!(
            "diagnostics for {}: saving to {}",
            layout.name,
            target.display()
        ));
        let file = fs::File::create(target).map_err(write_error)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, path) in &sources {
            zip.start_file(*name, options).map_err(zip_error)?;
            match fs::read(path) {
                Ok(bytes) => zip.write_all(&bytes).map_err(write_error)?,
                Err(_) => zip
                    .write_all(format!("layoutswap: {name} was not present at {}\n", path.display()).as_bytes())
                    .map_err(write_error)?,
            }
        }
        zip.finish().map_err(zip_error)?;
        Ok(target.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::layouts::CaptureOutcome;
    use crate::script::run::FakeScriptRunner;
    use std::io::Read;
    use std::sync::Arc;

    const FIVE: &str = include_str!("../hardware/fixtures/five-monitors.json");

    fn members(path: &Path) -> Vec<(String, String)> {
        let file = fs::File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        (0..archive.len())
            .map(|i| {
                let mut entry = archive.by_index(i).unwrap();
                let mut text = String::new();
                entry.read_to_string(&mut text).unwrap();
                (entry.name().to_string(), text)
            })
            .collect()
    }

    #[test]
    fn the_zip_holds_exactly_the_five_members_with_their_contents() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        app.probe().unwrap();
        let layout = match app.capture("Desk", None).unwrap() {
            CaptureOutcome::Saved { layout } => *layout,
            other => panic!("expected Saved, got {other:?}"),
        };
        let log = app.switch_script_path(&layout).with_file_name(SWITCH_LOG_NAME);
        fs::write(&log, "2026-09-10 18:00:00  === Switch to Desk ===\n").unwrap();

        let target = dir.path().join("out.zip");
        let written = app.write_diagnostics(&layout.id, &target).unwrap();
        assert_eq!(written, target);

        let members = members(&target);
        let names: Vec<&str> = members.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "layoutswap.log",
                "switch.log",
                "last-probe.json",
                "switch.ps1",
                "config.json"
            ]
        );
        assert!(members[0].1.contains("probe: 5 monitors, 4 active"), "{}", members[0].1);
        assert!(members[0].1.contains("diagnostics for Desk: saving to"));
        assert!(members[1].1.contains("=== Switch to Desk ==="));
        assert!(members[2].1.contains("\"monitors\""));
        assert!(members[3].1.contains("#Requires -Version 5.1"));
        assert!(members[4].1.contains("\"schemaVersion\""));
    }

    #[test]
    fn a_missing_file_becomes_a_note_under_the_same_name() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        app.probe().unwrap();
        let layout = match app.capture("Desk", None).unwrap() {
            CaptureOutcome::Saved { layout } => *layout,
            other => panic!("expected Saved, got {other:?}"),
        };
        let target = dir.path().join("out.zip");
        app.write_diagnostics(&layout.id, &target).unwrap();
        let members = members(&target);
        assert_eq!(members.len(), 5);
        assert!(members[1].1.starts_with("layoutswap: switch.log was not present at"));
    }

    #[test]
    fn save_diagnostics_offers_the_dated_name_and_writes_where_the_user_said() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        app.probe().unwrap();
        let layout = match app.capture("Desk", None).unwrap() {
            CaptureOutcome::Saved { layout } => *layout,
            other => panic!("expected Saved, got {other:?}"),
        };
        let target = dir.path().join("chosen.zip");
        let written = app
            .save_diagnostics(&layout.id, |name| {
                assert!(name.starts_with("layoutswap-diagnostics-desk-"), "{name}");
                assert!(name.ends_with(".zip"));
                assert_eq!(name.len(), "layoutswap-diagnostics-desk-20260910-183012.zip".len());
                Some(target.clone())
            })
            .unwrap();
        assert_eq!(written, Some(target.clone()));
        assert_eq!(members(&target).len(), 5);
        assert_eq!(app.save_diagnostics(&layout.id, |_| None).unwrap(), None);
    }

    #[test]
    fn the_file_name_carries_the_layout_folder_and_the_stamp() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        app.probe().unwrap();
        let layout = match app.capture("Film & TV", None).unwrap() {
            CaptureOutcome::Saved { layout } => *layout,
            other => panic!("expected Saved, got {other:?}"),
        };
        assert_eq!(
            App::diagnostics_file_name(&layout, "20260910-183012"),
            "layoutswap-diagnostics-film-tv-20260910-183012.zip"
        );
        assert!(matches!(
            app.write_diagnostics("nope", &dir.path().join("x.zip")),
            Err(AppError::LayoutNotFound { .. })
        ));
    }
}
