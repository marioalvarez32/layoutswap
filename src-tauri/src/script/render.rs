//! The script renderer: templates -> Script text.
//!
//! This is the deep module of the app. Every Windows PowerShell 5.1 quirk, escaping rule,
//! step ordering and logging convention lives here so every feature gets them for free
//! (ADR-0001, ADR-0004, `docs/windows-behaviour.md`). Templates under
//! `src-tauri/templates/*.ps1.tmpl` are included at compile time and rendered output is
//! checked against golden files under `src-tauri/tests/golden/`.
//!
//! Two kinds of script come out of here:
//!
//! - The **probe**, read-only, rendered on every app start. It prints one JSON document
//!   on standard output and nothing else there, so it carries the `#Requires` line and
//!   the generated-by header but no `USER SETTINGS` block and no log: a log line would
//!   corrupt the JSON. Failures go to standard error with exit code 1.
//! - The **switch script** per layout, self-contained: interop code, the arrangement
//!   blob and the summary are embedded, so it runs alone from a shortcut. It follows the
//!   full shape in `CODING_STANDARDS.md`: `#Requires`, header, `USER SETTINGS`, then the
//!   steps. Every line it prints also lands in `switch.log` beside it with a timestamp.
//!   Monitors are named by the one rule in [`crate::config::layouts::monitor_labels`].
//!
//! # Progress lines
//!
//! A switch script prints one line per step status change, in the format
//! `[step/of] status text` where status is `running`, `done`, `failed` or `skipped`;
//! see [`super::progress`] for the parser. The rows are the layout's [`timeline`]:
//! Check monitors, the steps before the apply, Apply arrangement, the steps after it,
//! Verify, numbered over the whole switch; a cancel is refused from the apply row on.
//! A needs-you line's text is `<waiting>: <action>; <n> s left of <total> s`, printed
//! every second while the script waits for a physical action (every ten seconds from
//! a shortcut launch, and the log keeps every tenth line either way), after the band's
//! two lines once. A failed line's text is
//! `<step name>: <next action>; <reason>`, with ` (Windows error N)` appended when a
//! code exists; [`super::progress::ProgressLine::failure_parts`] splits it. The
//! check step's reason is `Absent: <label>, <label>`, so the app can name the
//! monitors even when its own probe cannot; the apply step's reason starts with
//! `Windows Extend was applied instead` when the fallback landed. Every other line is
//! log.
//!
//! # Exit codes
//!
//! 0 applied, 1 could not apply, 2 needed a user action (press the input button, wait
//! for another switch to finish).
//!
//! # Template version
//!
//! [`TEMPLATE_VERSION`] is bumped whenever a template's behaviour changes. A layout
//! whose script was rendered with an older version is stale and is regenerated on the
//! next app start.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::config::layouts::{
    monitor_labels, step_labeller, step_sentence, ApplyFailure, Layout, Step, StepKind, StepSide,
    WaitRule,
};

/// Bump on every change to a template's behaviour.
pub const TEMPLATE_VERSION: u32 = 7;

/// The fixed rows of every switch, by name.
pub const CHECK_ROW: &str = "Check monitors";
pub const APPLY_ROW: &str = "Apply arrangement";
pub const VERIFY_ROW: &str = "Verify";
/// Check monitors is always the first row.
pub const CHECK_STEP: u32 = 1;

/// The rows a layout's switch runs through, numbered from one, and where the fixed
/// ones sit (CONTEXT.md: Timeline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    pub rows: Vec<String>,
    /// The one-based row of Apply arrangement, from which a cancel is refused.
    pub apply: u32,
    /// The rows that send an input source, so a cancel can say which already ran.
    pub sends: Vec<u32>,
}

impl Timeline {
    /// Verify is always the last row.
    pub fn verify(&self) -> u32 {
        self.rows.len() as u32
    }
}

