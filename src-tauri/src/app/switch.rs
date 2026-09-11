//! Switch: running a layout's generated script from the app with live progress.
//!
//! One switch at a time. The app refuses while it runs one itself or while the lock
//! file says a shortcut launch holds one. The script's output streams through the
//! [`ScriptRunner`] seam: a parsed progress line becomes a progress event, every other
//! line a log event, and the command resolves once the script exits. Cancel kills the
//! script until the Apply arrangement step reports running; after that the arrangement
//! is changing under Windows and the switch has to finish.

use std::time::Instant;

use serde::Serialize;
use ts_rs::TS;

use super::App;
use crate::config::layouts::{Layout, SWITCH_LOG_NAME};
use crate::error::AppError;
use crate::script::lock::{self, LOCK_FILE_NAME};
use crate::script::progress::{parse_progress_line, ProgressLine, StepStatus};
use crate::script::render::{APPLY_STEP, SWITCH_STEPS};
use crate::script::run::CancelHandle;

/// What the app tells the webview while a switch runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum SwitchEvent {
    /// The script is running: the checks passed and the progress screen can open.
    Started {
        layout_id: String,
        /// The step names in order, so the screen lists them before any reports.
        steps: Vec<String>,
        /// The one-based step from which Cancel is disabled.
        apply_step: u32,
    },
    Progress {
        line: ProgressLine,
    },
    Log {
        text: String,
    },
}

/// How a switch ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "outcome", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum SwitchResult {
    /// Verify passed. `duration_ms` is the time from launch to exit.
    Applied {
        duration_ms: u32,
    },
    /// The script stopped short. The next action comes first, as the script printed it.
    Failed {
        /// The one-based step that failed, or null when the script stopped before or
        /// outside a step.
        step: Option<u32>,
        step_name: String,
        next_action: String,
        reason: String,
        exit_code: i32,
        /// `switch.log` in the layout folder.
        log_path: String,
    },
    Cancelled,
}

/// The switch the app is running right now.
pub(super) struct ActiveSwitch {
    layout_name: String,
    cancel: CancelHandle,
    /// Set once the Apply arrangement step reports, after which Cancel is refused.
    apply_started: bool,
    cancelled: bool,
}

impl App {
    pub fn lock_path(&self) -> std::path::PathBuf {
        self.root().join(LOCK_FILE_NAME)
    }

    /// Runs `layout_id`'s switch script, sending every event to `on_event`, and returns
    /// once it has exited. Refused while another switch runs, from the app or from a
    /// shortcut, or when the script is not on disk.
    pub fn switch(
        &self,
        layout_id: &str,
        on_event: &dyn Fn(SwitchEvent),
    ) -> Result<SwitchResult, AppError> {
        let config = self.store.load()?;
        let layout = config
            .layouts
            .iter()
            .find(|l| l.id == layout_id)
            .ok_or_else(|| AppError::LayoutNotFound {
                id: layout_id.to_string(),
            })?;
        let script = self.switch_script_path(layout);
        if !script.exists() {
            return Err(AppError::ScriptMissing { path: script });
        }

        let mut running = {
            // Checked and started under the one lock, so two presses cannot both start.
            let mut active = self.active_switch();
            if let Some(active) = active.as_ref() {
                return Err(AppError::SwitchRunning {
                    layout: active.layout_name.clone(),
                });
            }
            if let Some(holder) = lock::read_lock(&self.lock_path()) {
                return Err(AppError::SwitchLocked {
                    layout: holder.layout_name(),
                    path: self.lock_path(),
                });
            }
            let running = self.runner.start(&script, &[])?;
            *active = Some(ActiveSwitch {
                layout_name: layout.name.clone(),
                cancel: running.cancel_handle(),
                apply_started: false,
                cancelled: false,
            });
            running
        };
        let started = Instant::now();
        on_event(SwitchEvent::Started {
            layout_id: layout.id.clone(),
            steps: SWITCH_STEPS.iter().map(|s| s.to_string()).collect(),
            apply_step: APPLY_STEP,
        });

        let mut last_failed: Option<ProgressLine> = None;
        let mut last_log = String::new();
        while let Some(text) = running.next_line() {
            match parse_progress_line(&text) {
                Some(line) => {
                    if line.step >= APPLY_STEP {
                        if let Some(active) = self.active_switch().as_mut() {
                            active.apply_started = true;
                        }
                    }
                    if line.status == StepStatus::Failed {
                        last_failed = Some(line.clone());
                    }
                    on_event(SwitchEvent::Progress { line });
                }
                None => {
                    if !text.trim().is_empty() && !text.starts_with("Exit code ") {
                        last_log = text.clone();
                    }
                    on_event(SwitchEvent::Log { text });
                }
            }
        }
        let exit_code = running.wait();
        let duration_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let cancelled = self
            .active_switch()
            .take()
            .is_some_and(|active| active.cancelled);
        // A killed script never reaches its own cleanup.
        lock::clear_lock_of(&self.lock_path(), &layout.name);

        // A kill that lands after the script exited with 0 changed nothing: the
        // arrangement applied and verified, and the result says so.
        if exit_code == 0 {
            return Ok(SwitchResult::Applied { duration_ms });
        }
        if cancelled {
            return Ok(SwitchResult::Cancelled);
        }
        Ok(self.failure(layout, exit_code, last_failed, &last_log))
    }

