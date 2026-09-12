//! The capabilities read: `read(runner, script) -> entries by device path`. The
//! script prints one JSON document with each Active monitor's raw DDC-CI capabilities
//! string and the modes Windows lists for it; this module runs it and turns the
//! string into the codes the app cares about (CONTEXT.md: Capabilities).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::config::capabilities::{Capabilities, Mode};
use crate::error::AppError;
use crate::script::run::ScriptRunner;

const VCP_INPUT_SOURCE: &str = "60";
const VCP_POWER_MODE: &str = "D6";

/// The document the capabilities script prints.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilitiesReport {
    read_at: String,
    #[serde(default)]
    monitors: BTreeMap<String, ReportEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportEntry {
    #[serde(default)]
    answered: bool,
    #[serde(default)]
    raw: String,
    #[serde(default)]
    modes: Vec<Mode>,
}

/// Runs the capabilities script at `script` and returns one entry per monitor it read.
pub fn read(
    runner: &dyn ScriptRunner,
    script: &Path,
) -> Result<BTreeMap<String, Capabilities>, AppError> {
    let output = runner.run(script, &[])?;
    if !output.succeeded() {
        return Err(AppError::CapabilitiesFailed {
            exit_code: output.exit_code,
            stderr: output.stderr.trim().to_string(),
        });
    }
    parse(&output.stdout)
}

/// Turns the script's JSON into entries. Pure, so the fixture tests cover the string
/// parsing without a monitor attached.
pub fn parse(json: &str) -> Result<BTreeMap<String, Capabilities>, AppError> {
    let json = json.trim_start_matches('\u{feff}');
    let report: CapabilitiesReport = serde_json::from_str(json)
        .map_err(|source| AppError::CapabilitiesUnreadable { source })?;
    Ok(report
        .monitors
        .into_iter()
        .map(|(device_path, entry)| {
            let answered = entry.answered && !entry.raw.trim().is_empty();
            let capabilities = Capabilities {
                read_at: report.read_at.clone(),
                answered,
                input_codes: if answered {
                    vcp_group(&entry.raw, VCP_INPUT_SOURCE)
                } else {
                    vec![]
                },
                power_modes: if answered {
                    vcp_group(&entry.raw, VCP_POWER_MODE)
                } else {
                    vec![]
                },
                modes: canonical_modes(entry.modes),
                raw: entry.raw,
            };
            (device_path, capabilities)
        })
        .collect())
}

/// The values a capabilities string declares for one VCP code, from its `vcp(...)`
/// group: `60(11 12 0F)` reads as `[0x11, 0x12, 0x0F]`. A code listed without values,
/// or not listed, reads as none. Codes are two hex digits; a code's own values sit in
/// the parentheses right after it, one level down, so `160(` never matches `60`.
pub fn vcp_group(raw: &str, code: &str) -> Vec<u32> {
    let Some(start) = raw.find("vcp(") else {
        return vec![];
    };
    let body = &raw[start + "vcp(".len()..];
    let mut depth = 0usize;
    let mut token = String::new();
    let mut wanted = false;
    let mut values = Vec::new();
    for ch in body.chars() {
        match ch {
            '(' => {
                if depth == 0 {
                    wanted = token.eq_ignore_ascii_case(code);
                }
                depth += 1;
                token.clear();
            }
            ')' => {
                if depth == 0 {
                    break;
                }
                if depth == 1 && wanted {
                    push_hex(&token, &mut values);
                    return values;
                }
                depth -= 1;
                token.clear();
            }
            ' ' => {
                if depth == 1 && wanted {
                    push_hex(&token, &mut values);
                }
                token.clear();
            }
            _ => token.push(ch),
        }
    }
    vec![]
}

fn push_hex(token: &str, into: &mut Vec<u32>) {
    if let Ok(value) = u32::from_str_radix(token.trim(), 16) {
        if !token.trim().is_empty() {
            into.push(value);
        }
    }
}

