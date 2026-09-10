//! Layouts as the config stores them, and the pure rules around them: the name rules,
//! the folder slug, how a probe becomes a summary, how a monitor is named, whether a
//! script is stale, and (in `verify`) whether a switch landed. Capture itself (probe
//! plus store) is orchestrated in `crate::app`.

pub mod verify;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::hardware::{ArrangementBlob, Inventory, MonitorState, Point, Size};

/// A named, saved arrangement (ADR-0006): the raw blob that gets applied and the
/// derived summary the UI shows. The summary is never edited, so the two cannot drift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Layout {
    /// Stable for the life of the layout; renames and re-captures keep it.
    pub id: String,
    pub name: String,
    /// The folder under `<app root>\layouts` holding the generated script and its log.
    pub folder: String,
    pub captured_at: String,
    pub arrangement: ArrangementBlob,
    pub summary: Summary,
    /// The generated switch script on disk, or null when none has been written yet.
    #[serde(default)]
    pub script: Option<ScriptRecord>,
}

/// What the app knows about a layout's generated script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ScriptRecord {
    /// The template version it was rendered with.
    pub template_version: u32,
    pub rendered_at: String,
}

/// Whether a layout's script can be trusted (CONTEXT.md: Stale).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum ScriptState {
    /// Written by this template version after the last capture, and present on disk.
    Current,
    /// Older than the template or the layout; regenerating clears it.
    Stale,
    /// Not on disk.
    Missing,
}

/// A layout's script state with the path the app would open, as the detail shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ScriptStatus {
    pub layout_id: String,
    pub state: ScriptState,
    pub path: String,
}

/// The stale rule: a script is stale when its template version is older than the
/// app's or when it was rendered before the layout was last captured.
pub fn script_state(layout: &Layout, template_version: u32, file_exists: bool) -> ScriptState {
    if !file_exists {
        return ScriptState::Missing;
    }
    match &layout.script {
        None => ScriptState::Stale,
        Some(record)
            if record.template_version < template_version
                || is_before(&record.rendered_at, &layout.captured_at) =>
        {
            ScriptState::Stale
        }
        Some(_) => ScriptState::Current,
    }
}

/// Chronological comparison of two RFC 3339 timestamps, whatever their offsets. Falls
/// back to text order when one does not parse, so a hand-edited config still loads.
fn is_before(a: &str, b: &str) -> bool {
    use chrono::DateTime;
    match (
        DateTime::parse_from_rfc3339(a),
        DateTime::parse_from_rfc3339(b),
    ) {
        (Ok(a), Ok(b)) => a < b,
        _ => a < b,
    }
}

/// How a monitor is named wherever it appears: the alias, else the reported name; and
/// when another monitor in the same list would read the same, the reported name under
/// an alias or the connector under a reported name follows after a middle dot. The
/// renderer's `chipLabels` in `src/domain/monitors.ts` is the same rule.
pub fn monitor_labels(
    aliases: &BTreeMap<String, String>,
    monitors: &[SummaryMonitor],
) -> Vec<String> {
    let displays: Vec<(String, String)> = monitors
        .iter()
        .map(|m| {
            match aliases
                .get(&m.device_path)
                .map(|a| a.trim())
                .filter(|a| !a.is_empty())
            {
                Some(alias) => (alias.to_string(), m.reported_name.clone()),
                None => (m.reported_name.clone(), m.connector.clone()),
            }
        })
        .collect();
    displays
        .iter()
        .map(|(label, detail)| {
            let shared = displays.iter().filter(|(other, _)| other == label).count() > 1;
            if shared {
                format!("{label} · {detail}")
            } else {
                label.clone()
            }
        })
        .collect()
}

/// The file name of a layout's switch script inside its folder.
pub const SWITCH_SCRIPT_NAME: &str = "switch.ps1";
pub const SWITCH_LOG_NAME: &str = "switch.log";

/// The readable description of a layout's arrangement, derived at capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Summary {
    /// Every monitor connected at capture: Active ones on, Available ones off.
    pub monitors: Vec<SummaryMonitor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct SummaryMonitor {
    pub device_path: String,
    /// The alias fallback: what to show when the alias map has no entry.
    pub reported_name: String,
    pub connector: String,
    pub on: bool,
    pub position: Option<Point>,
    pub size: Option<Size>,
    pub refresh_hz: Option<f64>,
    pub rotation: Option<u32>,
    pub scale_percent: Option<u32>,
    pub primary: bool,
}

