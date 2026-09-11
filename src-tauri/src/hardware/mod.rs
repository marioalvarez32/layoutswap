//! The Windows hardware adapter: `probe(runner, script) -> Inventory`.
//!
//! The implementation runs the probe script through a [`ScriptRunner`] and parses its
//! JSON. Hardware knowledge lives in this one directory: what a device path is, which
//! output technology is which connector, how a DPI becomes a scale, what a DDC-CI
//! input source code is. Nothing else in the crate reads those raw facts.

pub mod input_source;
mod report;

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::script::run::ScriptRunner;
use report::{ProbeReport, ReportMonitor};

/// What the probe found: every monitor Windows knows about and the arrangement of the
/// active ones, as the raw display-config arrays needed to apply it again (ADR-0006).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Inventory {
    /// When the probe ran, as the script's local ISO 8601 timestamp.
    pub probed_at: String,
    pub monitors: Vec<Monitor>,
    pub arrangement: ArrangementBlob,
}

/// One physical monitor, identified by its device path (ADR-0005).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Monitor {
    pub device_path: String,
    /// The name the monitor reports, or a stand-in when it reports none.
    pub reported_name: String,
    /// The GPU's friendly name, empty when Windows did not say.
    pub gpu: String,
    pub gpu_device_path: String,
    /// The physical connector, such as "DisplayPort 2" or "Built-in".
    pub connector: String,
    /// The GDI name (`\\.\DISPLAY3`) while the monitor is Active, else null.
    pub gdi_name: Option<String>,
    pub state: MonitorState,
    pub position: Option<Point>,
    pub size: Option<Size>,
    pub refresh_hz: Option<f64>,
    /// Rotation in degrees clockwise: 0, 90, 180 or 270.
    pub rotation: Option<u32>,
    /// Windows scale, 100 for 100%.
    pub scale_percent: Option<u32>,
    pub primary: bool,
    /// The current input source as a VCP code 0x60 value, when DDC-CI answered.
    pub input_source: Option<u32>,
    /// The input source's name from the fixed table, "Input 0x1E" for an unknown code.
    pub input_source_name: Option<String>,
    /// The power mode as a VCP code 0xD6 value (1 awake, 2 standby, 4 off, 5 power
    /// off), when DDC-CI answered that read. Windows keeps a sleeping monitor Active.
    pub power_mode: Option<u32>,
    /// Asleep: the monitor answered the power-mode read with anything but awake, so a
    /// send to it would be swallowed. No answer, or an older probe, is never asleep.
    pub asleep: bool,
    pub ddc_ci: DdcCi,
}

/// Whether the probe could ask the monitor over DDC-CI. Only an Active monitor is
/// asked, because only it has the GDI name the physical monitor handle comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub enum DdcCi {
    Answered,
    /// Asked and silent within the probe's timeout, no physical monitor handle, or
    /// several handles under one GDI name (clone mode).
    NotAnswering,
    /// Not asked: the monitor is not Active, or the probe predates the read.
    #[default]
    NotRead,
}

impl DdcCi {
    /// The probe's word for the status; anything else reads as not read.
    fn from_probe(word: &str) -> Self {
        match word {
            "answered" => DdcCi::Answered,
            "notAnswering" => DdcCi::NotAnswering,
            _ => DdcCi::NotRead,
        }
    }
}

/// The glossary's three monitor states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "types.ts")]
pub enum MonitorState {
    /// Windows is drawing to it.
    Active,
    /// Plugged in and showing the PC, but Windows is not drawing to it.
    Available,
    /// Unplugged, off, or showing another device's input.
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "types.ts")]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "types.ts")]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// The SetDisplayConfig path and mode arrays exactly as Windows returned them for the
/// active monitors, base64-encoded, with the GPU device paths needed to remap the
/// per-boot adapter identifiers at apply time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ArrangementBlob {
    pub paths: String,
    pub modes: String,
    pub source_adapters: Vec<String>,
    pub target_adapters: Vec<String>,
    pub mode_adapters: Vec<String>,
}

/// Runs the probe script at `script` and returns what it found.
pub fn probe(runner: &dyn ScriptRunner, script: &Path) -> Result<Inventory, AppError> {
    let output = runner.run(script, &[])?;
    if !output.succeeded() {
        return Err(AppError::ProbeFailed {
            exit_code: output.exit_code,
            stderr: output.stderr.trim().to_string(),
            log_path: None,
        });
    }
    parse(&output.stdout)
}

