//! The composition root the commands call: the config store, the script runner, the
//! latest probe and the running switch, wired together under one app root. Commands
//! stay one line each; the tests here drive the same functions through a fake runner
//! and a temp directory. The switch itself lives in [`switch`].

mod switch;

pub use switch::{SwitchEvent, SwitchResult};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::config::layouts::{
    self, CaptureOutcome, Layout, ScriptRecord, ScriptState, ScriptStatus, SWITCH_SCRIPT_NAME,
};
use crate::config::store::ConfigStore;
use crate::config::{Config, WindowSize};
use crate::error::AppError;
use crate::hardware::{self, Inventory};
use crate::script::render::{self, TEMPLATE_VERSION};
use crate::script::run::ScriptRunner;

pub const PROBE_SCRIPT_NAME: &str = "probe.ps1";
pub const LAST_PROBE_NAME: &str = "last-probe.json";
pub const PROBE_LOG_NAME: &str = "probe.log";
pub const LAYOUTS_DIR_NAME: &str = "layouts";

pub struct App {
    store: ConfigStore,
    runner: Arc<dyn ScriptRunner>,
    last_probe: Mutex<Option<Inventory>>,
    active_switch: Mutex<Option<switch::ActiveSwitch>>,
}

impl App {
    pub fn new(root: impl Into<PathBuf>, runner: Arc<dyn ScriptRunner>) -> Self {
        App {
            store: ConfigStore::new(root),
            runner,
            last_probe: Mutex::new(None),
            active_switch: Mutex::new(None),
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
        write_script(&path, &render::render_probe(rendered_at).text)?;
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

    /// Capture: the latest probe becomes a layout with the given name, and its switch
    /// script is written. With `replace_id` the layout with that id is re-captured under
    /// the same id; without it, a name another layout already has (compared without
    /// case) is reported, not overwritten.
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
        let captured_at = now();
        let (id, folder, index) = match replace_id {
            Some(id) => {
                let index = config
                    .layouts
                    .iter()
                    .position(|l| l.id == id)
                    .ok_or_else(|| AppError::LayoutNotFound { id: id.to_string() })?;
                // A replace reaches here only under the same name (case aside), so the
                // folder stays; renaming a layout is a later slice.
                (
                    id.to_string(),
                    config.layouts[index].folder.clone(),
                    Some(index),
                )
            }
            None => {
                let id = layouts::new_id();
                let taken: Vec<&str> = config.layouts.iter().map(|l| l.folder.as_str()).collect();
                let folder = layouts::folder_name(&name, &id, &taken);
                (id, folder, None)
            }
        };
        let mut layout = Layout {
            id,
            name,
            folder,
            captured_at,
            arrangement: inventory.arrangement.clone(),
            summary,
            script: None,
        };
        self.write_switch_script(&mut layout, &config)?;
        match index {
            Some(index) => config.layouts[index] = layout.clone(),
            None => config.layouts.push(layout.clone()),
        }
        self.store.save(&config)?;
        Ok(CaptureOutcome::Saved {
            layout: Box::new(layout),
        })
    }

    /// Rewrites one layout's script and clears its stale state.
    pub fn regenerate_script(&self, layout_id: &str) -> Result<Layout, AppError> {
        let mut config = self.store.load()?;
        let index = config
            .layouts
            .iter()
            .position(|l| l.id == layout_id)
            .ok_or_else(|| AppError::LayoutNotFound {
                id: layout_id.to_string(),
            })?;
        let mut layout = config.layouts[index].clone();
        self.write_switch_script(&mut layout, &config)?;
        config.layouts[index] = layout.clone();
        self.store.save(&config)?;
        Ok(layout)
    }

    /// On start: every layout whose script is missing or was rendered by an older
    /// template gets a fresh one, so scripts never silently fall behind the app.
    /// Returns the ids regenerated.
    pub fn regenerate_stale_scripts(&self) -> Result<Vec<String>, AppError> {
        let mut config = self.store.load()?;
        let mut regenerated = Vec::new();
        let snapshot = config.clone();
        for layout in &mut config.layouts {
            let exists = self.switch_script_path(layout).exists();
            if layouts::script_state(layout, TEMPLATE_VERSION, exists) != ScriptState::Current {
                self.write_switch_script(layout, &snapshot)?;
                regenerated.push(layout.id.clone());
            }
        }
        if !regenerated.is_empty() {
            self.store.save(&config)?;
        }
        Ok(regenerated)
    }

    /// The script state of every layout, read from the config and the disk.
    pub fn script_states(&self) -> Result<Vec<ScriptStatus>, AppError> {
        let config = self.store.load()?;
        Ok(config
            .layouts
            .iter()
            .map(|layout| {
                let path = self.switch_script_path(layout);
                ScriptStatus {
                    layout_id: layout.id.clone(),
                    state: layouts::script_state(layout, TEMPLATE_VERSION, path.exists()),
                    path: path.display().to_string(),
                }
            })
            .collect())
    }

    /// Opens a layout's script with whatever Windows associates with `.ps1` files.
    pub fn open_script(&self, layout_id: &str) -> Result<(), AppError> {
        let config = self.store.load()?;
        let layout = config
            .layouts
            .iter()
            .find(|l| l.id == layout_id)
            .ok_or_else(|| AppError::LayoutNotFound {
                id: layout_id.to_string(),
            })?;
        let path = self.switch_script_path(layout);
        if !path.exists() {
            return Err(AppError::ScriptMissing { path });
        }
        open_with_default(&path)
    }

    pub fn switch_script_path(&self, layout: &Layout) -> PathBuf {
        self.layouts_dir()
            .join(&layout.folder)
            .join(SWITCH_SCRIPT_NAME)
    }

    /// Renders the layout's switch script into its folder and records the render on
    /// the layout. `config` supplies the aliases for the labels the script prints.
    fn write_switch_script(&self, layout: &mut Layout, config: &Config) -> Result<(), AppError> {
        let path = self.switch_script_path(layout);
        let folder = path
            .parent()
            .expect("the script sits inside the layout folder");
        fs::create_dir_all(folder).map_err(|source| AppError::LayoutFolder {
            path: folder.to_path_buf(),
            source,
        })?;
        let rendered_at = now();
        let script = render::render_switch(
            layout,
            &config.aliases,
            &self.root().display().to_string(),
            &rendered_at,
        );
        write_script(&path, &script.text)?;
        layout.script = Some(ScriptRecord {
            template_version: TEMPLATE_VERSION,
            rendered_at,
        });
        Ok(())
    }

    fn layouts_dir(&self) -> PathBuf {
        self.root().join(LAYOUTS_DIR_NAME)
    }

    fn last_probe(&self) -> std::sync::MutexGuard<'_, Option<Inventory>> {
        self.last_probe
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Windows PowerShell 5.1 reads a `.ps1` without a byte-order mark as ANSI, so every
/// script is written as UTF-8 with the mark; labels with a middle dot then print right.
fn write_script(path: &Path, text: &str) -> Result<(), AppError> {
    let mut bytes = Vec::with_capacity(text.len() + 3);
    bytes.extend_from_slice(b"\xEF\xBB\xBF");
    bytes.extend_from_slice(text.as_bytes());
    fs::write(path, bytes).map_err(|source| AppError::ScriptWrite {
        path: path.to_path_buf(),
        source,
    })
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

#[cfg(not(test))]
fn open_with_default(path: &Path) -> Result<(), AppError> {
    // `start` goes through the shell association; the empty string is the window title.
    std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|source| AppError::ScriptOpen {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
fn open_with_default(path: &Path) -> Result<(), AppError> {
    OPENED.with(|o| o.borrow_mut().push(path.to_path_buf()));
    Ok(())
}

#[cfg(test)]
thread_local! {
    static OPENED: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
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
            CaptureOutcome::Saved { layout } => *layout,
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
    fn capture_stores_a_layout_with_its_summary_and_writes_its_script() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let layout = saved(app.capture("  Desk ", None).unwrap());
        assert_eq!(layout.name, "Desk");
        assert_eq!(layout.folder, "desk");
        assert_eq!(layout.id.len(), 32);
        assert_eq!(layout.summary.monitors.len(), 5);
        assert_eq!(layout.summary.monitors.iter().filter(|m| m.on).count(), 4);
        assert_eq!(layout.arrangement.source_adapters.len(), 4);

        let script = app.root().join("layouts").join("desk").join("switch.ps1");
        let bytes = fs::read(&script).unwrap();
        assert_eq!(
            &bytes[..3],
            b"\xEF\xBB\xBF",
            "byte-order mark for PowerShell 5.1"
        );
        let text = fs::read_to_string(&script).unwrap();
        assert!(text
            .trim_start_matches('\u{feff}')
            .starts_with("#Requires -Version 5.1"));
        assert!(text.contains("switch to Desk"));
        assert!(text.contains(&format!("$AppRoot       = '{}'", app.root().display())));
        assert_eq!(
            layout.script.as_ref().map(|s| s.template_version),
            Some(TEMPLATE_VERSION)
        );

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
    fn replace_keeps_the_id_and_folder_and_rewrites_the_script() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let first = saved(app.capture("Desk", None).unwrap());
        let first_script = fs::read_to_string(app.switch_script_path(&first)).unwrap();

        let replaced = saved(app.capture("desk", Some(&first.id)).unwrap());
        assert_eq!(replaced.id, first.id);
        assert_eq!(replaced.folder, "desk");
        assert_eq!(replaced.name, "desk");
        let second_script = fs::read_to_string(app.switch_script_path(&replaced)).unwrap();
        assert!(second_script.contains("switch to desk"));
        assert_ne!(first_script, second_script);
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
        assert!(app.switch_script_path(&b).exists());
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

    #[test]
    fn script_states_read_the_config_and_the_disk() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let layout = saved(app.capture("Desk", None).unwrap());
        let states = app.script_states().unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].layout_id, layout.id);
        assert_eq!(states[0].state, ScriptState::Current);
        assert!(states[0].path.ends_with("switch.ps1"));

        fs::remove_file(app.switch_script_path(&layout)).unwrap();
        assert_eq!(app.script_states().unwrap()[0].state, ScriptState::Missing);
    }

    #[test]
    fn regenerate_rewrites_the_script_and_clears_the_stale_state() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let layout = saved(app.capture("Desk", None).unwrap());
        fs::remove_file(app.switch_script_path(&layout)).unwrap();

        let fresh = app.regenerate_script(&layout.id).unwrap();
        assert!(app.switch_script_path(&fresh).exists());
        assert_eq!(app.script_states().unwrap()[0].state, ScriptState::Current);
        assert!(fresh.script.unwrap().rendered_at >= layout.script.unwrap().rendered_at);
    }

    #[test]
    fn scripts_from_an_older_template_are_regenerated_on_start() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let a = saved(app.capture("Desk", None).unwrap());
        let b = saved(app.capture("Film", None).unwrap());

        // Age one layout's script record as an older app would have left it.
        let mut config = app.load_config().unwrap();
        config.layouts[0].script = Some(ScriptRecord {
            template_version: TEMPLATE_VERSION - 1,
            rendered_at: "2020-01-01T00:00:00+00:00".into(),
        });
        app.store.save(&config).unwrap();
        assert_eq!(app.script_states().unwrap()[0].state, ScriptState::Stale);

        let regenerated = app.regenerate_stale_scripts().unwrap();
        assert_eq!(regenerated, vec![a.id.clone()]);
        let states = app.script_states().unwrap();
        assert!(states.iter().all(|s| s.state == ScriptState::Current));
        assert_eq!(states[1].layout_id, b.id);

        assert!(app.regenerate_stale_scripts().unwrap().is_empty());
    }