/// What a capture command returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "outcome", rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum CaptureOutcome {
    Saved {
        layout: Box<Layout>,
    },
    /// Another layout already has this name (compared without case). Confirm to replace it.
    NameTaken {
        id: String,
        name: String,
    },
}

pub const NAME_MAX_CHARS: usize = 40;

/// The name rules: trimmed, not empty, at most 40 characters. Returns the trimmed name.
pub fn validate_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(AppError::InvalidLayoutName {
            reason: "Give the layout a name.".into(),
        });
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return Err(AppError::InvalidLayoutName {
            reason: format!("Keep the name to {NAME_MAX_CHARS} characters or fewer."),
        });
    }
    Ok(name.to_string())
}

/// Names are unique regardless of letter case.
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// The lowercase ASCII slug a folder name is derived from.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        "layout".to_string()
    } else {
        out
    }
}

/// The folder for a layout: its slug, with the id appended when another layout's
/// folder would collide on the slug alone.
pub fn folder_name(name: &str, id: &str, taken: &[&str]) -> String {
    let base = slug(name);
    if taken.iter().any(|t| t.eq_ignore_ascii_case(&base)) {
        format!("{base}-{}", short_id(id))
    } else {
        base
    }
}

pub fn short_id(id: &str) -> &str {
    &id[..id.len().min(8)]
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// The summary of what the probe showed: Active monitors on with their details,
/// Available monitors off, Absent monitors left out because they are not connected.
pub fn summarise(inventory: &Inventory) -> Summary {
    let monitors = inventory
        .monitors
        .iter()
        .filter(|m| m.state != MonitorState::Absent)
        .map(|m| {
            let on = m.state == MonitorState::Active;
            SummaryMonitor {
                device_path: m.device_path.clone(),
                reported_name: m.reported_name.clone(),
                connector: m.connector.clone(),
                on,
                position: if on { m.position } else { None },
                size: if on { m.size } else { None },
                refresh_hz: if on { m.refresh_hz } else { None },
                rotation: if on { m.rotation } else { None },
                scale_percent: if on { m.scale_percent } else { None },
                primary: on && m.primary,
            }
        })
        .collect();
    Summary { monitors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(validate_name("  Desk ").unwrap(), "Desk");
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"x".repeat(40)).is_ok());
        let error = validate_name(&"x".repeat(41)).unwrap_err();
        assert!(error.to_string().contains("40 characters"));
    }

    #[test]
    fn names_compare_without_case() {
        assert!(same_name("Desk", "desk "));
        assert!(!same_name("Desk", "Desk 2"));
    }

    #[test]
    fn slugs_are_lowercase_ascii_with_single_dashes() {
        assert_eq!(slug("Desk"), "desk");
        assert_eq!(slug("  Film & TV night!  "), "film-tv-night");
        assert_eq!(slug("Café"), "caf");
        assert_eq!(slug("日本語"), "layout");
        assert_eq!(slug("--a--b--"), "a-b");
    }

    #[test]
    fn a_colliding_slug_gets_the_short_id_appended() {
        assert_eq!(folder_name("Desk", "0123456789abcdef", &[]), "desk");
        assert_eq!(
            folder_name("Desk!", "0123456789abcdef", &["desk"]),
            "desk-01234567"
        );
        assert_eq!(folder_name("Desk", "id", &["DESK"]), "desk-id");
    }

    fn layout_with(script: Option<ScriptRecord>) -> Layout {
        Layout {
            id: "id".into(),
            name: "Desk".into(),
            folder: "desk".into(),
            captured_at: "2026-09-09T14:33:00-05:00".into(),
            arrangement: crate::hardware::ArrangementBlob {
                paths: String::new(),
                modes: String::new(),
                source_adapters: vec![],
                target_adapters: vec![],
                mode_adapters: vec![],
            },
            summary: Summary { monitors: vec![] },
            script,
        }
    }

    #[test]
    fn a_script_is_current_only_when_this_template_wrote_it_after_the_capture() {
        let current = Some(ScriptRecord {
            template_version: 3,
            rendered_at: "2026-09-09T14:34:00-05:00".into(),
        });
        assert_eq!(
            script_state(&layout_with(current.clone()), 3, true),
            ScriptState::Current
        );
        assert_eq!(
            script_state(&layout_with(current.clone()), 4, true),
            ScriptState::Stale
        );
        assert_eq!(
            script_state(&layout_with(current), 3, false),
            ScriptState::Missing
        );
        assert_eq!(
            script_state(&layout_with(None), 3, true),
            ScriptState::Stale
        );
        let before_capture = Some(ScriptRecord {
            template_version: 3,
            rendered_at: "2026-09-09T14:00:00-05:00".into(),
        });
        assert_eq!(
            script_state(&layout_with(before_capture), 3, true),
            ScriptState::Stale
        );
    }

    #[test]
    fn the_stale_rule_compares_instants_not_text() {
        // 14:40 UTC is before 14:33 at UTC-5 (19:33 UTC), although the text sorts after it.
        let mut layout = layout_with(Some(ScriptRecord {
            template_version: 3,
            rendered_at: "2026-09-09T14:40:00+00:00".into(),
        }));
        layout.captured_at = "2026-09-09T14:33:00-05:00".into();
        assert_eq!(script_state(&layout, 3, true), ScriptState::Stale);
        layout.script.as_mut().unwrap().rendered_at = "2026-09-09T19:34:00+00:00".into();
        assert_eq!(script_state(&layout, 3, true), ScriptState::Current);
    }

    #[test]
    fn monitors_are_named_by_alias_or_name_and_told_apart_when_they_read_the_same() {
        let inventory = crate::hardware::parse(FIVE).unwrap();
        let summary = summarise(&inventory);
        let mut aliases = BTreeMap::new();
        let labels = monitor_labels(&aliases, &summary.monitors);
        assert!(labels.contains(&"KG241Y X1".to_string()));
        assert!(labels.contains(&"MSI MP165 E6 · USB-C DisplayPort 1".to_string()));
        assert!(labels.contains(&"MSI MP165 E6 · USB-C DisplayPort 2".to_string()));
        assert!(labels.contains(&"Built-in display".to_string()));

        let acer = summary
            .monitors
            .iter()
            .find(|m| m.reported_name == "KG241Y X1")
            .unwrap();
        aliases.insert(acer.device_path.clone(), "Side".into());
        let labels = monitor_labels(&aliases, &summary.monitors);
        assert!(labels.contains(&"Side".to_string()));
        assert!(!labels.iter().any(|l| l.starts_with("KG241Y")));

        // Two aliases that read the same are told apart by the reported name.
        let msi: Vec<String> = summary
            .monitors
            .iter()
            .filter(|m| m.reported_name == "MSI MP165 E6")
            .map(|m| m.device_path.clone())
            .collect();
        aliases.insert(msi[0].clone(), "Portrait".into());
        aliases.insert(msi[1].clone(), "Portrait".into());
        let labels = monitor_labels(&aliases, &summary.monitors);
        assert_eq!(
            labels
                .iter()
                .filter(|l| l.as_str() == "Portrait · MSI MP165 E6")
                .count(),
            2
        );
    }

    #[test]
    fn summary_keeps_connected_monitors_and_details_of_on_ones() {
        let inventory = crate::hardware::parse(FIVE).unwrap();
        let summary = summarise(&inventory);
        assert_eq!(summary.monitors.len(), 5);
        let on: Vec<&SummaryMonitor> = summary.monitors.iter().filter(|m| m.on).collect();
        let off: Vec<&SummaryMonitor> = summary.monitors.iter().filter(|m| !m.on).collect();
        assert_eq!(on.len(), 4);
        assert_eq!(off.len(), 1);
        assert_eq!(off[0].reported_name, "VG34VQEL1A");
        assert_eq!(off[0].position, None);
        assert!(on.iter().filter(|m| m.primary).count() == 1);
        assert!(on
            .iter()
            .all(|m| m.size.is_some() && m.refresh_hz.is_some()));
    }

    #[test]
    fn summary_leaves_absent_monitors_out() {
        let json = FIVE.replace(
            r#""active":false,"available":true"#,
            r#""active":false,"available":false"#,
        );
        let summary = summarise(&crate::hardware::parse(&json).unwrap());
        assert_eq!(summary.monitors.len(), 4);
    }

    const FIVE: &str = include_str!("../../hardware/fixtures/five-monitors.json");
}
