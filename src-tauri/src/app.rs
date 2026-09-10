//! The composition root the commands call: the config store, the script runner and
//! the latest probe, wired together under one app root. Commands stay one line each;
//! the tests here drive the same functions through a fake runner and a temp directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::config::layouts::{self, CaptureOutcome, Layout};
use crate::config::store::ConfigStore;
use crate::config::{Config, WindowSize};
use crate::error::AppError;
use crate::hardware::{self, Inventory};
use crate::script::render;
use crate::script::run::ScriptRunner;

pub const PROBE_SCRIPT_NAME: &str = "probe.ps1";
pub const LAST_PROBE_NAME: &str = "last-probe.json";
pub const PROBE_LOG_NAME: &str = "probe.log";
pub const LAYOUTS_DIR_NAME: &str = "layouts";

pub struct App {
    store: ConfigStore,
    runner: Arc<dyn ScriptRunner>,
    last_probe: Mutex<Option<Inventory>>,
}

impl App {
    pub fn new(root: impl Into<PathBuf>, runner: Arc<dyn ScriptRunner>) -> Self {
        App {
            store: ConfigStore::new(root),
            runner,
            last_probe: Mutex::new(None),
        }
    }

    pub fn root(&self) -> &Path {
        self.store.root()
    }

    pub fn probe_script_path(&self) -> PathBuf {
        self.root().join(PROBE_SCRIPT_NAME)
    }

    /// Writes the probe script into the app root, replacing the previous one, so the
    /// probe on disk always matches the running app.
    pub fn write_probe_script(&self, rendered_at: &str) -> Result<PathBuf, AppError> {
        let path = self.probe_script_path();
        fs::create_dir_all(self.root()).map_err(|source| AppError::ScriptWrite {
            path: path.clone(),
            source,
        })?;
        fs::write(&path, render::render_probe(rendered_at).text).map_err(|source| {
            AppError::ScriptWrite {
                path: path.clone(),
                source,
            }
        })?;
        Ok(path)
    }

    /// Runs the probe, remembers the result for the next capture, and keeps a copy on
    /// disk for diagnostics. A failed run lands in `probe.log`, and the error names it.
    pub fn probe(&self) -> Result<Inventory, AppError> {
        let inventory = match hardware::probe(self.runner.as_ref(), &self.probe_script_path()) {
            Ok(inventory) => inventory,
            Err(AppError::ProbeFailed {
                exit_code, stderr, ..
            }) => {
                let log_path = self.append_probe_log(exit_code, &stderr);
                return Err(AppError::ProbeFailed {
                    exit_code,
                    stderr,
                    log_path,
                });
            }
            Err(other) => return Err(other),
        };
        if let Ok(json) = serde_json::to_string_pretty(&inventory) {
            // Best effort: a failed diagnostics copy must not fail the probe.
            let _ = fs::create_dir_all(self.root());
            let _ = fs::write(self.root().join(LAST_PROBE_NAME), json);
        }
        *self.last_probe() = Some(inventory.clone());
        Ok(inventory)
    }

    /// Best effort: the log is for diagnostics and must not turn one failure into two.
    fn append_probe_log(&self, exit_code: i32, stderr: &str) -> Option<PathBuf> {
        use std::io::Write;
        let path = self.root().join(PROBE_LOG_NAME);
        let _ = fs::create_dir_all(self.root());
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()?;
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(file, "{stamp}  probe exited with exit code {exit_code}").ok()?;
        for line in stderr.lines() {
            writeln!(file, "{stamp}  {line}").ok()?;
        }
        Some(path)
    }

    pub fn load_config(&self) -> Result<Config, AppError> {
        self.store.load()
    }

    pub fn update_window_size(&self, size: WindowSize) -> Result<(), AppError> {
        self.store.update_window_size(size)
    }

