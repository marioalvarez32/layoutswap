//! Export and import: the whole config as one file, so a user can back it up and
//! restore it on a reinstall. Export copies the config file byte for byte. Import reads
//! the chosen file through the same parser and migrations the store uses, refuses it
//! without touching the current config when it cannot be read, and otherwise replaces
//! the config, removes the folders of the layouts it replaces, and regenerates every
//! imported layout's script for this machine's app root. The config is saved before
//! the scripts: a script that fails to write is stale in the config and is
//! regenerated on the next start, so the app never points at a half-imported state.
//! The window size stays this machine's when it has one; on a fresh install the
//! file's size is restored with everything else.

use std::fs;
use std::path::PathBuf;

use super::App;
use crate::config::Config;
use crate::error::AppError;
use crate::script::lock;

impl App {
    /// Export: asks `ask` where to save, then copies the config file there as-is. A
    /// config that has never been saved is written first so the copy is complete.
    /// `None` when the user cancelled.
    pub fn export_config(
        &self,
        ask: impl FnOnce(&str) -> Option<PathBuf>,
    ) -> Result<Option<PathBuf>, AppError> {
        if !self.store.path().exists() {
            self.store.save(&self.store.load()?)?;
        }
        let date = chrono::Local::now().format("%Y-%m-%d");
        let Some(target) = ask(&format!("layoutswap-config-{date}.json")) else {
            return Ok(None);
        };
        fs::copy(self.store.path(), &target).map_err(|source| AppError::ExportWrite {
            path: target.clone(),
            source,
        })?;
        self.log(format!("export config: saved to {}", target.display()));
        Ok(Some(target))
    }

