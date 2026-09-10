//! Layouts as the config stores them, and the pure rules around them: the name rules,
//! the folder slug, and how a probe becomes a summary. Capture itself (probe plus
//! store) is orchestrated in `crate::app`.

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
}

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
        layout: Layout,
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

    const FIVE: &str = include_str!("../hardware/fixtures/five-monitors.json");
}