/// Turns the probe's JSON into an [`Inventory`]. Pure, so the fixture tests below
/// cover every derivation without a monitor attached.
pub fn parse(json: &str) -> Result<Inventory, AppError> {
    let json = json.trim_start_matches('\u{feff}');
    let report: ProbeReport =
        serde_json::from_str(json).map_err(|source| AppError::ProbeUnreadable { source })?;
    let mut monitors: Vec<Monitor> = report.monitors.iter().map(monitor_from).collect();
    monitors.sort_by_key(|m| {
        (
            match m.state {
                MonitorState::Active => 0,
                MonitorState::Available => 1,
                MonitorState::Absent => 2,
            },
            m.position.map(|p| p.y).unwrap_or(0),
            m.position.map(|p| p.x).unwrap_or(0),
            m.reported_name.clone(),
        )
    });
    Ok(Inventory {
        probed_at: report.probed_at,
        monitors,
        arrangement: report.arrangement,
    })
}

fn monitor_from(r: &ReportMonitor) -> Monitor {
    let state = match (r.active, r.available) {
        (true, _) => MonitorState::Active,
        (false, true) => MonitorState::Available,
        (false, false) => MonitorState::Absent,
    };
    let connector = connector_name(r.output_technology, r.connector_instance);
    let reported_name = if r.friendly_name.trim().is_empty() {
        if is_internal(r.output_technology) {
            "Built-in display".to_string()
        } else {
            "Unnamed monitor".to_string()
        }
    } else {
        r.friendly_name.trim().to_string()
    };
    let has_mode = r.active && r.has_mode;
    let position = has_mode.then_some(Point { x: r.x, y: r.y });
    let size = has_mode.then_some(Size {
        width: r.width,
        height: r.height,
    });
    let refresh_hz = (r.active && r.refresh_den > 0)
        .then(|| f64::from(r.refresh_num) / f64::from(r.refresh_den));
    let rotation = r.active.then_some(match r.rotation {
        2 => 90,
        3 => 180,
        4 => 270,
        _ => 0,
    });
    let scale_percent = (r.active && r.dpi > 0).then(|| (r.dpi * 100 + 48) / 96);
    let ddc_ci = if r.active {
        DdcCi::from_probe(&r.ddc_ci)
    } else {
        DdcCi::NotRead
    };
    let input_source = (ddc_ci == DdcCi::Answered).then_some(r.input_source).flatten();
    let power_mode = (ddc_ci == DdcCi::Answered).then_some(r.power_mode).flatten();
    Monitor {
        device_path: r.device_path.clone(),
        reported_name,
        gpu: r.gpu_name.trim().to_string(),
        gpu_device_path: r.adapter_device_path.clone(),
        connector,
        gdi_name: (r.active && !r.gdi_name.is_empty()).then(|| r.gdi_name.clone()),
        state,
        position,
        size,
        refresh_hz,
        rotation,
        scale_percent,
        primary: has_mode && r.x == 0 && r.y == 0,
        input_source,
        input_source_name: input_source.map(input_source::name),
        power_mode,
        asleep: power_mode.is_some_and(|mode| mode != POWER_AWAKE),
        ddc_ci,
    }
}

/// VCP code 0xD6 reads 1 for a monitor that is awake; 2, 4 and 5 are its sleep states.
const POWER_AWAKE: u32 = 1;

const OUTPUT_INTERNAL: u32 = 0x8000_0000;

fn is_internal(output_technology: u32) -> bool {
    matches!(output_technology, 6 | 11 | 13 | OUTPUT_INTERNAL)
}

/// DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY to the words a user sees. Windows numbers a
/// connector from one only when the adapter has several of that type, and reports zero
/// for the only one, so a lone HDMI reads "HDMI" and two USB-C outputs read "1" and "2".
fn connector_name(output_technology: u32, instance: u32) -> String {
    let base = match output_technology {
        0 => "VGA",
        1 => "S-Video",
        2 => "Composite",
        3 => "Component",
        4 => "DVI",
        5 => "HDMI",
        6 | 11 | 13 | OUTPUT_INTERNAL => return "Built-in".to_string(),
        9 => "SDI",
        10 => "DisplayPort",
        12 => "UDI",
        15 => "Miracast",
        16 | 17 => "Virtual",
        18 => "USB-C DisplayPort",
        _ => "Other",
    };
    if instance == 0 {
        base.to_string()
    } else {
        format!("{base} {instance}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::run::FakeScriptRunner;

    /// Five monitors on the reference machine: four on, the ultrawide off, and two MSI
    /// panels that report the same name. Hand-written in the probe's raw shape.
    pub const FIVE_MONITORS: &str = include_str!("fixtures/five-monitors.json");

    fn by_path<'a>(inventory: &'a Inventory, fragment: &str) -> &'a Monitor {
        inventory
            .monitors
            .iter()
            .find(|m| m.device_path.contains(fragment))
            .unwrap_or_else(|| panic!("no monitor with {fragment}"))
    }

    #[test]
    fn parses_the_five_monitor_fixture() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        assert_eq!(inventory.monitors.len(), 5);
        assert_eq!(inventory.probed_at, "2026-09-09T14:32:05.1234567-05:00");
        let active = inventory
            .monitors
            .iter()
            .filter(|m| m.state == MonitorState::Active)
            .count();
        assert_eq!(active, 4);
        assert_eq!(inventory.arrangement.source_adapters.len(), 4);
    }

    #[test]
    fn the_off_monitor_is_available_with_no_arrangement_details() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        let ultrawide = by_path(&inventory, "AUS343F");
        assert_eq!(ultrawide.state, MonitorState::Available);
        assert_eq!(ultrawide.reported_name, "VG34VQEL1A");
        assert_eq!(ultrawide.position, None);
        assert_eq!(ultrawide.size, None);
        assert_eq!(ultrawide.refresh_hz, None);
        assert_eq!(ultrawide.gdi_name, None);
        assert!(!ultrawide.primary);
    }

    #[test]
    fn two_identical_panels_read_apart_by_device_path_and_connector() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        let panels: Vec<&Monitor> = inventory
            .monitors
            .iter()
            .filter(|m| m.reported_name == "MSI MP165 E6")
            .collect();
        assert_eq!(panels.len(), 2);
        assert_ne!(panels[0].device_path, panels[1].device_path);
        assert_ne!(panels[0].connector, panels[1].connector);
        assert!(panels
            .iter()
            .all(|p| p.connector.starts_with("USB-C DisplayPort")));
    }

    #[test]
    fn derives_position_size_refresh_rotation_scale_and_primary() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        let acer = by_path(&inventory, "ACR0EC4");
        assert_eq!(acer.state, MonitorState::Active);
        assert_eq!(acer.position, Some(Point { x: 0, y: 0 }));
        assert_eq!(
            acer.size,
            Some(Size {
                width: 1920,
                height: 1080
            })
        );
        assert_eq!(acer.refresh_hz, Some(60.0));
        assert_eq!(acer.rotation, Some(0));
        assert_eq!(acer.scale_percent, Some(100));
        assert!(acer.primary);
        assert_eq!(acer.gdi_name.as_deref(), Some(r"\\.\DISPLAY2"));
        assert_eq!(acer.connector, "HDMI");
        assert_eq!(acer.gpu, "NVIDIA GeForce RTX 5070 Laptop GPU");

        let built_in = by_path(&inventory, "EDO4245");
        assert_eq!(built_in.reported_name, "Built-in display");
        assert_eq!(built_in.connector, "Built-in");
        assert_eq!(built_in.scale_percent, Some(150));
        assert_eq!(built_in.rotation, Some(0));
        assert!(!built_in.primary);
    }

    #[test]
    fn reads_the_input_source_only_where_ddc_ci_answered() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        let acer = by_path(&inventory, "ACR0EC4");
        assert_eq!(acer.ddc_ci, DdcCi::Answered);
        assert_eq!(acer.input_source, Some(0x11));
        assert_eq!(acer.input_source_name.as_deref(), Some("HDMI 1"));
        let built_in = by_path(&inventory, "EDO4245");
        assert_eq!(built_in.ddc_ci, DdcCi::NotAnswering);
        assert_eq!(built_in.input_source, None);
        let ultrawide = by_path(&inventory, "AUS343F");
        assert_eq!(ultrawide.ddc_ci, DdcCi::NotRead);
        assert_eq!(ultrawide.input_source, None);

        // An older probe without the fields reads as not read.
        let older = FIVE_MONITORS
            .replace(r#","inputSource":17,"powerMode":1,"ddcCi":"answered""#, "")
            .replace(r#","inputSource":27,"powerMode":1,"ddcCi":"answered""#, "")
            .replace(r#","inputSource":27,"powerMode":4,"ddcCi":"answered""#, "")
            .replace(r#","inputSource":null,"powerMode":null,"ddcCi":"notAnswering""#, "")
            .replace(r#","inputSource":null,"powerMode":null,"ddcCi":"notRead""#, "");
        assert!(!older.contains("ddcCi"));
        let inventory = parse(&older).unwrap();
        assert!(inventory.monitors.iter().all(|m| m.ddc_ci == DdcCi::NotRead));

        // A word this version does not know reads as not read; an inactive monitor
        // reads as not read whatever the probe said.
        let odd = FIVE_MONITORS
            .replacen(r#""ddcCi":"answered""#, r#""ddcCi":"maybe""#, 1)
            .replacen(r#""ddcCi":"notRead""#, r#""ddcCi":"answered""#, 1);
        let inventory = parse(&odd).unwrap();
        assert_eq!(by_path(&inventory, "ACR0EC4").ddc_ci, DdcCi::NotRead);
        assert_eq!(by_path(&inventory, "AUS343F").ddc_ci, DdcCi::NotRead);
    }

    #[test]
    fn reads_the_power_mode_only_where_ddc_ci_answered_and_derives_asleep() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        let acer = by_path(&inventory, "ACR0EC4");
        assert_eq!(acer.power_mode, Some(1));
        assert!(!acer.asleep);
        let mut panels: Vec<(Option<u32>, bool)> = inventory
            .monitors
            .iter()
            .filter(|m| m.reported_name == "MSI MP165 E6")
            .map(|m| (m.power_mode, m.asleep))
            .collect();
        panels.sort();
        assert_eq!(panels, vec![(Some(1), false), (Some(4), true)]);
        assert_eq!(by_path(&inventory, "EDO4245").power_mode, None);
        assert_eq!(by_path(&inventory, "AUS343F").power_mode, None);

        // A power mode from a monitor whose DDC-CI did not answer is dropped, and it
        // is not asleep: the status decides, not the number.
        let contradictory = FIVE_MONITORS.replace(
            r#""powerMode":null,"ddcCi":"notAnswering""#,
            r#""powerMode":4,"ddcCi":"notAnswering""#,
        );
        let inventory = parse(&contradictory).unwrap();
        let built_in = by_path(&inventory, "EDO4245");
        assert_eq!(built_in.power_mode, None);
        assert!(!built_in.asleep);

        // A monitor that answered the input read but not the power-mode read stays
        // answered, with no power mode and not asleep; an older probe without the
        // field reads the same.
        let partial = FIVE_MONITORS.replacen(r#""powerMode":1,"#, r#""powerMode":null,"#, 1);
        let inventory = parse(&partial).unwrap();
        let acer = by_path(&inventory, "ACR0EC4");
        assert_eq!(acer.ddc_ci, DdcCi::Answered);
        assert_eq!(acer.power_mode, None);
        assert!(!acer.asleep);
        let older = FIVE_MONITORS.replace(r#""powerMode":1,"#, "").replace(r#""powerMode":4,"#, "");
        let inventory = parse(&older).unwrap();
        assert!(inventory.monitors.iter().all(|m| m.power_mode.is_none() && !m.asleep));
    }

    #[test]
    fn an_absent_monitor_has_no_live_details() {
        let json = FIVE_MONITORS.replace(
            r#""active":false,"available":true"#,
            r#""active":false,"available":false"#,
        );
        let inventory = parse(&json).unwrap();
        let ultrawide = by_path(&inventory, "AUS343F");
        assert_eq!(ultrawide.state, MonitorState::Absent);
    }

    #[test]
    fn active_monitors_come_first_in_reading_order() {
        let inventory = parse(FIVE_MONITORS).unwrap();
        let names: Vec<&str> = inventory
            .monitors
            .iter()
            .map(|m| m.reported_name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "MSI MP165 E6",
                "MSI MP165 E6",
                "KG241Y X1",
                "Built-in display",
                "VG34VQEL1A"
            ]
        );
    }

    #[test]
    fn probe_runs_the_script_and_parses_its_output() {
        let runner = FakeScriptRunner::with_stdout(FIVE_MONITORS);
        let inventory = probe(&runner, Path::new(r"C:\root\probe.ps1")).unwrap();
        assert_eq!(inventory.monitors.len(), 5);
        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].1.is_empty(), "the probe takes no arguments");
    }

    #[test]
    fn a_failed_probe_names_the_exit_code_and_what_to_do() {
        let runner = FakeScriptRunner::returning(crate::script::run::ScriptOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "probe failed: QueryDisplayConfig failed with error 87\r\n".into(),
        });
        let error = probe(&runner, Path::new("probe.ps1")).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("exit code 1"), "{message}");
        assert!(message.contains("error 87"), "{message}");
        assert!(message.starts_with("Check that"), "{message}");
    }

    #[test]
    fn unreadable_probe_output_is_reported_as_such() {
        let error = parse("not json").unwrap_err();
        assert!(matches!(error, AppError::ProbeUnreadable { .. }));
    }

    #[test]
    fn connectors_read_as_words() {
        assert_eq!(connector_name(10, 0), "DisplayPort");
        assert_eq!(connector_name(10, 2), "DisplayPort 2");
        assert_eq!(connector_name(5, 1), "HDMI 1");
        assert_eq!(connector_name(OUTPUT_INTERNAL, 3), "Built-in");
        assert_eq!(connector_name(99, 0), "Other");
    }
}