/// Largest resolution first, then rate ascending, each mode once.
fn canonical_modes(mut modes: Vec<Mode>) -> Vec<Mode> {
    modes.sort_by(|a, b| {
        (b.width, b.height, a.hz).cmp(&(a.width, a.height, b.hz))
    });
    modes.dedup();
    modes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::run::FakeScriptRunner;

    /// The reference machine as the script reported it on 2026-09-11, modes cut to a
    /// handful each: the built-in panel silent, the Acer and the ultrawide wakeable,
    /// the MSI panels declaring power off only. Keyed as the probe fixture spells the
    /// device paths, so the two fixtures describe one machine.
    pub const CAPABILITIES: &str = include_str!("fixtures/capabilities.json");

    fn by_path<'a>(entries: &'a BTreeMap<String, Capabilities>, fragment: &str) -> &'a Capabilities {
        entries
            .iter()
            .find(|(path, _)| path.contains(fragment))
            .map(|(_, entry)| entry)
            .unwrap_or_else(|| panic!("no entry with {fragment}"))
    }

    #[test]
    fn parses_the_input_codes_power_modes_and_modes_of_a_monitor_that_answered() {
        let entries = parse(CAPABILITIES).unwrap();
        assert_eq!(entries.len(), 5);
        let acer = by_path(&entries, "ACR0EC4");
        assert!(acer.answered);
        assert_eq!(acer.read_at, "2026-09-11T19:39:48.0888655-05:00");
        assert_eq!(acer.input_codes, vec![0x11, 0x12, 0x0F]);
        assert_eq!(acer.power_modes, vec![1, 2, 4, 5]);
        assert!(acer.can_wake());
        assert!(acer.raw.starts_with("(prot(monitor)type(LCD)model(ACER)"));
        assert_eq!(
            acer.modes,
            vec![
                Mode { width: 1920, height: 1080, hz: 50 },
                Mode { width: 1920, height: 1080, hz: 59 },
                Mode { width: 1920, height: 1080, hz: 60 },
                Mode { width: 1920, height: 1080, hz: 119 },
                Mode { width: 1920, height: 1080, hz: 120 },
            ],
            "rate ascending within a resolution, the duplicate 60 Hz gone"
        );
        // The ultrawide's string puts spaces before its groups and declares D6(01 05).
        let ultrawide = by_path(&entries, "AUS343F");
        assert_eq!(ultrawide.input_codes, vec![0x11, 0x12, 0x0F]);
        assert_eq!(ultrawide.power_modes, vec![1, 5]);
        assert!(ultrawide.can_wake());
        assert_eq!(
            ultrawide.modes[..3],
            [
                Mode { width: 3440, height: 1440, hz: 60 },
                Mode { width: 3440, height: 1440, hz: 75 },
                Mode { width: 3440, height: 1440, hz: 100 },
            ],
            "largest resolution first"
        );
    }

    #[test]
    fn a_silent_monitor_is_not_answered_and_carries_only_its_modes() {
        let entries = parse(CAPABILITIES).unwrap();
        let built_in = by_path(&entries, "EDO4245");
        assert!(!built_in.answered);
        assert!(built_in.input_codes.is_empty());
        assert!(built_in.power_modes.is_empty());
        assert!(!built_in.can_wake());
        assert_eq!(built_in.raw, "");
        assert_eq!(
            built_in.modes,
            vec![
                Mode { width: 1024, height: 768, hz: 60 },
                Mode { width: 800, height: 600, hz: 60 },
                Mode { width: 800, height: 600, hz: 165 },
                Mode { width: 640, height: 480, hz: 60 },
                Mode { width: 640, height: 480, hz: 165 },
            ]
        );

        // A monitor that claims to have answered with an empty string is silent too.
        let empty = r#"{"readAt":"2026-09-11T12:00:00-05:00","monitors":{"path-x":{"answered":true,"raw":"   ","modes":[]}}}"#;
        let entries = parse(empty).unwrap();
        let x = by_path(&entries, "path-x");
        assert!(!x.answered);
        assert_eq!(x.raw, "   ", "kept as sent, for diagnostics");
    }

    #[test]
    fn a_monitor_that_declares_power_off_only_cannot_be_woken() {
        let entries = parse(CAPABILITIES).unwrap();
        let msi = by_path(&entries, "MSI30E5");
        assert!(msi.answered);
        assert_eq!(msi.input_codes, vec![0x11, 0x0F, 0x10]);
        assert_eq!(msi.power_modes, vec![5]);
        assert!(!msi.can_wake());

        // A string with no power-mode group at all reads the same way.
        let none = r#"{"readAt":"2026-09-11T12:00:00-05:00","monitors":{"path-y":{"answered":true,"raw":"(prot(monitor)vcp(02 60(11 0F) 62))","modes":[]}}}"#;
        let entries = parse(none).unwrap();
        let y = by_path(&entries, "path-y");
        assert!(y.answered);
        assert_eq!(y.input_codes, vec![0x11, 0x0F]);
        assert_eq!(y.power_modes, Vec::<u32>::new());
        assert!(!y.can_wake());
    }

    #[test]
    fn vcp_groups_are_read_from_real_strings_off_the_reference_machine() {
        // Fragments as the monitors sent them on 2026-09-11.
        let acer = "(prot(monitor)type(LCD)model(ACER)cmds(01 02 03 07 0C E3 F3)vcp(04 10 12 14(05 06 08 0B) 16 18 1A 59 5A 5B 5C 5D 5E 60(11 12 0F)   6C 6E 70   9B 9C 9D 9E 9F A0 CC(01 02 03 04 05 06 07 08 09 0A 0C 0D 0E 14 16 1E 24) D6(01 02 04 05) E0(00 04 05)) mswhql(1)asset_eep(40)mccs_ver(2.2))";
        assert_eq!(vcp_group(acer, "60"), vec![0x11, 0x12, 0x0F], "runs of spaces between codes");
        assert_eq!(vcp_group(acer, "D6"), vec![1, 2, 4, 5]);
        let ultrawide = "vcp(02 04 05 08 10 12 14(01 05 06 08 0B) 16 18 1A 52 60(11 12 0F) 62 86(01 02 0B) 87(00 0A 14 1E 28 32) 8D(01 02) AC AE B6 C0 C6 C8 C9 CA CC(01 02) D6(01 05) DF DC(01 02 03 04 05 06 07 08 09 0A 0B))";
        assert_eq!(vcp_group(ultrawide, "60"), vec![0x11, 0x12, 0x0F]);
        assert_eq!(vcp_group(ultrawide, "D6"), vec![1, 5]);
        let msi = "vcp(02 04 60(11 0F 10) 62 CC(00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F 10 11 12 13 14 15 16 17 18 19 1A 1B 1C 1D 1E 1F 20 21 22 23 24 25) D6(05)) mswhql(1)asset_eep(40)mccs_ver(2.2))";
        assert_eq!(vcp_group(msi, "60"), vec![0x11, 0x0F, 0x10]);
        assert_eq!(vcp_group(msi, "D6"), vec![5]);
        // Codes are matched whole and case-insensitively; a code without values is none.
        assert_eq!(vcp_group("vcp(160(01 02) 60 d6(01))", "60"), Vec::<u32>::new());
        assert_eq!(vcp_group("vcp(160(01 02) 60 d6(01))", "D6"), vec![1]);
        assert_eq!(vcp_group("(prot(monitor)cmds(01 02))", "60"), Vec::<u32>::new());
    }

    #[test]
    fn read_runs_the_script_and_parses_its_output() {
        let runner = FakeScriptRunner::with_stdout(CAPABILITIES);
        let entries = read(&runner, Path::new("C:/x/capabilities.ps1")).unwrap();
        assert_eq!(entries.len(), 5);
        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].0.ends_with("capabilities.ps1"));
    }

    #[test]
    fn a_failed_script_is_an_error_with_its_exit_code_and_stderr() {
        let runner = FakeScriptRunner::returning(crate::script::run::ScriptOutput {
            stdout: String::new(),
            stderr: "capabilities failed: QueryDisplayConfig failed with error 87".into(),
            exit_code: 1,
        });
        let error = read(&runner, Path::new("C:/x/capabilities.ps1")).unwrap_err();
        assert!(matches!(error, AppError::CapabilitiesFailed { exit_code: 1, .. }), "{error:?}");
    }
}
