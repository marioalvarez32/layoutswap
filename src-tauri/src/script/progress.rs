//! The progress line a generated script prints on every step status change, and its
//! parser. The format is defined once here and documented in [`super::render`]:
//!
//! ```text
//! [step/of] status text
//! ```
//!
//! where `status` is `running`, `done`, `failed` or `skipped`. Every other line the
//! script prints is log.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts")]
pub enum StepStatus {
    Running,
    Done,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ProgressLine {
    /// One-based.
    pub step: u32,
    pub of: u32,
    pub status: StepStatus,
    /// The step name, with the reason appended after a colon on a failure.
    pub text: String,
}

/// The three pieces of a failed line's text, as the script's `Write-Failure` prints them:
/// `<step name>: <next action>; <reason>`. Missing pieces come back empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureParts {
    pub step_name: String,
    pub next_action: String,
    pub reason: String,
}

impl ProgressLine {
    /// Splits a failed line's text into the step name, the next action and the reason.
    /// A line that is not a failure has no next action: the whole text is the step name.
    pub fn failure_parts(&self) -> FailureParts {
        if self.status != StepStatus::Failed {
            return FailureParts {
                step_name: self.text.clone(),
                next_action: String::new(),
                reason: String::new(),
            };
        }
        let (step_name, rest) = match self.text.split_once(": ") {
            Some((step, rest)) => (step.to_string(), rest),
            None => (String::new(), self.text.as_str()),
        };
        let (next_action, reason) = match rest.split_once("; ") {
            Some((action, reason)) => (action.to_string(), reason.to_string()),
            None => (rest.to_string(), String::new()),
        };
        FailureParts {
            step_name,
            next_action,
            reason,
        }
    }
}

/// Parses one line of script output. Anything that is not a progress line is log.
pub fn parse_progress_line(line: &str) -> Option<ProgressLine> {
    let line = line.trim_end();
    let rest = line.strip_prefix('[')?;
    let (counter, rest) = rest.split_once(']')?;
    let (step, of) = counter.split_once('/')?;
    let step: u32 = step.trim().parse().ok()?;
    let of: u32 = of.trim().parse().ok()?;
    if step == 0 || of == 0 || step > of {
        return None;
    }
    let rest = rest.strip_prefix(' ')?;
    let (status, text) = match rest.split_once(' ') {
        Some((status, text)) => (status, text),
        None => (rest, ""),
    };
    let status = match status {
        "running" => StepStatus::Running,
        "done" => StepStatus::Done,
        "failed" => StepStatus::Failed,
        "skipped" => StepStatus::Skipped,
        _ => return None,
    };
    Some(ProgressLine {
        step,
        of,
        status,
        text: text.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_status() {
        assert_eq!(
            parse_progress_line("[1/3] running Check monitors"),
            Some(ProgressLine {
                step: 1,
                of: 3,
                status: StepStatus::Running,
                text: "Check monitors".into()
            })
        );
        assert_eq!(
            parse_progress_line("[3/3] done Verify\r\n").unwrap().status,
            StepStatus::Done
        );
        assert_eq!(
            parse_progress_line("[2/3] skipped Apply arrangement")
                .unwrap()
                .status,
            StepStatus::Skipped
        );
    }

    #[test]
    fn keeps_the_reason_of_a_failure() {
        let line = parse_progress_line(
            "[2/3] failed Apply arrangement: Windows could not apply the arrangement, bad configuration (Windows error 1610)",
        )
        .unwrap();
        assert_eq!(line.status, StepStatus::Failed);
        assert!(line.text.starts_with("Apply arrangement: "));
        assert!(line.text.ends_with("(Windows error 1610)"));
    }

    #[test]
    fn a_failed_line_splits_into_step_action_and_reason() {
        let line = parse_progress_line(
            "[1/3] failed Check monitors: press the input button on Ultrawide, or plug it in, then switch again; 1 Absent",
        )
        .unwrap();
        assert_eq!(
            line.failure_parts(),
            FailureParts {
                step_name: "Check monitors".into(),
                next_action: "press the input button on Ultrawide, or plug it in, then switch again".into(),
                reason: "1 Absent".into(),
            }
        );
        let bare = parse_progress_line("[2/3] failed Apply arrangement").unwrap();
        assert_eq!(bare.failure_parts().step_name, "");
        assert_eq!(bare.failure_parts().next_action, "Apply arrangement");
        let done = parse_progress_line("[3/3] done Verify").unwrap();
        assert_eq!(done.failure_parts().step_name, "Verify");
        assert_eq!(done.failure_parts().next_action, "");
    }

    #[test]
    fn everything_else_is_log() {
        for line in [
            "",
            "  Ultrawide: Available",
            "2026-09-10 08:00:00  [1/3] done Check monitors",
            "[1/3]",
            "[1/3] waiting Check monitors",
            "[0/3] running x",
            "[4/3] running x",
            "[a/3] running x",
            "=== Switch to Desk ===",
        ] {
            assert_eq!(parse_progress_line(line), None, "{line:?}");
        }
    }
}
