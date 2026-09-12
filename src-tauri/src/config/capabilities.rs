//! Capabilities: what a monitor declared it can do, read on Re-check and kept by
//! device path (CONTEXT.md: Capabilities). The entry is what the app stores; how it
//! is read is the hardware module's business.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// VCP code 0xD6 value a monitor must accept as a write for the app to wake it.
pub const POWER_MODE_AWAKE: u32 = 1;

/// One monitor's capabilities, as last read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Capabilities {
    /// When the read ran, as the script's local ISO 8601 timestamp.
    pub read_at: String,
    /// Whether the monitor answered the capabilities request at all. False means the
    /// rest is empty and the monitor is treated as it was before the read.
    pub answered: bool,
    /// The input source codes (VCP 0x60) the monitor declares it accepts.
    pub input_codes: Vec<u32>,
    /// The power mode values (VCP 0xD6) the monitor declares it accepts as a write.
    pub power_modes: Vec<u32>,
    /// The modes Windows lists for the monitor, largest first, without duplicates.
    pub modes: Vec<Mode>,
    /// The capabilities string as the monitor sent it, for diagnostics; empty when it
    /// sent nothing.
    pub raw: String,
}

/// One display mode Windows lists: a resolution at a refresh rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
}

impl Capabilities {
    /// Whether the app can wake the monitor over DDC-CI: it declared the awake power
    /// mode writable. Unknown until a read answered, and false for a monitor that
    /// declares other modes only (the reference MSI panels declare D6(05)).
    pub fn can_wake(&self) -> bool {
        self.answered && self.power_modes.contains(&POWER_MODE_AWAKE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(answered: bool, power_modes: &[u32]) -> Capabilities {
        Capabilities {
            read_at: "2026-09-11T11:52:00.0000000-05:00".into(),
            answered,
            input_codes: vec![0x11, 0x12, 0x0F],
            power_modes: power_modes.to_vec(),
            modes: vec![],
            raw: String::new(),
        }
    }

    #[test]
    fn can_wake_only_when_the_awake_mode_is_declared_writable() {
        assert!(entry(true, &[1, 2, 4, 5]).can_wake(), "the Acer");
        assert!(entry(true, &[1, 5]).can_wake(), "the ultrawide");
        assert!(!entry(true, &[5]).can_wake(), "an MSI panel");
        assert!(!entry(true, &[]).can_wake(), "no D6 group");
        assert!(!entry(false, &[1]).can_wake(), "a silent monitor carries nothing");
    }
}