    /// Capture: the latest probe becomes a layout with the given name. With `replace_id`
    /// the layout with that id is re-captured under the same id; without it, a name
    /// another layout already has (compared without case) is reported, not overwritten.
    pub fn capture(
        &self,
        name: &str,
        replace_id: Option<&str>,
    ) -> Result<CaptureOutcome, AppError> {
        let name = layouts::validate_name(name)?;
        let inventory = self.last_probe().clone().ok_or(AppError::NoProbeYet)?;
        let mut config = self.store.load()?;

        if let Some(existing) = config
            .layouts
            .iter()
            .find(|l| layouts::same_name(&l.name, &name) && Some(l.id.as_str()) != replace_id)
        {
            return Ok(CaptureOutcome::NameTaken {
                id: existing.id.clone(),
                name: existing.name.clone(),
            });
        }

        let summary = layouts::summarise(&inventory);
        let captured_at = chrono::Local::now().to_rfc3339();
        let layout = match replace_id {
            Some(id) => {
                let index = config
                    .layouts
                    .iter()
                    .position(|l| l.id == id)
                    .ok_or_else(|| AppError::LayoutNotFound { id: id.to_string() })?;
                // A replace reaches here only under the same name (case aside), so the
                // folder stays; renaming a layout is a later slice.
                let folder = config.layouts[index].folder.clone();
                let layout = Layout {
                    id: id.to_string(),
                    name,
                    folder,
                    captured_at,
                    arrangement: inventory.arrangement.clone(),
                    summary,
                };
                config.layouts[index] = layout.clone();
                layout
            }
            None => {
                let id = layouts::new_id();
                let taken: Vec<&str> = config.layouts.iter().map(|l| l.folder.as_str()).collect();
                let folder = layouts::folder_name(&name, &id, &taken);
                let layout = Layout {
                    id,
                    name,
                    folder,
                    captured_at,
                    arrangement: inventory.arrangement.clone(),
                    summary,
                };
                config.layouts.push(layout.clone());
                layout
            }
        };

        self.ensure_layout_folder(&layout.folder)?;
        self.store.save(&config)?;
        Ok(CaptureOutcome::Saved { layout })
    }

    fn layouts_dir(&self) -> PathBuf {
        self.root().join(LAYOUTS_DIR_NAME)
    }

    fn ensure_layout_folder(&self, folder: &str) -> Result<(), AppError> {
        let path = self.layouts_dir().join(folder);
        fs::create_dir_all(&path).map_err(|source| AppError::LayoutFolder { path, source })
    }

    fn last_probe(&self) -> std::sync::MutexGuard<'_, Option<Inventory>> {
        self.last_probe
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::MonitorState;
    use crate::script::run::FakeScriptRunner;
    use tempfile::TempDir;

    const FIVE: &str = include_str!("hardware/fixtures/five-monitors.json");

    fn app() -> (TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        (dir, app)
    }

    fn saved(outcome: CaptureOutcome) -> Layout {
        match outcome {
            CaptureOutcome::Saved { layout } => layout,
            other => panic!("expected Saved, got {other:?}"),
        }
    }