    #[test]
    fn open_script_opens_the_file_and_refuses_a_missing_one() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let layout = saved(app.capture("Desk", None).unwrap());
        app.open_script(&layout.id).unwrap();
        let opened = OPENED.with(|o| o.borrow().clone());
        assert!(opened.contains(&app.switch_script_path(&layout)));

        fs::remove_file(app.switch_script_path(&layout)).unwrap();
        assert!(matches!(
            app.open_script(&layout.id).unwrap_err(),
            AppError::ScriptMissing { .. }
        ));
        assert!(matches!(
            app.open_script("nope").unwrap_err(),
            AppError::LayoutNotFound { .. }
        ));
    }

    #[test]
    fn the_script_names_monitors_by_alias_or_by_name_and_connector_when_shared() {
        let (_dir, app) = app();
        app.probe().unwrap();
        let mut config = app.load_config().unwrap();
        config.aliases.insert(
            r"\\?\DISPLAY#ACR0EC4#5&1a2b3c4d&0&UID8448#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"
                .into(),
            "Side".into(),
        );
        app.store.save(&config).unwrap();
        let layout = saved(app.capture("Desk", None).unwrap());
        let text = fs::read_to_string(app.switch_script_path(&layout)).unwrap();
        assert!(text.contains("    'Side'"), "alias is used");
        assert!(
            text.contains("'MSI MP165 E6 · USB-C DisplayPort 1'"),
            "{text}"
        );
        assert!(text.contains("'MSI MP165 E6 · USB-C DisplayPort 2'"));
        assert!(text.contains("    'Built-in display'"));
    }
}