/// `aliases` names the monitors a send step reads, by the one label rule.
pub fn timeline(layout: &Layout, aliases: &BTreeMap<String, String>) -> Timeline {
    let label = step_labeller(aliases, &layout.summary.monitors);
    let numbered = numbered_steps(layout);
    let before = numbered.iter().filter(|(_, s)| s.side == StepSide::Before);
    let after = numbered.iter().filter(|(_, s)| s.side == StepSide::After);
    let mut rows = vec![CHECK_ROW.to_string()];
    rows.extend(before.map(|(_, s)| step_sentence(s, &label)));
    let apply = rows.len() as u32 + 1;
    rows.push(APPLY_ROW.to_string());
    rows.extend(after.map(|(_, s)| step_sentence(s, &label)));
    rows.push(VERIFY_ROW.to_string());
    let sends = numbered
        .iter()
        .filter(|(_, s)| matches!(s.kind, StepKind::SendInput { .. }))
        .map(|(row, _)| *row)
        .collect();
    Timeline { rows, apply, sends }
}

/// Every step with its row number: the rule that numbers the timeline, in one place.
/// Check monitors is row 1, the before steps follow, the apply takes the next row,
/// then the after steps, then Verify.
fn numbered_steps(layout: &Layout) -> Vec<(u32, &Step)> {
    let mut row = CHECK_STEP;
    let mut out = Vec::new();
    for side in [StepSide::Before, StepSide::After] {
        if side == StepSide::After {
            row += 1; // Apply arrangement
        }
        for step in layout.steps.iter().filter(|s| s.side == side) {
            row += 1;
            out.push((row, step));
        }
    }
    out
}
/// The prefix of the check step's failure reason, followed by the labels joined by
/// a comma and a space.
pub const ABSENT_REASON_PREFIX: &str = "Absent: ";
/// The prefix of the apply step's failure reason when the Extend fallback landed.
pub const EXTENDED_REASON_PREFIX: &str = "Windows Extend was applied instead";

/// Rendered script text, ready to be written to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub text: String,
}

const PROBE_TEMPLATE: &str = include_str!("../../templates/probe.ps1.tmpl");
const SWITCH_TEMPLATE: &str = include_str!("../../templates/switch.ps1.tmpl");

/// The probe script. `rendered_at` lands in the header so a user can tell which app
/// start wrote the file; it is a parameter so golden tests stay stable.
pub fn render_probe(rendered_at: &str) -> Script {
    Script {
        text: PROBE_TEMPLATE.replace("{{RENDERED_AT}}", rendered_at),
    }
}

