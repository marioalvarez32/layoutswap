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

/// What a step does. The send step arrives with its own ticket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum StepKind {
    /// Wait a number of seconds, for a dock or a monitor to settle.
    Wait { seconds: u32 },
}

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

/// The step as one sentence, the same one the editor row and the script print.
pub fn step_sentence(step: &Step) -> String {
    match &step.kind {
        StepKind::Wait { seconds } => format!(
            "Wait {seconds} {}",
            if *seconds == 1 { "second" } else { "seconds" }
        ),
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

    #[test]
    fn a_step_reads_as_one_sentence() {
        assert_eq!(step_sentence(&wait("a", StepSide::Before, 3)), "Wait 3 seconds");
        assert_eq!(step_sentence(&wait("a", StepSide::Before, 1)), "Wait 1 second");
    }

    #[test]
    fn a_step_serialises_flat_with_its_kind() {
        let json = serde_json::to_string(&wait("a", StepSide::After, 2)).unwrap();
        assert_eq!(json, r#"{"id":"a","side":"after","kind":"wait","seconds":2}"#);
        let back: Step = serde_json::from_str(&json).unwrap();
        assert_eq!(back, wait("a", StepSide::After, 2));
    }
}
