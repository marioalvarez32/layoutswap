//! Steps: what a layout runs around Apply arrangement, their bounds, and the one
//! sentence each reads as (CONTEXT.md: Step, Timeline, Wait rule).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

pub const DEFAULT_DROP_WAIT_SECONDS: u32 = 5;
pub const DEFAULT_AVAILABLE_WAIT_SECONDS: u32 = 120;
pub const WAIT_SECONDS_MAX: u32 = 600;
pub const DROP_WAIT_SECONDS_MAX: u32 = 60;
pub const AVAILABLE_WAIT_SECONDS_MAX: u32 = 600;

/// One ordered action inside a switch (CONTEXT.md: Step).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Step {
    /// Stable for the life of the step, so the editor can track rows.
    pub id: String,
    pub side: StepSide,
    #[serde(flatten)]
    pub kind: StepKind,
}

/// Which side of Apply arrangement a step runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum StepSide {
    Before,
    After,
}

/// What a step does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum StepKind {
    /// Wait a number of seconds, for a dock or a monitor to settle.
    Wait { seconds: u32 },
    /// Send a monitor to an input source over DDC-CI, then follow the wait rule.
    SendInput {
        /// The monitor, by device path (ADR-0005); any monitor the layout knows.
        device_path: String,
        /// The VCP code 0x60 value, from the fixed table or typed as another code.
        input_source: u32,
        wait: WaitRule,
    },
}

/// What a send step waits for after sending (CONTEXT.md: Wait rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum WaitRule {
    None,
    /// Until the monitor reports the new input over DDC-CI or stops being Available,
    /// up to the layout's drop wait. Some monitors keep the link alive while showing
    /// another device, so the report is the signal that works for them.
    Drop,
    /// Until the monitor is Available again, up to the layout's Available wait.
    Available,
}

/// The largest VCP code 0x60 value a monitor can hold.
pub const INPUT_SOURCE_MAX: u32 = 0xFF;

/// What a switch does when the arrangement apply fails (CONTEXT.md: Extend fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum ApplyFailure {
    Stop,
    Extend,
}

/// What a save from the layout editor changes: the steps, the timings and the fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct LayoutEdits {
    pub steps: Vec<Step>,
    pub drop_wait_seconds: u32,
    pub available_wait_seconds: u32,
    pub on_apply_failure: ApplyFailure,
}

/// The bounds a save must respect: a wait step is 1 to 600 s, the drop wait 0 to 60 s,
/// the Available wait 1 to 600 s, and step ids are unique.
pub fn validate_edits(edits: &LayoutEdits) -> Result<(), AppError> {
    let refuse = |reason: &str| AppError::InvalidLayoutEdit {
        reason: reason.to_string(),
    };
    let mut ids = std::collections::BTreeSet::new();
    for step in &edits.steps {
        if step.id.trim().is_empty() || !ids.insert(step.id.as_str()) {
            return Err(refuse("Every step needs its own id."));
        }
        match &step.kind {
            StepKind::Wait { seconds } if *seconds == 0 || *seconds > WAIT_SECONDS_MAX => {
                return Err(refuse(&format!(
                    "Keep a wait step between 1 and {WAIT_SECONDS_MAX} seconds."
                )));
            }
            StepKind::Wait { .. } => {}
            StepKind::SendInput {
                device_path,
                input_source,
                ..
            } => {
                if device_path.trim().is_empty() {
                    return Err(refuse("Pick a monitor for the send step."));
                }
                if *input_source == 0 || *input_source > INPUT_SOURCE_MAX {
                    return Err(refuse(&format!(
                        "Keep the input source code between 0x01 and 0x{INPUT_SOURCE_MAX:02X}."
                    )));
                }
            }
        }
    }
    if edits.drop_wait_seconds > DROP_WAIT_SECONDS_MAX {
        return Err(refuse(&format!(
            "Keep the drop wait between 0 and {DROP_WAIT_SECONDS_MAX} seconds."
        )));
    }
    if edits.available_wait_seconds == 0
        || edits.available_wait_seconds > AVAILABLE_WAIT_SECONDS_MAX
    {
        return Err(refuse(&format!(
            "Keep the Available wait between 1 and {AVAILABLE_WAIT_SECONDS_MAX} seconds."
        )));
    }
    Ok(())
}

/// The label for a step's monitor the layout does not know.
pub const UNKNOWN_MONITOR: &str = "unknown monitor";

/// The step as one sentence, the same one the editor row and the script print.
/// `label` names a monitor by device path; the send step's wait rule follows after a
/// comma: "Send HDMI 1 to Ultrawide, then wait until Ultrawide drops".
pub fn step_sentence(step: &Step, label: &dyn Fn(&str) -> String) -> String {
    match &step.kind {
        StepKind::Wait { seconds } => format!(
            "Wait {seconds} {}",
            if *seconds == 1 { "second" } else { "seconds" }
        ),
        StepKind::SendInput {
            device_path,
            input_source,
            wait,
        } => {
            let monitor = label(device_path);
            let input = crate::hardware::input_source::name(*input_source);
            let tail = match wait {
                WaitRule::None => String::new(),
                WaitRule::Drop => format!(", then wait until {monitor} shows {input} or drops"),
                WaitRule::Available => format!(", then wait until {monitor} is Available"),
            };
            format!("Send {input} to {monitor}{tail}")
        }
    }
}