/// The layout's switch script. `aliases` is the config's alias map, `app_root` is where
/// the lock file lives, and `rendered_at` goes in the header.
pub fn render_switch(
    layout: &Layout,
    aliases: &BTreeMap<String, String>,
    app_root: &str,
    rendered_at: &str,
) -> Script {
    let labels = monitor_labels(aliases, &layout.summary.monitors);
    let entry = |i: usize| {
        let m = &layout.summary.monitors[i];
        format!(
            "    '{}'   # {}",
            ps_escape(&labels[i]),
            ps_escape(&m.device_path)
        )
    };
    let on: Vec<String> = (0..layout.summary.monitors.len())
        .filter(|&i| layout.summary.monitors[i].on)
        .map(entry)
        .collect();
    let off: Vec<String> = (0..layout.summary.monitors.len())
        .filter(|&i| !layout.summary.monitors[i].on)
        .map(entry)
        .collect();
    let summary = EmbeddedSummary {
        monitors: layout
            .summary
            .monitors
            .iter()
            .zip(&labels)
            .map(|(m, label)| EmbeddedMonitor {
                device_path: &m.device_path,
                label: label.clone(),
                on: m.on,
                x: m.position.map(|p| p.x).unwrap_or(0),
                y: m.position.map(|p| p.y).unwrap_or(0),
                width: m.size.map(|s| s.width).unwrap_or(0),
                height: m.size.map(|s| s.height).unwrap_or(0),
                refresh_hz: m.refresh_hz,
                rotation: m.rotation,
                scale_percent: m.scale_percent,
                primary: m.primary,
            })
            .collect(),
    };
    let timeline = timeline(layout, aliases);
    let label = step_labeller(aliases, &layout.summary.monitors);
    let header = timeline
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| format!("[{}/{}] {row}", i + 1, timeline.rows.len()))
        .collect::<Vec<_>>()
        .join("  ");
    let numbered = numbered_steps(layout);
    let blocks = |side: StepSide| -> Vec<String> {
        numbered
            .iter()
            .filter(|(_, s)| s.side == side)
            .map(|(row, s)| step_block(*row, s, &label))
            .collect()
    };
    let before_blocks = blocks(StepSide::Before);
    let after_blocks = blocks(StepSide::After);
    let text = SWITCH_TEMPLATE
        .replace("{{STEPS_HEADER}}", &header)
        .replace("{{STEP_COUNT}}", &timeline.rows.len().to_string())
        .replace("{{APPLY_STEP}}", &timeline.apply.to_string())
        .replace("{{VERIFY_STEP}}", &timeline.verify().to_string())
        .replace("{{DROP_WAIT_SECONDS}}", &layout.drop_wait_seconds.to_string())
        .replace(
            "{{AVAILABLE_WAIT_SECONDS}}",
            &layout.available_wait_seconds.to_string(),
        )
        .replace(
            "{{ON_APPLY_FAILURE}}",
            match layout.on_apply_failure {
                ApplyFailure::Stop => "stop",
                ApplyFailure::Extend => "extend",
            },
        )
        .replace("{{STEPS_BEFORE_PS}}", &before_blocks.join("\n"))
        .replace("{{STEPS_AFTER_PS}}", &after_blocks.join("\n"))
        .replace("{{LAYOUT_NAME}}", &layout.name)
        .replace("{{LAYOUT_NAME_PS}}", &ps_escape(&layout.name))
        .replace("{{LAYOUT_ID}}", &ps_escape(&layout.id))
        .replace("{{CAPTURED_AT}}", &ps_escape(&layout.captured_at))
        .replace("{{APP_ROOT_PS}}", &ps_escape(app_root))
        .replace("{{RENDERED_AT}}", rendered_at)
        .replace("{{TEMPLATE_VERSION}}", &TEMPLATE_VERSION.to_string())
        .replace("{{MONITORS_ON_PS}}", &on.join("\n"))
        .replace("{{MONITORS_OFF_PS}}", &off.join("\n"))
        .replace(
            "{{ARRANGEMENT_JSON}}",
            &here_string_json(&layout.arrangement),
        )
        .replace("{{SUMMARY_JSON}}", &here_string_json(&summary));
    Script { text }
}

