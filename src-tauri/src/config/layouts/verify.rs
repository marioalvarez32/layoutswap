//! Verify: comparing what the probe shows after a switch to the layout's summary.
//!
//! The generated script runs the same comparison on its own; this pure copy is what the
//! app uses to explain a result, and the tests here pin the rules for both: the on set,
//! each on monitor's position and size must match; refresh, rotation and scale
//! differences are warnings.

use super::{Summary, SummaryMonitor};
use crate::hardware::{Inventory, Monitor, MonitorState};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerifyOutcome {
    /// What makes the switch not applied. Empty when it passed.
    pub failures: Vec<String>,
    /// Differences that do not fail the switch.
    pub warnings: Vec<String>,
}

impl VerifyOutcome {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// `label` names each monitor in the messages; the caller supplies the alias fallback.
pub fn verify(
    summary: &Summary,
    inventory: &Inventory,
    label: impl Fn(&SummaryMonitor) -> String,
) -> VerifyOutcome {
    let mut outcome = VerifyOutcome::default();
    let live = |path: &str| -> Option<&Monitor> {
        inventory
            .monitors
            .iter()
            .find(|m| m.device_path.eq_ignore_ascii_case(path))
    };

    for expected in &summary.monitors {
        let name = label(expected);
        let actual = live(&expected.device_path);
        let is_active = actual.is_some_and(|m| m.state == MonitorState::Active);
        match (expected.on, is_active) {
            (true, false) => {
                outcome.failures.push(format!("{name} is not on"));
                continue;
            }
            (false, true) => {
                outcome
                    .failures
                    .push(format!("{name} is on but should be off"));
                continue;
            }
            (false, false) => continue,
            (true, true) => {}
        }
        let actual = actual.expect("active monitors come from the inventory");
        match (expected.position, actual.position) {
            (Some(want), Some(got)) if want != got => outcome.failures.push(format!(
                "{name} landed at {},{} instead of {},{}",
                got.x, got.y, want.x, want.y
            )),
            _ => {}
        }
        match (expected.size, actual.size) {
            (Some(want), Some(got)) if want != got => outcome.failures.push(format!(
                "{name} is {}x{} instead of {}x{}",
                got.width, got.height, want.width, want.height
            )),
            _ => {}
        }
        match (expected.refresh_hz, actual.refresh_hz) {
            (Some(want), Some(got)) if (want - got).abs() > 0.01 => outcome
                .warnings
                .push(format!("{name} runs at {got} Hz instead of {want} Hz")),
            _ => {}
        }
        match (expected.rotation, actual.rotation) {
            (Some(want), Some(got)) if want != got => outcome
                .warnings
                .push(format!("{name} is rotated {got} instead of {want}")),
            _ => {}
        }
        match (expected.scale_percent, actual.scale_percent) {
            (Some(want), Some(got)) if want != got => outcome
                .warnings
                .push(format!("{name} is scaled {got}% instead of {want}%")),
            _ => {}
        }
    }

    // Monitors the summary never saw: the loop above already covered the ones it did.
    for monitor in &inventory.monitors {
        let known = summary
            .monitors
            .iter()
            .any(|m| m.device_path.eq_ignore_ascii_case(&monitor.device_path));
        if monitor.state == MonitorState::Active && !known {
            outcome
                .failures
                .push(format!("an extra monitor is on: {}", monitor.reported_name));
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::layouts::summarise;
    use crate::hardware::{parse, Point, Size};

    const FIVE: &str = include_str!("../../hardware/fixtures/five-monitors.json");

    fn fixture() -> (Summary, Inventory) {
        let inventory = parse(FIVE).unwrap();
        (summarise(&inventory), inventory)
    }

    fn name(m: &SummaryMonitor) -> String {
        m.reported_name.clone()
    }

    fn active_mut<'a>(inventory: &'a mut Inventory, fragment: &str) -> &'a mut Monitor {
        inventory
            .monitors
            .iter_mut()
            .find(|m| m.device_path.contains(fragment))
            .unwrap()
    }

    #[test]
    fn an_exact_match_passes_with_no_warnings() {
        let (summary, inventory) = fixture();
        let outcome = verify(&summary, &inventory, name);
        assert!(outcome.passed(), "{outcome:?}");
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn position_drift_fails_and_states_both_positions() {
        let (summary, mut inventory) = fixture();
        active_mut(&mut inventory, "ACR0EC4").position = Some(Point { x: 1920, y: 0 });
        let outcome = verify(&summary, &inventory, name);
        assert_eq!(
            outcome.failures,
            vec!["KG241Y X1 landed at 1920,0 instead of 0,0"]
        );
    }

    #[test]
    fn size_drift_fails() {
        let (summary, mut inventory) = fixture();
        active_mut(&mut inventory, "EDO4245").size = Some(Size {
            width: 1920,
            height: 1200,
        });
        let outcome = verify(&summary, &inventory, name);
        assert_eq!(
            outcome.failures,
            vec!["Built-in display is 1920x1200 instead of 2560x1600"]
        );
    }

    #[test]
    fn an_extra_on_monitor_fails() {
        let (summary, mut inventory) = fixture();
        let ultrawide = active_mut(&mut inventory, "AUS343F");
        ultrawide.state = MonitorState::Active;
        ultrawide.position = Some(Point { x: 0, y: -1440 });
        ultrawide.size = Some(Size {
            width: 3440,
            height: 1440,
        });
        let outcome = verify(&summary, &inventory, name);
        assert_eq!(outcome.failures, vec!["VG34VQEL1A is on but should be off"]);
    }

    #[test]
    fn a_missing_on_monitor_fails() {
        let (summary, mut inventory) = fixture();
        let acer = active_mut(&mut inventory, "ACR0EC4");
        acer.state = MonitorState::Available;
        acer.position = None;
        acer.size = None;
        let outcome = verify(&summary, &inventory, name);
        assert_eq!(outcome.failures, vec!["KG241Y X1 is not on"]);
    }

    #[test]
    fn a_monitor_the_summary_never_saw_but_is_on_fails() {
        let (summary, mut inventory) = fixture();
        let mut extra = active_mut(&mut inventory, "ACR0EC4").clone();
        extra.device_path = r"\\?\DISPLAY#NEW0001#0#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}".into();
        extra.reported_name = "TV".into();
        inventory.monitors.push(extra);
        let outcome = verify(&summary, &inventory, name);
        assert_eq!(outcome.failures, vec!["an extra monitor is on: TV"]);
    }

    #[test]
    fn a_refresh_only_difference_warns_without_failing() {
        let (summary, mut inventory) = fixture();
        active_mut(&mut inventory, "ACR0EC4").refresh_hz = Some(59.94);
        let outcome = verify(&summary, &inventory, name);
        assert!(outcome.passed(), "{outcome:?}");
        assert_eq!(
            outcome.warnings,
            vec!["KG241Y X1 runs at 59.94 Hz instead of 60 Hz"]
        );
    }

    #[test]
    fn rotation_and_scale_differences_warn() {
        let (summary, mut inventory) = fixture();
        let built_in = active_mut(&mut inventory, "EDO4245");
        built_in.rotation = Some(90);
        built_in.scale_percent = Some(100);
        let outcome = verify(&summary, &inventory, name);
        assert!(outcome.passed());
        assert_eq!(outcome.warnings.len(), 2);
        assert!(outcome.warnings[0].contains("rotated 90 instead of 0"));
        assert!(outcome.warnings[1].contains("scaled 100% instead of 150%"));
    }

    #[test]
    fn uses_the_label_the_caller_supplies() {
        let (summary, mut inventory) = fixture();
        active_mut(&mut inventory, "ACR0EC4").position = Some(Point { x: 1, y: 1 });
        let outcome = verify(&summary, &inventory, |m| format!("<{}>", m.connector));
        assert_eq!(
            outcome.failures,
            vec!["<HDMI> landed at 1,1 instead of 0,0"]
        );
    }
}