    #[test]
    fn the_probe_script_is_written_into_the_app_root_and_replaced_each_time() {
        let (_dir, app) = app();
        let path = app.write_probe_script("first").unwrap();
        assert_eq!(path, app.root().join("probe.ps1"));
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("# rendered first"));
        app.write_probe_script("second").unwrap();
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("# rendered second"));
    }

    #[test]
    fn probe_runs_the_script_in_the_app_root_and_keeps_a_copy() {
        let (_dir, app) = app();
        let inventory = app.probe().unwrap();
        assert_eq!(inventory.monitors.len(), 5);
        let copy = fs::read_to_string(app.root().join("last-probe.json")).unwrap();
        assert!(copy.contains("\"monitors\""));
    }

    #[test]
    fn a_failed_probe_is_logged_and_the_error_names_the_log() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::returning(
                crate::script::run::ScriptOutput {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: "probe failed: QueryDisplayConfig failed with error 87".into(),
                },
            )),
        );
        let error = app.probe().unwrap_err();
        let log = app.root().join("probe.log");
        match &error {
            AppError::ProbeFailed { log_path, .. } => {
                assert_eq!(log_path.as_deref(), Some(log.as_path()))
            }
            other => panic!("expected ProbeFailed, got {other:?}"),
        }
        let text = fs::read_to_string(&log).unwrap();
        assert!(text.contains("exit code 1"), "{text}");
        assert!(text.contains("error 87"), "{text}");
        let payload = crate::error::AppErrorPayload::from(&error);
        assert_eq!(payload.log_path, Some(log.display().to_string()));
    }

    #[test]
    fn capture_before_a_probe_says_to_wait() {
        let (_dir, app) = app();
        let error = app.capture("Desk", None).unwrap_err();
        assert!(matches!(error, AppError::NoProbeYet));
    }

    #[test]
    fn capture_stores_a_layout_with_its_summary_and_folder() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let layout = saved(app.capture("  Desk ", None).unwrap());
        assert_eq!(layout.name, "Desk");
        assert_eq!(layout.folder, "desk");
        assert_eq!(layout.id.len(), 32);
        assert_eq!(layout.summary.monitors.len(), 5);
        assert_eq!(layout.summary.monitors.iter().filter(|m| m.on).count(), 4);
        assert_eq!(layout.arrangement.source_adapters.len(), 4);
        assert!(app.root().join("layouts").join("desk").is_dir());

        let config = app.load_config().unwrap();
        assert_eq!(config.layouts, vec![layout]);
    }

    #[test]
    fn capture_with_an_existing_name_reports_the_conflict_and_stores_nothing() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let first = saved(app.capture("Desk", None).unwrap());
        let outcome = app.capture("desk", None).unwrap();
        assert_eq!(
            outcome,
            CaptureOutcome::NameTaken {
                id: first.id.clone(),
                name: "Desk".into()
            }
        );
        assert_eq!(app.load_config().unwrap().layouts.len(), 1);
    }

    #[test]
    fn replace_keeps_the_id_and_folder_and_takes_the_new_capture() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let first = saved(app.capture("Desk", None).unwrap());

        let replaced = saved(app.capture("desk", Some(&first.id)).unwrap());
        assert_eq!(replaced.id, first.id);
        assert_eq!(replaced.folder, "desk");
        assert_eq!(replaced.name, "desk");
        let config = app.load_config().unwrap();
        assert_eq!(config.layouts.len(), 1);
        assert_eq!(config.layouts[0].id, first.id);
    }

    #[test]
    fn replace_with_an_unknown_id_is_an_error() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let error = app.capture("Desk", Some("nope")).unwrap_err();
        assert!(matches!(error, AppError::LayoutNotFound { .. }));
    }

    #[test]
    fn two_layouts_whose_names_share_a_slug_get_distinct_folders() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let a = saved(app.capture("Desk!", None).unwrap());
        let b = saved(app.capture("Desk?", None).unwrap());
        assert_eq!(a.folder, "desk");
        assert_eq!(b.folder, format!("desk-{}", layouts::short_id(&b.id)));
        assert!(app.root().join("layouts").join(&b.folder).is_dir());
    }

    #[test]
    fn an_invalid_name_is_refused_before_anything_is_written() {
        let (_dir, app) = app();
        app.probe().unwrap();
        assert!(matches!(
            app.capture("   ", None).unwrap_err(),
            AppError::InvalidLayoutName { .. }
        ));
        assert!(!app.store.path().exists());
    }

    #[test]
    fn the_summary_records_on_and_off_from_the_probe_states() {
        let (_dir, app) = app();
        let inventory = app.probe().unwrap();
        let layout = saved(app.capture("Desk", None).unwrap());
        for monitor in &inventory.monitors {
            let summary = layout
                .summary
                .monitors
                .iter()
                .find(|m| m.device_path == monitor.device_path)
                .unwrap();
            assert_eq!(summary.on, monitor.state == MonitorState::Active);
        }
    }
}