/// The PowerShell block for one step at its row. Validate-only runs skip every step;
/// the send step's function handles that itself, along with the skip when its monitor
/// is not Active and the wait rule after the send.
fn step_block(row: u32, step: &Step, label: &dyn Fn(&str) -> String) -> String {
    let sentence = ps_escape(&step_sentence(step, label));
    let rule = "#-----------------------------------------------------------------------------";
    let mut lines = vec![
        rule.to_string(),
        format!("# [{row}] {sentence}"),
        rule.to_string(),
    ];
    match &step.kind {
        StepKind::Wait { seconds } => {
            lines.push("if ($ValidateOnly) {".to_string());
            lines.push(format!("    Write-Step {row} 'skipped' '{sentence}: validated only'"));
            lines.push("} else {".to_string());
            lines.push(format!("    Write-Step {row} 'running' '{sentence}'"));
            lines.push(format!("    Start-Sleep -Seconds {seconds}"));
            lines.push(format!("    Write-Step {row} 'done' '{sentence}'"));
            lines.push("}".to_string());
        }
        StepKind::SendInput {
            device_path,
            input_source,
            wait,
        } => {
            let wait = match wait {
                WaitRule::None => "none",
                WaitRule::Drop => "drop",
                WaitRule::Available => "available",
            };
            let side = match step.side {
                StepSide::Before => "before",
                StepSide::After => "after",
            };
            lines.push(format!(
                "Send-InputSource -Row {row} -Sentence '{sentence}' -DevicePath '{}' -Label '{}' -Code {input_source} -InputName '{}' -Wait '{wait}' -Side '{side}'",
                ps_escape(device_path),
                ps_escape(&label(device_path)),
                ps_escape(&crate::hardware::input_source::name(*input_source)),
            ));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Inside a single-quoted PowerShell string only the quote itself needs care.
fn ps_escape(value: &str) -> String {
    value.replace('\'', "''")
}

/// Pretty JSON for a single-quoted here-string. Indented lines never start with `'@`,
/// so the here-string cannot be closed early; nothing else is interpreted inside it.
fn here_string_json(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value).expect("layout data serialises")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddedSummary<'a> {
    monitors: Vec<EmbeddedMonitor<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddedMonitor<'a> {
    device_path: &'a str,
    label: String,
    on: bool,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    refresh_hz: Option<f64>,
    rotation: Option<u32>,
    scale_percent: Option<u32>,
    primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::layouts::{
        summarise, Step, StepKind, Summary, WaitRule, DEFAULT_AVAILABLE_WAIT_SECONDS,
        DEFAULT_DROP_WAIT_SECONDS,
    };
    use crate::hardware::{parse, ArrangementBlob, Inventory, MonitorState};
    use crate::script::progress::{parse_progress_line, StepStatus};
    use std::fs;
    use std::path::PathBuf;

    const RENDERED_AT: &str = "2026-09-09T14:32:05-05:00";
    const APP_ROOT: &str = r"C:\Users\example\AppData\Local\layoutswap";
    const FIVE: &str = include_str!("../hardware/fixtures/five-monitors.json");

    fn golden_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(name)
    }

    /// Line endings depend on the checkout (`.gitattributes` makes both files CRLF on
    /// Windows and LF elsewhere), so the comparison ignores them.
    fn normalise(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    fn layout(name: &str, inventory: &Inventory) -> Layout {
        Layout {
            id: "0123456789abcdef0123456789abcdef".into(),
            name: name.into(),
            folder: crate::config::layouts::slug(name),
            captured_at: "2026-09-09T14:33:00-05:00".into(),
            updated_at: "2026-09-09T14:33:00-05:00".into(),
            arrangement: inventory.arrangement.clone(),
            summary: summarise(inventory),
            steps: vec![],
            drop_wait_seconds: DEFAULT_DROP_WAIT_SECONDS,
            available_wait_seconds: DEFAULT_AVAILABLE_WAIT_SECONDS,
            on_apply_failure: ApplyFailure::Stop,
            script: None,
        }
    }

    fn wait(id: &str, side: StepSide, seconds: u32) -> Step {
        Step {
            id: id.into(),
            side,
            kind: StepKind::Wait { seconds },
        }
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

    fn path_of(inventory: &Inventory, fragment: &str) -> String {
        inventory
            .monitors
            .iter()
            .find(|m| m.device_path.contains(fragment))
            .unwrap()
            .device_path
            .clone()
    }

    fn small_blob(paths: usize) -> ArrangementBlob {
        ArrangementBlob {
            paths: "QUFB".into(),
            modes: "QkJC".into(),
            source_adapters: vec!["gpu-a".into(); paths],
            target_adapters: vec!["gpu-a".into(); paths],
            mode_adapters: vec!["gpu-a".into(); paths * 2],
        }
    }

    /// The three fixtures the golden files cover, rendered with the aliases each needs.
    fn fixtures() -> Vec<(&'static str, Layout, BTreeMap<String, String>)> {
        // Five monitors, one directly above the primary, one off, two identical names.
        let five = parse(FIVE).unwrap();
        let desk = layout("Desk", &five);

        // Two monitors, one off; the blob covers the one on monitor.
        let mut two = five.clone();
        two.monitors
            .retain(|m| m.device_path.contains("ACR0EC4") || m.device_path.contains("AUS343F"));
        two.arrangement = small_blob(1);
        let one_off = layout("Film night", &two);

        // Two monitors with the same reported name, both on, one of them aliased.
        let mut twins = five.clone();
        twins.monitors.retain(|m| m.reported_name == "MSI MP165 E6");
        twins.arrangement = small_blob(2);
        assert!(twins
            .monitors
            .iter()
            .all(|m| m.state == MonitorState::Active));
        let twins_layout = layout("Meeting's twins", &twins);
        let mut twin_aliases = BTreeMap::new();
        twin_aliases.insert(
            twins.monitors[0].device_path.clone(),
            "Portrait".to_string(),
        );

        // The desk again with wait steps on both sides of the apply.
        let mut steps = layout("Desk with steps", &five);
        steps.steps = vec![
            wait("step-a", StepSide::Before, 3),
            wait("step-b", StepSide::After, 1),
            wait("step-c", StepSide::After, 10),
        ];

        // The console direction: send the ultrawide to HDMI 1 and wait for it to drop,
        // before the apply turns it off. The ultrawide is aliased.
        let mut console = layout("Console", &five);
        console.steps = vec![send(
            "send-hdmi",
            StepSide::Before,
            &path_of(&five, "AUS343F"),
            0x11,
            WaitRule::Drop,
        )];
        console.drop_wait_seconds = 8;
        let mut console_aliases = BTreeMap::new();
        console_aliases.insert(path_of(&five, "AUS343F"), "Ultrawide".to_string());

        // The desk direction: after the apply, send the Acer back to HDMI 1 with no wait,
        // and a step on a monitor the layout never saw.
        let mut desk_return = layout("Desk return", &five);
        desk_return.on_apply_failure = ApplyFailure::Extend;
        desk_return.steps = vec![
            send("send-back", StepSide::After, &path_of(&five, "ACR0EC4"), 0x11, WaitRule::None),
            send("send-gone", StepSide::After, "gone", 0x1E, WaitRule::Available),
        ];

        vec![
            ("switch-desk.ps1", desk, BTreeMap::new()),
            ("switch-one-off.ps1", one_off, BTreeMap::new()),
            ("switch-twins.ps1", twins_layout, twin_aliases),
            ("switch-steps.ps1", steps, BTreeMap::new()),
            ("switch-console.ps1", console, console_aliases),
            ("switch-desk-return.ps1", desk_return, BTreeMap::new()),
        ]
    }

    fn render(layout: &Layout, aliases: &BTreeMap<String, String>) -> String {
        render_switch(layout, aliases, APP_ROOT, RENDERED_AT).text
    }

    #[test]
    fn the_probe_renders_to_its_golden_file() {
        let rendered = render_probe(RENDERED_AT).text;
        let golden = fs::read_to_string(golden_path("probe.ps1")).expect(
            "tests/golden/probe.ps1 is missing; run `cargo test -- --ignored write_golden_files`",
        );
        assert_eq!(
            normalise(&rendered),
            normalise(&golden),
            "the rendered probe differs from tests/golden/probe.ps1; if the change is intended, run `cargo test -- --ignored write_golden_files`"
        );
    }

    #[test]
    fn the_switch_scripts_render_to_their_golden_files() {
        for (name, layout, aliases) in fixtures() {
            let golden = fs::read_to_string(golden_path(name)).unwrap_or_else(|_| {
                panic!("tests/golden/{name} is missing; run `cargo test -- --ignored write_golden_files`")
            });
            assert_eq!(
                normalise(&render(&layout, &aliases)),
                normalise(&golden),
                "the rendered script differs from tests/golden/{name}; if the change is intended, run `cargo test -- --ignored write_golden_files`"
            );
        }
    }

    #[test]
    fn the_timeline_numbers_steps_around_the_fixed_rows() {
        let (_, layout, aliases) = fixtures().remove(3);
        let t = timeline(&layout, &aliases);
        assert_eq!(
            t.rows,
            vec![
                "Check monitors",
                "Wait 3 seconds",
                "Apply arrangement",
                "Wait 1 second",
                "Wait 10 seconds",
                "Verify"
            ]
        );
        assert_eq!(t.apply, 3);
        assert_eq!(t.verify(), 6);
        let text = render(&layout, &aliases);
        assert!(text.contains("# Steps: [1/6] Check monitors  [2/6] Wait 3 seconds  [3/6] Apply arrangement  [4/6] Wait 1 second  [5/6] Wait 10 seconds  [6/6] Verify"), "{text}");
        assert!(text.contains("Write-Step 2 'running' 'Wait 3 seconds'"));
        assert!(text.contains("Write-Step 5 'done' 'Wait 10 seconds'"));
        assert!(text.contains("Write-Step $ApplyStep 'running' 'Apply arrangement'"));
        assert!(text.contains("Write-Step $VerifyStep 'done' 'Verify'"));
        assert!(text.contains("    Write-Step 2 'running' 'Wait 3 seconds'\n    Start-Sleep -Seconds 3\n    Write-Step 2 'done'"), "{text}");
        let wait_at = text.find("Write-Step 2 'running'").unwrap();
        let apply_at = text.find("Write-Step $ApplyStep 'running'").unwrap();
        let after_at = text.find("Write-Step 4 'running'").unwrap();
        let verify_at = text.find("Write-Step $VerifyStep 'running'").unwrap();
        assert!(wait_at < apply_at && apply_at < after_at && after_at < verify_at);
        assert_eq!(timeline(&fixtures().remove(0).1, &BTreeMap::new()).apply, 2);
        assert!(t.sends.is_empty());
    }

    #[test]
    fn a_send_step_renders_as_one_call_with_its_label_code_and_wait() {
        let (_, console, aliases) = fixtures().remove(4);
        let t = timeline(&console, &aliases);
        assert_eq!(
            t.rows[1],
            "Send HDMI 1 to Ultrawide, then wait until Ultrawide shows HDMI 1 or drops"
        );
        assert_eq!(t.sends, vec![2]);
        let text = render(&console, &aliases);
        let call = text
            .lines()
            .find(|l| l.starts_with("Send-InputSource -Row 2 "))
            .expect("the send call");
        assert!(call.contains("-Label 'Ultrawide' -Code 17 -InputName 'HDMI 1' -Wait 'drop' -Side 'before'"), "{call}");
        assert!(call.contains("AUS343F"), "{call}");
        assert!(text.contains("$DropWaitSeconds      = 8"), "{text}");
        assert!(text.contains("$AvailableWaitSeconds = 120"));
        assert!(text.contains("function Send-InputSource"));
        assert!(text.contains("SetVCPFeature"));
        assert!(text.contains("SetLastError = true"));
        assert!(text.contains("function Wait-Available"));
        assert!(text.contains("ReadInput"), "the drop wait reads the input back");
        assert!(text.contains("'needs-you'"));
        assert!(text.contains("$AvailableSettleSeconds = 3"));

        let (_, desk_return, aliases) = fixtures().remove(5);
        let t = timeline(&desk_return, &aliases);
        assert_eq!(t.rows[2], "Send HDMI 1 to KG241Y X1");
        assert_eq!(t.rows[3], "Send Input 0x1E to unknown monitor, then wait until unknown monitor is Available");
        assert_eq!(t.sends, vec![3, 4]);
        let text = render(&desk_return, &aliases);
        let send_at = text.find("Send-InputSource -Row 3 ").unwrap();
        let apply_at = text.find("Write-Step $ApplyStep 'done'").unwrap();
        let verify_at = text.find("Write-Step $VerifyStep 'running'").unwrap();
        assert!(apply_at < send_at && send_at < verify_at);
        assert!(text.contains("-Label 'unknown monitor' -Code 30 -InputName 'Input 0x1E' -Wait 'available' -Side 'after'"), "{text}");
        assert!(text.contains("$OnApplyFailure       = 'extend'"), "{text}");
        assert!(text.contains("SDC_TOPOLOGY_EXTEND"));
        assert!(text.contains(&format!("\"{EXTENDED_REASON_PREFIX}: {{0}} (Windows error {{1}})\" -f $reason, $rc")), "{text}");
        assert!(text.contains("function Stop-Apply"));
        let validate_at = text.find("Stop-Apply $ArrangeAgain").expect("the validate failure goes through the fallback");
        let apply_at = text.find("Stop-Apply ('try the switch again").expect("the apply failure goes through the fallback");
        assert!(validate_at < apply_at);
        let (_, console, aliases) = fixtures().remove(4);
        assert!(render(&console, &aliases).contains("$OnApplyFailure       = 'stop'"));
    }

    #[test]
    fn the_probe_starts_with_the_requires_line_and_header() {
        let text = render_probe(RENDERED_AT).text;
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("#Requires -Version 5.1"));
        assert_eq!(lines.next(), Some("# generated by layoutswap: probe"));
        assert_eq!(lines.next(), Some("# rendered 2026-09-09T14:32:05-05:00"));
        assert!(!text.contains("{{"), "every placeholder is filled");
    }

    #[test]
    fn the_switch_script_has_the_documented_shape() {
        let (_, desk, aliases) = fixtures().remove(0);
        let text = render(&desk, &aliases);
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("#Requires -Version 5.1"));
        assert_eq!(
            lines.next(),
            Some("# generated by layoutswap: switch to Desk")
        );
        assert_eq!(
            lines.next().map(str::to_string),
            Some(format!(
                "# rendered 2026-09-09T14:32:05-05:00 with template version {TEMPLATE_VERSION}"
            ))
        );
        assert!(!text.contains("{{"), "every placeholder is filled");
        let settings = text.find("# USER SETTINGS").unwrap();
        let data = text.find("# Embedded layout data").unwrap();
        let steps = text.find("Write-Step 1 'running'").unwrap();
        assert!(
            settings < data && data < steps,
            "settings, then data, then steps"
        );
        assert!(text.contains(&format!("$AppRoot       = '{APP_ROOT}'")));
        assert!(text.contains("switch.lock"));
        assert!(text.contains("switch.log"));
        assert!(text.contains("[switch]$Pause"));
        assert!(text.contains("[switch]$ValidateOnly"));
        assert!(
            text.contains("PSTypeName]'LayoutswapSwitch.Api'"),
            "Add-Type is guarded"
        );
        assert!(
            text.contains("BytesToPaths(byte[] b)"),
            "non-generic wrappers"
        );
        assert!(
            text.contains("# Exit codes: 0 applied, 1 could not apply, 2 needed a user action."),
            "exit codes documented in the header"
        );
    }

    /// The format is written once in the template; every Write-Step call it makes must
    /// come out as a line the app's parser reads back.
    #[test]
    fn every_progress_line_the_template_prints_parses() {
        // The steps fixture has six rows; the fixed rows print through variables.
        let (_, desk, aliases) = fixtures().remove(3);
        let t = timeline(&desk, &aliases);
        let of = t.rows.len() as u32;
        let raw = render(&desk, &aliases);
        assert!(raw.contains("$ApplyStep     = 3"));
        assert!(raw.contains("$VerifyStep    = 6"));
        let text = raw
            .replace("$ApplyStep", &t.apply.to_string())
            .replace("$VerifyStep", &t.verify().to_string());
        assert!(text.contains(r#""[{0}/{1}] {2} {3}" -f $step, $StepCount, $status, $text"#));
        // The app lists the rows before the script reports any: the two agree.
        let header = text
            .lines()
            .find(|l| l.starts_with("# Steps:"))
            .expect("the header lists the rows");
        assert_eq!(header, "# Steps: [1/6] Check monitors  [2/6] Wait 3 seconds  [3/6] Apply arrangement  [4/6] Wait 1 second  [5/6] Wait 10 seconds  [6/6] Verify");
        assert!(text.contains("$StepCount     = 6"));
        assert!(text.contains("[Console]::OutputEncoding"));
        assert!(text.contains(&format!("(\"{ABSENT_REASON_PREFIX}{{0}}\" -f $names)")), "{text}");
        let mut seen = 0;
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("Write-Step ") else {
                continue;
            };
            let Some(step) = rest.chars().next().filter(char::is_ascii_digit) else {
                continue;
            };
            let parts: Vec<&str> = rest.split('\'').collect();
            let (status, step_text) = (parts[1], parts[3]);
            let printed = format!("[{step}/{of}] {status} {step_text}");
            let parsed = parse_progress_line(&printed)
                .unwrap_or_else(|| panic!("{printed:?} does not parse"));
            assert_eq!(parsed.step, step.to_digit(10).unwrap());
            assert_eq!(parsed.of, of);
            assert_eq!(parsed.text, step_text);
            assert!(matches!(
                (status, parsed.status),
                ("running", StepStatus::Running)
                    | ("done", StepStatus::Done)
                    | ("skipped", StepStatus::Skipped)
            ));
            seen += 1;
        }
        assert!(seen >= 15, "found only {seen} Write-Step calls");
        // A failure line built the same way parses too.
        let failure = parse_progress_line(
            "[2/3] failed Apply arrangement: try the switch again; Windows could not apply the arrangement, bad configuration (Windows error 1610)",
        )
        .unwrap();
        assert_eq!(failure.status, StepStatus::Failed);
    }

    #[test]
    fn the_switch_script_embeds_blob_and_summary_and_lists_on_and_off_monitors() {
        let (_, desk, aliases) = fixtures().remove(0);
        let text = render(&desk, &aliases);
        assert!(text.contains(&format!("\"paths\": \"{}\"", desk.arrangement.paths)));
        assert!(text.contains("\"modeAdapters\""));
        assert!(
            !text.contains("\"reportedName\""),
            "the summary embeds labels, not raw names"
        );
        assert!(text.contains("\"label\": \"KG241Y X1\""));
        let on_block = &text[text.find("$MonitorsOn").unwrap()..text.find("$MonitorsOff").unwrap()];
        assert_eq!(on_block.matches("    '").count(), 4);
        let off_block =
            &text[text.find("$MonitorsOff").unwrap()..text.find("# Embedded layout data").unwrap()];
        assert_eq!(off_block.matches("    '").count(), 1);
        assert!(off_block.contains("VG34VQEL1A"));
    }

    #[test]
    fn identical_panels_read_apart_in_the_script() {
        let (_, desk, aliases) = fixtures().remove(0);
        let text = render(&desk, &aliases);
        assert!(text.contains("    'MSI MP165 E6 \u{b7} USB-C DisplayPort 1'"));
        assert!(text.contains("    'MSI MP165 E6 \u{b7} USB-C DisplayPort 2'"));
        assert!(!text.contains("    'MSI MP165 E6'   #"));

        let (_, twins, aliases) = fixtures().remove(2);
        let text = render(&twins, &aliases);
        assert!(
            text.contains("    'Portrait'"),
            "the alias is used on its own"
        );
        assert!(
            text.contains("    'MSI MP165 E6'"),
            "the other panel needs no connector"
        );
    }

    #[test]
    fn single_quotes_in_names_are_doubled_for_powershell() {
        let (_, twins, aliases) = fixtures().remove(2);
        let text = render(&twins, &aliases);
        assert!(text.contains("$LayoutName    = 'Meeting''s twins'"));
        assert!(text.contains("# generated by layoutswap: switch to Meeting's twins"));
    }

    #[test]
    fn a_summary_with_no_monitors_still_renders() {
        let five = parse(FIVE).unwrap();
        let mut empty = layout("Empty", &five);
        empty.summary = Summary { monitors: vec![] };
        empty.arrangement = small_blob(0);
        let text = render(&empty, &BTreeMap::new());
        assert!(text.contains("$MonitorsOn    = @(\n\n)"));
    }

    /// Rewrites the golden files from the current templates. Run on purpose, then
    /// review the diff.
    #[test]
    #[ignore]
    fn write_golden_files() {
        fs::create_dir_all(golden_path("")).unwrap();
        fs::write(golden_path("probe.ps1"), render_probe(RENDERED_AT).text).unwrap();
        for (name, layout, aliases) in fixtures() {
            fs::write(golden_path(name), render(&layout, &aliases)).unwrap();
        }
    }
}
