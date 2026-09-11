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
use crate::config::layouts::verify::{verify, VerifyFailure};
use crate::config::layouts::{self, Layout};
use crate::config::Config;
use crate::error::AppError;
use crate::hardware::MonitorState;
use crate::script::lock::{self, LOCK_FILE_NAME};
use crate::script::progress::{parse_progress_line, ProgressLine, StepStatus};
use crate::script::render::{self, ABSENT_REASON_PREFIX, CHECK_STEP, EXTENDED_REASON_PREFIX};
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
        explanation: FailureExplanation,
    },
    /// The user cancelled. `sent` names the send rows that had already run, or were
    /// running when the script was killed, since their monitor may be showing another
    /// device now.
    Cancelled {
        sent: Vec<String>,
    },
}

/// What a fresh probe says about a failed step, so the result screen can name
/// monitors and positions instead of quoting the script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum FailureExplanation {
    /// Check monitors: these on monitors are Absent.
    Absent { monitors: Vec<String> },
    /// Verify: the arrangement is not what the layout says. Both lists are empty when
    /// the probe could not run or found the arrangement settled after the script left.
    Verify {
        failures: Vec<VerifyFailure>,
        warnings: Vec<String>,
    },
    /// The apply failed and the Extend fallback landed: every monitor shows the desktop,
    /// but not in this layout's arrangement.
    Extended,
    /// Nothing beyond the script's own line: another step failed.
    None,
}

