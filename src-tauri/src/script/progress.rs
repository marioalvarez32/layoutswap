//! The progress line a generated script prints on every step status change, and its
//! parser. The format is defined once here and documented in [`super::render`]:
//!
//! ```text
//! [step/of] status text
//! ```
//!
//! where `status` is one of the words below. The text of a failed or needs-you line has
//! the shape the renderer documents, `<name>: <action>; <detail>`, and the parser
//! splits it once into [`LineParts`] so no other layer parses text. Every other line
//! the script prints is log.

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
    /// Waiting for a physical action, repeated every second with the seconds left.
    #[serde(rename = "needsYou")]
    NeedsYou,
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
    /// The text split into its parts, for a failed or needs-you line; null otherwise.
    pub parts: Option<LineParts>,
}

/// The three pieces of a failed or needs-you line's text, `<name>: <action>; <detail>`:
/// for a failure the step name, the next action and the reason; for a needs-you line
/// what the script waits for, the physical action, and the seconds left. Missing
/// pieces come back empty.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct LineParts {
    pub name: String,
    pub action: String,
    pub detail: String,
}

fn split_parts(text: &str) -> LineParts {
    let (name, rest) = match text.split_once(": ") {
        Some((name, rest)) => (name.to_string(), rest),
        None => (String::new(), text),
    };
    let (action, detail) = match rest.split_once("; ") {
        Some((action, detail)) => (action.to_string(), detail.to_string()),
        None => (rest.to_string(), String::new()),
    };
    LineParts {
        name,
        action,
        detail,
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
        "needs-you" => StepStatus::NeedsYou,
        _ => return None,
    };
    let text = text.trim().to_string();
    let parts = matches!(status, StepStatus::Failed | StepStatus::NeedsYou).then(|| split_parts(&text));
    Some(ProgressLine {
        step,
        of,
        status,
        text,
        parts,
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
                text: "Check monitors".into(),
                parts: None,
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
        let waiting = parse_progress_line(
            "[1/3] needs-you Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 92 s left of 120 s",
        )
        .unwrap();
        assert_eq!(waiting.status, StepStatus::NeedsYou);
        assert!(waiting.text.ends_with("92 s left of 120 s"));
        assert_eq!(
            waiting.parts,
            Some(LineParts {
                name: "Waiting until Ultrawide is Available".into(),
                action: "press the input button on Ultrawide, or turn the other device off".into(),
                detail: "92 s left of 120 s".into(),
            })
        );
        assert_eq!(serde_json::to_string(&waiting.status).unwrap(), "\"needsYou\"");
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
            line.parts,
            Some(LineParts {
                name: "Check monitors".into(),
                action: "press the input button on Ultrawide, or plug it in, then switch again".into(),
                detail: "1 Absent".into(),
            })
        );
        let bare = parse_progress_line("[2/3] failed Apply arrangement").unwrap();
        let parts = bare.parts.unwrap();
        assert_eq!(parts.name, "");
        assert_eq!(parts.action, "Apply arrangement");
        let done = parse_progress_line("[3/3] done Verify").unwrap();
        assert_eq!(done.parts, None);
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