    /// Import: asks `ask` which file, reads it, and on success replaces the config,
    /// removes the replaced layouts' folders and regenerates every script. A file this
    /// version cannot read is refused with the problem named, and the current config
    /// stays as it was. Refused while a switch runs, from the app or from a shortcut.
    /// `None` when cancelled.
    pub fn import_config(
        &self,
        ask: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<Option<Config>, AppError> {
        if let Some(layout) = self.switching() {
            return Err(AppError::SwitchRunning { layout });
        }
        if let Some(holder) = lock::read_lock(&self.lock_path()) {
            return Err(AppError::SwitchLocked {
                layout: holder.layout_name(),
                path: self.lock_path(),
            });
        }
        let Some(path) = ask() else {
            return Ok(None);
        };
        let mut imported = self
            .store
            .read(&path)
            .map_err(|error| AppError::ImportRefused {
                path: path.clone(),
                problem: error.import_problem(),
            })?;
        let current = self.store.load()?;
        if self.store.path().exists() {
            imported.window = current.window;
        }
        self.store.save(&imported)?;

        // The replaced layouts' folders go, so no script lingers that the app no longer
        // lists and no imported folder inherits another layout's log. A layout that
        // comes back under its own id keeps its folder and its log.
        for old in &current.layouts {
            let kept = imported.layouts.iter().any(|l| l.id == old.id);
            let folder = self.layouts_dir().join(&old.folder);
            if !kept && folder.is_dir() {
                fs::remove_dir_all(&folder).map_err(|source| AppError::LayoutFolder {
                    path: folder.clone(),
                    source,
                })?;
            }
        }
        let snapshot = imported.clone();
        for layout in &mut imported.layouts {
            self.write_switch_script(layout, &snapshot)?;
        }
        self.store.save(&imported)?;
        self.log(format!(
            "import config: {} layouts from {}",
            imported.layouts.len(),
            path.display()
        ));
        Ok(Some(imported))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::layouts::{CaptureOutcome, Layout, ScriptState};
    use crate::config::WindowSize;
    use crate::script::run::FakeScriptRunner;
    use std::sync::Arc;
    use tempfile::TempDir;

    const FIVE: &str = include_str!("../hardware/fixtures/five-monitors.json");

    fn app_with_layouts(names: &[&str]) -> (TempDir, App, Vec<Layout>) {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        app.probe().unwrap();
        let layouts = names
            .iter()
            .map(|name| match app.capture(name, None).unwrap() {
                CaptureOutcome::Saved { layout } => *layout,
                other => panic!("expected Saved, got {other:?}"),
            })
            .collect();
        (dir, app, layouts)
    }

    #[test]
    fn export_copies_the_config_file_byte_for_byte_under_a_dated_name() {
        let (dir, app, _) = app_with_layouts(&["Desk", "Film"]);
        let target = dir.path().join("backup.json");
        let written = app
            .export_config(|name| {
                assert!(name.starts_with("layoutswap-config-"), "{name}");
                assert!(name.ends_with(".json"));
                assert_eq!(name.len(), "layoutswap-config-2026-09-10.json".len());
                Some(target.clone())
            })
            .unwrap();
        assert_eq!(written, Some(target.clone()));
        assert_eq!(fs::read(&target).unwrap(), fs::read(app.store.path()).unwrap());
        assert_eq!(app.export_config(|_| None).unwrap(), None);
    }

    #[test]
    fn export_of_a_never_saved_config_writes_the_defaults_first() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        assert!(!app.store.path().exists());
        let target = dir.path().join("backup.json");
        app.export_config(|_| Some(target.clone())).unwrap();
        assert!(app.store.path().exists());
        assert_eq!(fs::read(&target).unwrap(), fs::read(app.store.path()).unwrap());
    }

    #[test]
    fn import_replaces_the_config_and_regenerates_every_script_for_this_root() {
        let (source_dir, source, exported) = app_with_layouts(&["Desk", "Film"]);
        let file = source_dir.path().join("backup.json");
        source.export_config(|_| Some(file.clone())).unwrap();

        let (_dir, app, old) = app_with_layouts(&["Old"]);
        app.update_window_size(WindowSize {
            width: 1111,
            height: 777,
        })
        .unwrap();
        let old_folder = app.switch_script_path(&old[0]).parent().unwrap().to_path_buf();
        assert!(old_folder.is_dir());

        let imported = app.import_config(|| Some(file.clone())).unwrap().unwrap();
        assert_eq!(
            imported.layouts.iter().map(|l| l.id.clone()).collect::<Vec<_>>(),
            exported.iter().map(|l| l.id.clone()).collect::<Vec<_>>()
        );
        assert_eq!(imported.window.width, 1111, "the window size stays this machine's");
        let stored = app.load_config().unwrap();
        assert_eq!(stored, imported);
        for layout in &stored.layouts {
            let path = app.switch_script_path(layout);
            assert!(path.exists(), "{}", path.display());
            let text = fs::read_to_string(&path).unwrap();
            assert!(
                text.contains(&format!("$AppRoot       = '{}'", app.root().display())),
                "the script points at this root"
            );
            let exported_record = exported.iter().find(|l| l.id == layout.id).unwrap().script.clone().unwrap();
            assert!(layout.script.clone().unwrap().rendered_at >= exported_record.rendered_at);
        }
        assert!(app
            .script_states()
            .unwrap()
            .iter()
            .all(|s| s.state == ScriptState::Current));
        let text = fs::read_to_string(app.app_log_path()).unwrap();
        assert!(text.contains("import config: 2 layouts from"), "{text}");
        assert!(!old_folder.exists(), "the replaced layout's folder is gone");
    }

    #[test]
    fn import_keeps_the_capabilities_entries_including_monitors_this_machine_never_saw() {
        // The source machine read a monitor this machine never sees.
        let source_dir = tempfile::tempdir().unwrap();
        let runner = Arc::new(FakeScriptRunner::with_stdout(FIVE));
        runner.set_stdout_for(
            crate::app::CAPABILITIES_SCRIPT_NAME,
            r#"{"readAt":"2026-09-11T11:52:00-05:00","monitors":{"path-foreign":{"answered":true,"raw":"(prot(monitor)vcp(60(11 0F) D6(01)))","modes":[]}}}"#,
        );
        let source = App::new(source_dir.path().join("layoutswap"), runner as _);
        source.probe().unwrap();
        source.capture("Desk", None).unwrap();
        let stored = source.read_capabilities().unwrap();
        let entry = stored.get("path-foreign").cloned().expect("the foreign entry is stored");
        let file = source_dir.path().join("backup.json");
        source.export_config(|_| Some(file.clone())).unwrap();

        let (_dir, app, _) = app_with_layouts(&["Old"]);
        let imported = app.import_config(|| Some(file.clone())).unwrap().unwrap();
        assert_eq!(imported.capabilities.get("path-foreign"), Some(&entry));
        assert_eq!(app.load_config().unwrap().capabilities.get("path-foreign"), Some(&entry));
    }

    #[test]
    fn a_layout_that_comes_back_under_its_own_id_keeps_its_folder_and_log() {
        let (dir, app, layouts) = app_with_layouts(&["Desk"]);
        let file = dir.path().join("backup.json");
        app.export_config(|_| Some(file.clone())).unwrap();
        let log = app.switch_log_path(&layouts[0]);
        fs::write(&log, "2026-09-10 18:00:00  === Switch to Desk ===\n").unwrap();
        let imported = app.import_config(|| Some(file.clone())).unwrap().unwrap();
        assert_eq!(imported.layouts[0].id, layouts[0].id);
        assert!(log.exists(), "the log of a layout that came back is kept");
    }

    #[test]
    fn a_fresh_install_takes_the_window_size_from_the_file() {
        let (source_dir, source, _) = app_with_layouts(&["Desk"]);
        source
            .update_window_size(WindowSize {
                width: 1500,
                height: 900,
            })
            .unwrap();
        let file = source_dir.path().join("backup.json");
        source.export_config(|_| Some(file.clone())).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            dir.path().join("layoutswap"),
            Arc::new(FakeScriptRunner::with_stdout(FIVE)),
        );
        assert!(!app.store.path().exists());
        let imported = app.import_config(|| Some(file.clone())).unwrap().unwrap();
        assert_eq!(imported.window.width, 1500);
        assert_eq!(app.load_config().unwrap().window.width, 1500);
    }