/// The switch the app is running right now.
pub(super) struct ActiveSwitch {
    layout_id: String,
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
                layout_id: layout.id.clone(),
                layout_name: layout.name.clone(),
                cancel: running.cancel_handle(),
                apply_started: false,
                cancelled: false,
            });
            running
        };
        let started = Instant::now();
        let timeline = render::timeline(layout, &config.aliases);
        self.log(format!("switch {}: started", layout.name));
        on_event(SwitchEvent::Started {
            layout_id: layout.id.clone(),
            steps: timeline.rows.clone(),
            apply_step: timeline.apply,
        });

        let mut last_failed: Option<ProgressLine> = None;
        let mut last_log = String::new();
        let mut sent: Vec<String> = Vec::new();
        while let Some(text) = running.next_line() {
            match parse_progress_line(&text) {
                Some(line) => {
                    if line.step >= timeline.apply {
                        if let Some(active) = self.active_switch().as_mut() {
                            active.apply_started = true;
                        }
                    }
                    if line.status == StepStatus::Failed {
                        last_failed = Some(line.clone());
                    }
                    if timeline.sends.contains(&line.step)
                        && matches!(line.status, StepStatus::Running | StepStatus::Done)
                    {
                        let row = timeline.rows[line.step as usize - 1].clone();
                        if !sent.contains(&row) {
                            sent.push(row);
                        }
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
            self.log(format!(
                "switch {}: applied in {:.1} s",
                layout.name,
                f64::from(duration_ms) / 1000.0
            ));
            return Ok(SwitchResult::Applied { duration_ms });
        }
        if cancelled {
            self.log(format!(
                "switch {}: cancelled{}",
                layout.name,
                if sent.is_empty() {
                    String::new()
                } else {
                    format!(" after: {}", sent.join("; "))
                }
            ));
            return Ok(SwitchResult::Cancelled { sent });
        }
        let result = self.failure(layout, &config, &timeline, exit_code, last_failed, &last_log);
        if let SwitchResult::Failed {
            step_name, reason, ..
        } = &result
        {
            self.log(format!(
                "switch {}: failed at {}: {reason} (exit code {exit_code})",
                layout.name,
                if step_name.is_empty() { "start" } else { step_name }
            ));
        }
        Ok(result)
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

    /// The id of the layout a switch is running for, when one is.
    pub fn switching_layout_id(&self) -> Option<String> {
        self.active_switch()
            .as_ref()
            .map(|active| active.layout_id.clone())
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
        config: &Config,
        timeline: &render::Timeline,
        exit_code: i32,
        last_failed: Option<ProgressLine>,
        last_log: &str,
    ) -> SwitchResult {
        let log_path = self.switch_log_path(layout).display().to_string();
        match last_failed {
            Some(line) => {
                let explanation = self.explain(layout, config, timeline, &line);
                let parts = line.parts.clone().unwrap_or_default();
                SwitchResult::Failed {
                    step: Some(line.step),
                    step_name: parts.name,
                    next_action: parts.action,
                    reason: parts.detail,
                    exit_code,
                    log_path,
                    explanation,
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
                explanation: FailureExplanation::None,
            },
        }
    }

    /// A fresh probe explains the check and verify steps: which on monitors are Absent,
    /// or where the arrangement landed. Other steps explain nothing beyond the script's
    /// line. When the probe cannot run or disagrees with the script (the monitor came
    /// back in between), the check step's names come from the script's own reason.
    fn explain(
        &self,
        layout: &Layout,
        config: &Config,
        timeline: &render::Timeline,
        line: &ProgressLine,
    ) -> FailureExplanation {
        let label = layouts::labeller(&config.aliases, &layout.summary.monitors);
        let verify_step = timeline.verify();
        let reason = line.parts.as_ref().map(|p| p.detail.as_str()).unwrap_or("");
        if line.step == timeline.apply && reason.starts_with(EXTENDED_REASON_PREFIX) {
            return FailureExplanation::Extended;
        }
        match line.step {
            CHECK_STEP => {
                let from_probe: Vec<String> = self
                    .probe()
                    .map(|inventory| {
                        layout
                            .summary
                            .monitors
                            .iter()
                            .filter(|m| m.on)
                            .filter(|m| {
                                let live = inventory
                                    .monitors
                                    .iter()
                                    .find(|l| l.device_path.eq_ignore_ascii_case(&m.device_path));
                                live.map_or(true, |l| l.state == MonitorState::Absent)
                            })
                            .map(&label)
                            .collect()
                    })
                    .unwrap_or_default();
                let monitors = if from_probe.is_empty() {
                    absent_from_reason(&line.parts.clone().unwrap_or_default().detail)
                } else {
                    from_probe
                };
                FailureExplanation::Absent { monitors }
            }
            step if step == verify_step => match self.probe() {
                Ok(inventory) => {
                    let outcome = verify(&layout.summary, &inventory, label);
                    FailureExplanation::Verify {
                        failures: outcome.failures,
                        warnings: outcome.warnings,
                    }
                }
                Err(_) => FailureExplanation::Verify {
                    failures: vec![],
                    warnings: vec![],
                },
            },
            _ => FailureExplanation::None,
        }
    }
}

/// The labels the check step named, from its reason line.
fn absent_from_reason(reason: &str) -> Vec<String> {
    reason
        .strip_prefix(ABSENT_REASON_PREFIX)
        .map(|names| names.split(", ").map(str::to_string).collect())
        .unwrap_or_default()
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
        "[1/3] failed Check monitors: press the input button on Ultrawide, or plug it in, then switch again; Absent: Ultrawide",
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
                explanation,
            } => {
                assert_eq!(step, Some(1));
                assert_eq!(step_name, "Check monitors");
                assert_eq!(
                    next_action,
                    "press the input button on Ultrawide, or plug it in, then switch again"
                );
                assert_eq!(reason, "Absent: Ultrawide");
                assert_eq!(exit_code, 2);
                assert!(log_path.ends_with("switch.log"), "{log_path}");
                assert!(log_path.contains("desk"));
                // The probe still shows every on monitor present, so the names come from
                // the script's own line.
                assert_eq!(
                    explanation,
                    FailureExplanation::Absent {
                        monitors: vec!["Ultrawide".into()]
                    }
                );
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
    fn a_check_failure_names_the_absent_monitors_from_a_fresh_probe() {
        let (_dir, app, layout, runner) =
            app_and_runner(FakeScriptRunner::with_stdout(FIVE).streaming(ABSENT, 2));
        // The Acer went Absent after the capture.
        let absent = FIVE.replacen(
            r#""active":true,"available":true,"hasMode":true,"x":0,"y":0,"width":1920,"height":1080"#,
            r#""active":false,"available":false,"hasMode":false,"x":0,"y":0,"width":0,"height":0"#,
            1,
        );
        assert_ne!(absent, FIVE, "the fixture line the test bends must exist");
        runner.set_stdout(&absent);
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed {
                step, explanation, ..
            } => {
                assert_eq!(step, Some(1));
                assert_eq!(
                    explanation,
                    FailureExplanation::Absent {
                        monitors: vec!["KG241Y X1".into()]
                    }
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        let text = fs::read_to_string(app.app_log_path()).unwrap();
        assert!(text.contains("switch Desk: started"), "{text}");
        assert!(
            text.contains("switch Desk: failed at Check monitors: Absent: Ultrawide (exit code 2)"),
            "{text}"
        );
    }

    #[test]
    fn a_verify_failure_states_where_the_monitor_landed_and_where_it_belongs() {
        let landed = &[
            "[1/3] running Check monitors",
            "[1/3] done Check monitors",
            "[2/3] running Apply arrangement",
            "[2/3] done Apply arrangement",
            "[3/3] running Verify",
            "  warning: KG241Y X1 runs at 75 Hz instead of 60 Hz",
            "[3/3] failed Verify: arrange the monitors in Windows Settings > Display, then save the layout again; KG241Y X1 landed at 1920,0 instead of 0,0",
            "Exit code 1",
        ];
        let (_dir, app, layout, runner) =
            app_and_runner(FakeScriptRunner::with_stdout(FIVE).streaming(landed, 1));
        let moved = FIVE.replacen(
            r#""hasMode":true,"x":0,"y":0,"width":1920,"height":1080,"refreshNum":60000"#,
            r#""hasMode":true,"x":1920,"y":0,"width":1920,"height":1080,"refreshNum":75000"#,
            1,
        );
        assert_ne!(moved, FIVE, "the fixture line the test bends must exist");
        runner.set_stdout(&moved);
        let (result, events) = collect(&app, &layout.id);
        match result.unwrap() {
            SwitchResult::Failed {
                step,
                next_action,
                explanation,
                ..
            } => {
                assert_eq!(step, Some(3));
                assert!(next_action.starts_with("arrange the monitors in Windows Settings > Display"));
                assert_eq!(
                    explanation,
                    FailureExplanation::Verify {
                        failures: vec![VerifyFailure::Misplaced {
                            label: "KG241Y X1".into(),
                            actual: crate::hardware::Point { x: 1920, y: 0 },
                            expected: crate::hardware::Point { x: 0, y: 0 },
                        }],
                        warnings: vec!["KG241Y X1 runs at 75 Hz instead of 60 Hz".into()],
                    }
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        // The script's warning line reached the log tail, not a failure.
        assert!(events.iter().any(|e| matches!(
            e,
            SwitchEvent::Log { text } if text.contains("warning: KG241Y X1 runs at 75 Hz")
        )));
    }

    #[test]
    fn a_verify_failure_whose_probe_cannot_run_still_reads_as_a_verify_failure() {
        let landed = &[
            "[3/3] running Verify",
            "[3/3] failed Verify: arrange the monitors in Windows Settings > Display, then save the layout again; KG241Y X1 landed at 1920,0 instead of 0,0",
            "Exit code 1",
        ];
        let (_dir, app, layout, runner) =
            app_and_runner(FakeScriptRunner::with_stdout(FIVE).streaming(landed, 1));
        runner.set_stdout("not json");
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed { explanation, .. } => assert_eq!(
                explanation,
                FailureExplanation::Verify {
                    failures: vec![],
                    warnings: vec![]
                }
            ),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_timeline_with_wait_steps_reaches_the_screen_in_order_and_applies() {
        use crate::config::layouts::{ApplyFailure, LayoutEdits, Step, StepKind, StepSide};
        let lines = &[
            "[1/5] running Check monitors",
            "[1/5] done Check monitors",
            "[2/5] running Wait 3 seconds",
            "[2/5] done Wait 3 seconds",
            "[3/5] running Apply arrangement",
            "[3/5] done Apply arrangement",
            "[4/5] running Wait 1 second",
            "[4/5] done Wait 1 second",
            "[5/5] running Verify",
            "[5/5] done Verify",
            "Exit code 0",
        ];
        let (_dir, app, layout) =
            app_with(FakeScriptRunner::with_stdout(FIVE).streaming(lines, 0));
        let wait = |id: &str, side, seconds| Step {
            id: id.into(),
            side,
            kind: StepKind::Wait { seconds },
        };
        app.save_layout(
            &layout.id,
            LayoutEdits {
                steps: vec![wait("a", StepSide::Before, 3), wait("b", StepSide::After, 1)],
                drop_wait_seconds: 5,
                available_wait_seconds: 120,
                on_apply_failure: ApplyFailure::Stop,
            },
        )
        .unwrap();
        let (result, events) = collect(&app, &layout.id);
        assert!(matches!(result.unwrap(), SwitchResult::Applied { .. }));
        assert_eq!(
            events[0],
            SwitchEvent::Started {
                layout_id: layout.id.clone(),
                steps: vec![
                    "Check monitors".into(),
                    "Wait 3 seconds".into(),
                    "Apply arrangement".into(),
                    "Wait 1 second".into(),
                    "Verify".into()
                ],
                apply_step: 3,
            }
        );
        assert_eq!(
            progress_lines(&events),
            vec![
                "1/5 Running Check monitors",
                "1/5 Done Check monitors",
                "2/5 Running Wait 3 seconds",
                "2/5 Done Wait 3 seconds",
                "3/5 Running Apply arrangement",
                "3/5 Done Apply arrangement",
                "4/5 Running Wait 1 second",
                "4/5 Done Wait 1 second",
                "5/5 Running Verify",
                "5/5 Done Verify",
            ]
        );
    }

    /// A layout whose one step sends the Acer to HDMI 1 before the apply and waits for it to drop.
    fn send_edits() -> crate::config::layouts::LayoutEdits {
        use crate::config::layouts::{ApplyFailure, LayoutEdits, Step, StepKind, StepSide, WaitRule};
        let acer = crate::hardware::parse(FIVE)
            .unwrap()
            .monitors
            .iter()
            .find(|m| m.device_path.contains("ACR0EC4"))
            .unwrap()
            .device_path
            .clone();
        LayoutEdits {
            steps: vec![Step {
                id: "s".into(),
                side: StepSide::Before,
                kind: StepKind::SendInput {
                    device_path: acer,
                    input_source: 0x11,
                    wait: WaitRule::Drop,
                },
            }],
            drop_wait_seconds: 5,
            available_wait_seconds: 120,
            on_apply_failure: ApplyFailure::Stop,
        }
    }

    #[test]
    fn a_cancel_after_a_send_names_the_send_that_ran() {
        let edits = send_edits();
        let (runner, gate) = FakeScriptRunner::with_stdout(FIVE)
            .streaming(
                &[
                    "[1/4] running Check monitors",
                    "[1/4] done Check monitors",
                    "[2/4] running Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 shows HDMI 1 or drops",
                    "  KG241Y X1: HDMI 1 sent (was DisplayPort 1)",
                    "[2/4] done Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 shows HDMI 1 or drops",
                    "[3/4] running Apply arrangement",
                ],
                0,
            )
            .pausing_after(5);
        let (_dir, app, layout) = app_with(runner);
        app.save_layout(&layout.id, edits.clone()).unwrap();
        let run = {
            let app = Arc::clone(&app);
            let id = layout.id.clone();
            thread::spawn(move || collect(&app, &id))
        };
        wait_until(|| gate.parked());
        app.cancel_switch().unwrap();
        let (result, _) = run.join().unwrap();
        assert_eq!(
            result.unwrap(),
            SwitchResult::Cancelled {
                sent: vec!["Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 shows HDMI 1 or drops".into()]
            }
        );
    }

    #[test]
    fn a_send_skipped_for_a_monitor_that_is_not_active_still_applies() {
        let edits = send_edits();
        let skipped = &[
            "[1/4] done Check monitors",
            "[2/4] running Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 drops",
            "[2/4] skipped Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 drops: KG241Y X1 is not Active",
            "[3/4] done Apply arrangement",
            "[4/4] done Verify",
            "Exit code 0",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(skipped, 0));
        app.save_layout(&layout.id, edits.clone()).unwrap();
        let (result, events) = collect(&app, &layout.id);
        assert!(matches!(result.unwrap(), SwitchResult::Applied { .. }));
        assert!(progress_lines(&events).iter().any(|l| l.starts_with("2/4 Skipped")));
    }

    #[test]
    fn a_send_whose_command_failed_stops_with_the_input_button_action() {
        let edits = send_edits();
        let failed = &[
            "[1/4] done Check monitors",
            "[2/4] running Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 drops",
            "[2/4] failed Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 drops: press the input button on KG241Y X1 to pick HDMI 1, then switch again; could not send the input source (SetVCPFeature failed, Win32 error 31)",
            "Exit code 2",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(failed, 2));
        app.save_layout(&layout.id, edits).unwrap();
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed {
                step,
                next_action,
                explanation,
                ..
            } => {
                assert_eq!(step, Some(2));
                assert_eq!(next_action, "press the input button on KG241Y X1 to pick HDMI 1, then switch again");
                assert_eq!(explanation, FailureExplanation::None);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_needs_you_wait_reaches_the_screen_as_one_row_counting_down_then_applies() {
        let lines = &[
            "[1/3] running Check monitors",
            "  Ultrawide: Absent",
            "Press the input button on Ultrawide, or turn the other device off.",
            "Waiting until Ultrawide is Available",
            "[1/3] needs-you Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 120 s left of 120 s",
            "[1/3] needs-you Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 119 s left of 120 s",
            "  Available; settling",
            "[1/3] done Check monitors",
            "[2/3] done Apply arrangement",
            "[3/3] done Verify",
            "Exit code 0",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(lines, 0));
        let (result, events) = collect(&app, &layout.id);
        assert!(matches!(result.unwrap(), SwitchResult::Applied { .. }));
        let progress = progress_lines(&events);
        assert_eq!(progress[1], "1/3 NeedsYou Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 120 s left of 120 s");
        assert_eq!(progress[2], "1/3 NeedsYou Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 119 s left of 120 s");
        assert_eq!(progress[3], "1/3 Done Check monitors");
    }

    #[test]
    fn a_wait_that_times_out_fails_with_the_input_button_action() {
        let lines = &[
            "[1/3] running Check monitors",
            "[1/3] needs-you Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 1 s left of 120 s",
            "[1/3] failed Check monitors: press the input button on Ultrawide, or plug it in, then switch again; Absent: Ultrawide",
            "Exit code 2",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(lines, 2));
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed {
                step,
                next_action,
                explanation,
                ..
            } => {
                assert_eq!(step, Some(1));
                assert!(next_action.starts_with("press the input button on Ultrawide"));
                assert_eq!(
                    explanation,
                    FailureExplanation::Absent {
                        monitors: vec!["Ultrawide".into()]
                    }
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_send_step_whose_available_wait_times_out_fails_with_its_own_action() {
        let edits = send_edits();
        let lines = &[
            "[1/4] done Check monitors",
            "[2/4] running Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 drops",
            "[2/4] needs-you Waiting until KG241Y X1 is Available: press the input button on KG241Y X1, or turn the other device off; 1 s left of 120 s",
            "[2/4] failed Send HDMI 1 to KG241Y X1, then wait until KG241Y X1 drops: press the input button on KG241Y X1, or turn the other device off, then switch again; KG241Y X1 not Available after 120 s",
            "Exit code 2",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(lines, 2));
        app.save_layout(&layout.id, edits).unwrap();
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed {
                step,
                next_action,
                reason,
                ..
            } => {
                assert_eq!(step, Some(2));
                assert_eq!(next_action, "press the input button on KG241Y X1, or turn the other device off, then switch again");
                assert_eq!(reason, "KG241Y X1 not Available after 120 s");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_cancel_during_a_wait_resolves_cancelled() {
        let (runner, gate) = FakeScriptRunner::with_stdout(FIVE)
            .streaming(
                &[
                    "[1/3] running Check monitors",
                    "[1/3] needs-you Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 120 s left of 120 s",
                    "[1/3] needs-you Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 119 s left of 120 s",
                ],
                0,
            )
            .pausing_after(2);
        let (_dir, app, layout) = app_with(runner);
        let run = {
            let app = Arc::clone(&app);
            let id = layout.id.clone();
            thread::spawn(move || collect(&app, &id))
        };
        wait_until(|| gate.parked());
        app.cancel_switch().unwrap();
        let (result, _) = run.join().unwrap();
        assert_eq!(result.unwrap(), SwitchResult::Cancelled { sent: vec![] });
    }

    #[test]
    fn a_fallback_that_landed_resolves_failed_as_extended_and_a_plain_apply_failure_does_not() {
        let extended = &[
            "[1/3] done Check monitors",
            "[2/3] running Apply arrangement",
            "  Side not Available; falling back to Windows Extend",
            "[2/3] failed Apply arrangement: arrange the monitors in Windows Settings > Display, then save the layout again; Windows Extend was applied instead: Windows could not apply the arrangement, invalid parameter, a monitor in the layout is probably not connected (Windows error 87)",
            "Exit code 2",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(extended, 2));
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed {
                step,
                next_action,
                explanation,
                ..
            } => {
                assert_eq!(step, Some(2));
                assert!(next_action.starts_with("arrange the monitors"));
                assert_eq!(explanation, FailureExplanation::Extended);
            }
            other => panic!("expected Failed, got {other:?}"),
        }

        let plain = &[
            "[1/3] done Check monitors",
            "[2/3] running Apply arrangement",
            "[2/3] failed Apply arrangement: try the switch again, and if it keeps failing arrange the monitors in Windows Settings > Display, then save the layout again; Windows could not apply the arrangement, bad configuration (Windows error 1610)",
            "Exit code 1",
        ];
        let (_dir, app, layout) = app_with(FakeScriptRunner::with_stdout(FIVE).streaming(plain, 1));
        match collect(&app, &layout.id).0.unwrap() {
            SwitchResult::Failed { explanation, .. } => assert_eq!(explanation, FailureExplanation::None),
            other => panic!("expected Failed, got {other:?}"),
        }
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
        let (runner, gate) = FakeScriptRunner::with_stdout(FIVE)
            .streaming(APPLIED, 0)
            .pausing_after(5);
        let (_dir, app, layout) = app_with(runner);
        let run = {
            let app = Arc::clone(&app);
            let id = layout.id.clone();
            thread::spawn(move || collect(&app, &id))
        };
        wait_until(|| gate.parked());
        // The lock the real script would have written by now, and cannot remove once killed.
        fs::write(app.lock_path(), "pid=1\r\nlayout=Desk\r\n").unwrap();
        app.cancel_switch().unwrap();
        let (result, events) = run.join().unwrap();
        assert_eq!(result.unwrap(), SwitchResult::Cancelled { sent: vec![] });
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