    /// Kills the running switch. Refused once the Apply arrangement step has reported.
    pub fn cancel_switch(&self) -> Result<(), AppError> {
        let cancel = {
            let mut guard = self.active_switch();
            let active = guard.as_mut().ok_or(AppError::NoSwitchRunning)?;
            if active.apply_started {
                return Err(AppError::CancelTooLate {
                    layout: active.layout_name.clone(),
                });
            }
            active.cancelled = true;
            std::sync::Arc::clone(&active.cancel)
        };
        // Outside the lock: the kill waits on the script's process handle, which the
        // switch thread holds while it waits for the exit.
        cancel();
        Ok(())
    }

    /// The layout a switch is running for, when one is.
    pub fn switching(&self) -> Option<String> {
        self.active_switch()
            .as_ref()
            .map(|active| active.layout_name.clone())
    }

    fn active_switch(&self) -> std::sync::MutexGuard<'_, Option<ActiveSwitch>> {
        self.active_switch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The failed result: from the failed progress line when the script printed one,
    /// else from the last log line, which is where the script explains a refusal.
    fn failure(
        &self,
        layout: &Layout,
        exit_code: i32,
        last_failed: Option<ProgressLine>,
        last_log: &str,
    ) -> SwitchResult {
        let log_path = self
            .switch_script_path(layout)
            .with_file_name(SWITCH_LOG_NAME)
            .display()
            .to_string();
        match last_failed {
            Some(line) => {
                let parts = line.failure_parts();
                SwitchResult::Failed {
                    step: Some(line.step),
                    step_name: parts.step_name,
                    next_action: parts.next_action,
                    reason: parts.reason,
                    exit_code,
                    log_path,
                }
            }
            None => SwitchResult::Failed {
                step: None,
                step_name: String::new(),
                next_action: if last_log.is_empty() {
                    "Open the log, then try the switch again".to_string()
                } else {
                    last_log.to_string()
                },
                reason: format!("the script exited with exit code {exit_code}"),
                exit_code,
                log_path,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::layouts::CaptureOutcome;
    use crate::script::run::FakeScriptRunner;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use tempfile::TempDir;

    const FIVE: &str = include_str!("../hardware/fixtures/five-monitors.json");

    const APPLIED: &[&str] = &[
        "",
        "=== Switch to Desk ===",
        "[1/3] running Check monitors",
        "  Side: Available",
        "[1/3] done Check monitors",
        "[2/3] running Apply arrangement",
        "  validated",
        "  applied",
        "[2/3] done Apply arrangement",
        "[3/3] running Verify",
        "[3/3] done Verify",
        "Switched to Desk.",
        "Exit code 0",
    ];

    const ABSENT: &[&str] = &[
        "=== Switch to Desk ===",
        "[1/3] running Check monitors",
        "  Ultrawide: Absent",
        "[1/3] failed Check monitors: press the input button on Ultrawide, or plug it in, then switch again; 1 Absent",
        "Exit code 2",
    ];

    fn app_with(runner: FakeScriptRunner) -> (TempDir, Arc<App>, Layout) {
        let (dir, app, layout, _) = app_and_runner(runner);
        (dir, app, layout)
    }

    fn app_and_runner(
        runner: FakeScriptRunner,
    ) -> (TempDir, Arc<App>, Layout, Arc<FakeScriptRunner>) {
        let dir = tempfile::tempdir().unwrap();
        let runner = Arc::new(runner);
        let app = App::new(dir.path().join("layoutswap"), Arc::clone(&runner) as _);
        app.probe().unwrap();
        let layout = match app.capture("Desk", None).unwrap() {
            CaptureOutcome::Saved { layout } => *layout,
            other => panic!("expected Saved, got {other:?}"),
        };
        (dir, Arc::new(app), layout, runner)
    }

    fn collect(app: &App, layout_id: &str) -> (Result<SwitchResult, AppError>, Vec<SwitchEvent>) {
        let events = Mutex::new(Vec::new());
        let result = app.switch(layout_id, &|event| events.lock().unwrap().push(event));
        (result, events.into_inner().unwrap())
    }

    fn progress_lines(events: &[SwitchEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|e| match e {
                SwitchEvent::Progress { line } => Some(format!(
                    "{}/{} {:?} {}",
                    line.step, line.of, line.status, line.text
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_applied_switch_resolves_with_its_duration_and_the_events_in_order() {
        let (_dir, app, layout) =
            app_with(FakeScriptRunner::with_stdout(FIVE).streaming(APPLIED, 0));
        let (result, events) = collect(&app, &layout.id);
        match result.unwrap() {
            SwitchResult::Applied { duration_ms } => assert!(duration_ms < 10_000),
            other => panic!("expected Applied, got {other:?}"),
        }
        assert_eq!(
            events[0],
            SwitchEvent::Started {
                layout_id: layout.id.clone(),
                steps: vec![
                    "Check monitors".into(),
                    "Apply arrangement".into(),
                    "Verify".into()
                ],
                apply_step: 2,
            }
        );
        assert_eq!(
            progress_lines(&events),
            vec![
                "1/3 Running Check monitors",
                "1/3 Done Check monitors",
                "2/3 Running Apply arrangement",
                "2/3 Done Apply arrangement",
                "3/3 Running Verify",
                "3/3 Done Verify",
            ]
        );
        let logs: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                SwitchEvent::Log { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(logs.len(), APPLIED.len() - 6);
        assert!(logs.contains(&"  Side: Available"));
        assert!(logs.contains(&"Switched to Desk."));
        assert_eq!(app.switching(), None);
        assert!(!app.lock_path().exists());
    }

    #[test]
    fn the_layout_script_runs_through_the_runner_with_no_arguments() {
        let (_dir, app, layout, runner) =
            app_and_runner(FakeScriptRunner::with_stdout(FIVE).streaming(APPLIED, 0));
        collect(&app, &layout.id).0.unwrap();
        let calls = runner.calls.lock().unwrap();
        // The probe came first; the switch is the last call.
        let (script, args) = calls.last().unwrap();
        assert_eq!(script, &app.switch_script_path(&layout));
        assert!(args.is_empty());
    }

    #[test]
    fn a_failed_step_resolves_with_the_step_and_the_next_action() {
        let (_dir, app, layout) =
            app_with(FakeScriptRunner::with_stdout(FIVE).streaming(ABSENT, 2));
        let (result, events) = collect(&app, &layout.id);
        match result.unwrap() {
            SwitchResult::Failed {
                step,
                step_name,
                next_action,
                reason,
                exit_code,
                log_path,
            } => {
                assert_eq!(step, Some(1));
                assert_eq!(step_name, "Check monitors");
                assert_eq!(
                    next_action,
                    "press the input button on Ultrawide, or plug it in, then switch again"
                );
                assert_eq!(reason, "1 Absent");
                assert_eq!(exit_code, 2);
                assert!(log_path.ends_with("switch.log"), "{log_path}");
                assert!(log_path.contains("desk"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        let failed = events.iter().find_map(|e| match e {
            SwitchEvent::Progress { line } if line.status == StepStatus::Failed => Some(line),
            _ => None,
        });
        assert!(failed.is_some());
    }

    #[test]
    fn a_script_that_stops_outside_a_step_explains_itself_from_its_last_line() {
        let refused = &[
            "=== Switch to Desk ===",
            "Wait for the switch to Film to finish, then try again. If process 7 is no longer running, delete C:\\x\\switch.lock first.",
            "Exit code 2",
        ];
        let (_dir, app, layout) =
            app_with(FakeScriptRunner::with_stdout(FIVE).streaming(refused, 2));
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed {
                step,
                next_action,
                reason,
                ..
            } => {
                assert_eq!(step, None);
                assert!(next_action.starts_with("Wait for the switch to Film"));
                assert_eq!(reason, "the script exited with exit code 2");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_lock_from_a_shortcut_launch_refuses_the_switch_and_names_the_layout() {
        let (_dir, app, layout) =
            app_with(FakeScriptRunner::with_stdout(FIVE).streaming(APPLIED, 0));
        fs::write(app.lock_path(), "pid=4242\r\nlayout=Film\r\n").unwrap();
        let error = collect(&app, &layout.id).0.unwrap_err();
        match &error {
            AppError::SwitchLocked { layout, path } => {
                assert_eq!(layout, "Film");
                assert_eq!(path, &app.lock_path());
            }
            other => panic!("expected SwitchLocked, got {other:?}"),
        }
        assert_eq!(
            error.to_string(),
            format!(
                "Wait for the switch to Film to finish, then try again. If no switch is running, delete {} first.",
                app.lock_path().display()
            )
        );
        assert!(app.lock_path().exists(), "another run's lock is left alone");
    }

    #[test]
    fn a_second_switch_while_one_runs_is_refused_by_name() {
        let (runner, gate) = FakeScriptRunner::with_stdout(FIVE)
            .streaming(APPLIED, 0)
            .pausing_after(3);
        let (_dir, app, layout) = app_with(runner);
        let first = {
            let app = Arc::clone(&app);
            let id = layout.id.clone();
            thread::spawn(move || collect(&app, &id).0)
        };
        wait_until(|| app.switching().is_some());
        let error = collect(&app, &layout.id).0.unwrap_err();
        assert!(matches!(error, AppError::SwitchRunning { ref layout } if layout == "Desk"));
        gate.release();
        assert!(matches!(
            first.join().unwrap().unwrap(),
            SwitchResult::Applied { .. }
        ));
    }

    #[test]
    fn cancel_before_the_apply_kills_the_script_and_resolves_cancelled() {
        // Paused after "[1/3] done Check monitors": the apply has not reported yet.
        let (runner, _gate) = FakeScriptRunner::with_stdout(FIVE)
            .streaming(APPLIED, 0)
            .pausing_after(5);
        let (_dir, app, layout) = app_with(runner);
        let run = {
            let app = Arc::clone(&app);
            let id = layout.id.clone();
            thread::spawn(move || collect(&app, &id))
        };
        wait_until(|| app.switching().is_some());
        // The lock the real script would have written by now, and cannot remove once killed.
        fs::write(app.lock_path(), "pid=1\r\nlayout=Desk\r\n").unwrap();
        app.cancel_switch().unwrap();
        let (result, events) = run.join().unwrap();
        assert_eq!(result.unwrap(), SwitchResult::Cancelled);
        assert_eq!(
            progress_lines(&events),
            vec!["1/3 Running Check monitors", "1/3 Done Check monitors"]
        );
        assert_eq!(app.switching(), None);
        assert!(!app.lock_path().exists(), "the killed script's lock is cleared");
        assert!(matches!(app.cancel_switch(), Err(AppError::NoSwitchRunning)));
    }

    #[test]
    fn cancel_after_the_apply_has_reported_is_refused_and_the_switch_finishes() {
        // Paused after "[2/3] running Apply arrangement".
        let (runner, gate) = FakeScriptRunner::with_stdout(FIVE)
            .streaming(APPLIED, 0)
            .pausing_after(6);
        let (_dir, app, layout) = app_with(runner);
        let run = {
            let app = Arc::clone(&app);
            let id = layout.id.clone();
            thread::spawn(move || collect(&app, &id))
        };
        wait_until(|| app.switching().is_some());
        wait_until(|| app.active_switch().as_ref().is_some_and(|a| a.apply_started));
        let error = app.cancel_switch().unwrap_err();
        assert!(matches!(error, AppError::CancelTooLate { ref layout } if layout == "Desk"));
        assert_eq!(
            error.to_string(),
            "Wait for the switch to Desk to finish: the arrangement is already being applied and cannot be stopped."
        );
        gate.release();
        let (result, _) = run.join().unwrap();
        assert!(matches!(result.unwrap(), SwitchResult::Applied { .. }));
    }

    #[test]
    fn a_missing_script_is_refused_before_anything_runs() {
        let (_dir, app, layout) =
            app_with(FakeScriptRunner::with_stdout(FIVE).streaming(APPLIED, 0));
        fs::remove_file(app.switch_script_path(&layout)).unwrap();
        assert!(matches!(
            collect(&app, &layout.id).0.unwrap_err(),
            AppError::ScriptMissing { .. }
        ));
        assert!(matches!(
            collect(&app, "nope").0.unwrap_err(),
            AppError::LayoutNotFound { .. }
        ));
    }

    fn wait_until(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !condition() {
            assert!(Instant::now() < deadline, "timed out waiting");
            thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