    #[test]
    fn a_layout_from_another_machine_opens_with_every_monitor_absent_and_switch_names_them() {
        let (source_dir, source, _) = app_with_layouts(&["Desk"]);
        let file = source_dir.path().join("backup.json");
        source.export_config(|_| Some(file.clone())).unwrap();
        // Every device path in the file belongs to monitors this machine never sees.
        let foreign = fs::read_to_string(&file)
            .unwrap()
            .replace("DISPLAY#", "DISPLAY#FOREIGN-");
        fs::write(&file, foreign).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let runner = Arc::new(FakeScriptRunner::with_stdout(FIVE).streaming(
            &[
                "[1/3] running Check monitors",
                "[1/3] failed Check monitors: press the input button on KG241Y X1, or plug it in, then switch again; Absent: KG241Y X1",
                "Exit code 2",
            ],
            2,
        ));
        let app = App::new(dir.path().join("layoutswap"), Arc::clone(&runner) as _);
        app.probe().unwrap();
        let imported = app.import_config(|| Some(file.clone())).unwrap().unwrap();
        let layout = &imported.layouts[0];
        assert!(app.switch_script_path(layout).exists(), "an all-Absent layout is still imported");

        let events = std::sync::Mutex::new(Vec::new());
        let result = app
            .switch(&layout.id, &|event| events.lock().unwrap().push(event))
            .unwrap();
        match result {
            crate::app::SwitchResult::Failed {
                step, explanation, ..
            } => {
                assert_eq!(step, Some(1));
                let crate::app::FailureExplanation::Absent { monitors } = explanation else {
                    panic!("expected the Absent list");
                };
                // The probe lists none of them, so every on monitor is Absent.
                assert_eq!(monitors.len(), 4);
                assert!(monitors.contains(&"KG241Y X1".to_string()));
                assert!(monitors.contains(&"Built-in display".to_string()));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn import_is_refused_while_a_switch_runs_or_a_lock_is_held() {
        let (dir, app, _) = app_with_layouts(&["Desk"]);
        fs::write(app.lock_path(), "pid=1\r\nlayout=Film\r\n").unwrap();
        let error = app.import_config(|| Some(dir.path().join("x.json"))).unwrap_err();
        assert!(matches!(error, AppError::SwitchLocked { ref layout, .. } if layout == "Film"));
    }

    #[test]
    fn import_refuses_a_bad_file_by_name_and_leaves_the_config_alone() {
        let (dir, app, _) = app_with_layouts(&["Desk"]);
        let before = fs::read(app.store.path()).unwrap();

        let newer = dir.path().join("newer.json");
        fs::write(&newer, r#"{"schemaVersion": 99, "window": {"width": 1, "height": 1}}"#).unwrap();
        let error = app.import_config(|| Some(newer.clone())).unwrap_err();
        match &error {
            AppError::ImportRefused { path, problem } => {
                assert_eq!(path, &newer);
                assert!(problem.contains("schema version 99"), "{problem}");
            }
            other => panic!("expected ImportRefused, got {other:?}"),
        }
        assert!(error.to_string().starts_with("Pick another file to import"), "{error}");
        assert!(error.to_string().contains("newer.json"), "{error}");

        let garbage = dir.path().join("garbage.json");
        fs::write(&garbage, "not json").unwrap();
        assert!(matches!(
            app.import_config(|| Some(garbage.clone())).unwrap_err(),
            AppError::ImportRefused { .. }
        ));

        let wrong_shape = dir.path().join("shape.json");
        fs::write(&wrong_shape, r#"{"schemaVersion": 1, "window": "wide"}"#).unwrap();
        let error = app.import_config(|| Some(wrong_shape.clone())).unwrap_err();
        assert!(error.to_string().contains("not a layoutswap config"), "{error}");

        let missing = dir.path().join("missing.json");
        assert!(matches!(
            app.import_config(|| Some(missing)).unwrap_err(),
            AppError::ImportRefused { .. }
        ));

        assert_eq!(fs::read(app.store.path()).unwrap(), before, "the config is untouched");
        assert_eq!(app.import_config(|| None).unwrap(), None);
    }
}