/// The label rule for a step's monitor: the layout's label, or "unknown monitor".
pub fn step_labeller(
    aliases: &std::collections::BTreeMap<String, String>,
    monitors: &[super::SummaryMonitor],
) -> impl Fn(&str) -> String {
    let labels: Vec<(String, String)> = monitors
        .iter()
        .zip(super::monitor_labels(aliases, monitors))
        .map(|(m, label)| (m.device_path.clone(), label))
        .collect();
    move |device_path| {
        labels
            .iter()
            .find(|(path, _)| path == device_path)
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| UNKNOWN_MONITOR.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edits(steps: Vec<Step>) -> LayoutEdits {
        LayoutEdits {
            steps,
            drop_wait_seconds: DEFAULT_DROP_WAIT_SECONDS,
            available_wait_seconds: DEFAULT_AVAILABLE_WAIT_SECONDS,
            on_apply_failure: ApplyFailure::Stop,
        }
    }

    fn wait(id: &str, side: StepSide, seconds: u32) -> Step {
        Step {
            id: id.into(),
            side,
            kind: StepKind::Wait { seconds },
        }
    }

    #[test]
    fn edits_keep_waits_and_timings_within_bounds_and_ids_unique() {
        assert!(validate_edits(&edits(vec![
            wait("a", StepSide::Before, 3),
            wait("b", StepSide::After, 600)
        ]))
        .is_ok());
        let zero = validate_edits(&edits(vec![wait("a", StepSide::Before, 0)])).unwrap_err();
        assert!(zero.to_string().contains("between 1 and 600"), "{zero}");
        assert!(validate_edits(&edits(vec![wait("a", StepSide::Before, 601)])).is_err());
        let twice = validate_edits(&edits(vec![
            wait("a", StepSide::Before, 1),
            wait("a", StepSide::After, 1)
        ]))
        .unwrap_err();
        assert!(twice.to_string().contains("own id"), "{twice}");
        let mut drop = edits(vec![]);
        drop.drop_wait_seconds = 61;
        assert!(validate_edits(&drop).is_err());
        let mut available = edits(vec![]);
        available.available_wait_seconds = 0;
        assert!(validate_edits(&available).is_err());
    }

    fn send(id: &str, side: StepSide, path: &str, code: u32, wait: WaitRule) -> Step {
        Step {
            id: id.into(),
            side,
            kind: StepKind::SendInput {
                device_path: path.into(),
                input_source: code,
                wait,
            },
        }
    }

    fn label(path: &str) -> String {
        if path == "ultra" {
            "Ultrawide".into()
        } else {
            UNKNOWN_MONITOR.into()
        }
    }

    #[test]
    fn a_step_reads_as_one_sentence() {
        assert_eq!(step_sentence(&wait("a", StepSide::Before, 3), &label), "Wait 3 seconds");
        assert_eq!(step_sentence(&wait("a", StepSide::Before, 1), &label), "Wait 1 second");
        assert_eq!(
            step_sentence(&send("s", StepSide::Before, "ultra", 0x11, WaitRule::Drop), &label),
            "Send HDMI 1 to Ultrawide, then wait until Ultrawide shows HDMI 1 or drops"
        );
        assert_eq!(
            step_sentence(&send("s", StepSide::After, "ultra", 0x0F, WaitRule::Available), &label),
            "Send DisplayPort 1 to Ultrawide, then wait until Ultrawide is Available"
        );
        assert_eq!(
            step_sentence(&send("s", StepSide::After, "gone", 0x1E, WaitRule::None), &label),
            "Send Input 0x1E to unknown monitor"
        );
    }

    #[test]
    fn a_send_step_needs_a_monitor_and_a_code_a_monitor_can_hold() {
        assert!(validate_edits(&edits(vec![send("s", StepSide::Before, "ultra", 0x11, WaitRule::None)])).is_ok());
        assert!(validate_edits(&edits(vec![send("s", StepSide::Before, "ultra", 0xFF, WaitRule::None)])).is_ok());
        let no_monitor = validate_edits(&edits(vec![send("s", StepSide::Before, " ", 0x11, WaitRule::None)])).unwrap_err();
        assert!(no_monitor.to_string().contains("Pick a monitor"), "{no_monitor}");
        let zero = validate_edits(&edits(vec![send("s", StepSide::Before, "ultra", 0, WaitRule::None)])).unwrap_err();
        assert!(zero.to_string().contains("between 0x01 and 0xFF"), "{zero}");
        assert!(validate_edits(&edits(vec![send("s", StepSide::Before, "ultra", 0x100, WaitRule::None)])).is_err());
    }

    #[test]
    fn the_step_labeller_names_known_monitors_and_calls_the_rest_unknown() {
        let inventory = crate::hardware::parse(include_str!("../../hardware/fixtures/five-monitors.json")).unwrap();
        let summary = super::super::summarise(&inventory);
        let acer = summary.monitors.iter().find(|m| m.reported_name == "KG241Y X1").unwrap();
        let mut aliases = std::collections::BTreeMap::new();
        aliases.insert(acer.device_path.clone(), "Side".to_string());
        let label = step_labeller(&aliases, &summary.monitors);
        assert_eq!(label(&acer.device_path), "Side");
        assert_eq!(label("nope"), "unknown monitor");
    }

    #[test]
    fn a_step_serialises_flat_with_its_kind() {
        let json = serde_json::to_string(&wait("a", StepSide::After, 2)).unwrap();
        assert_eq!(json, r#"{"id":"a","side":"after","kind":"wait","seconds":2}"#);
        let back: Step = serde_json::from_str(&json).unwrap();
        assert_eq!(back, wait("a", StepSide::After, 2));
        let json = serde_json::to_string(&send("s", StepSide::Before, "ultra", 0x11, WaitRule::Drop)).unwrap();
        assert_eq!(
            json,
            r#"{"id":"s","side":"before","kind":"sendInput","devicePath":"ultra","inputSource":17,"wait":"drop"}"#
        );
    }
}
