//! Input sources as VCP code 0x60 values monitors agree on (MCCS 2.2), and their names.
//! A code outside the table is still usable: it reads as "Input 0x1E".

use serde::Serialize;
use ts_rs::TS;

/// One entry of the fixed table, as the step editor offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct InputSource {
    pub code: u32,
    pub name: String,
}

/// The fixed table for the editor, in order.
pub fn table() -> Vec<InputSource> {
    INPUT_SOURCES
        .iter()
        .map(|(code, name)| InputSource {
            code: *code,
            name: name.to_string(),
        })
        .collect()
}

/// The fixed table, in the order the step editor offers it.
pub const INPUT_SOURCES: [(u32, &str); 8] = [
    (0x01, "VGA"),
    (0x03, "DVI 1"),
    (0x04, "DVI 2"),
    (0x0F, "DisplayPort 1"),
    (0x10, "DisplayPort 2"),
    (0x11, "HDMI 1"),
    (0x12, "HDMI 2"),
    (0x1B, "USB-C"),
];

/// "HDMI 1" for a known code, "Input 0x1E" for any other.
pub fn name(code: u32) -> String {
    INPUT_SOURCES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| format!("Input 0x{code:02X}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_every_code_in_the_table() {
        for (code, expected) in INPUT_SOURCES {
            assert_eq!(name(code), expected);
        }
        assert_eq!(name(0x11), "HDMI 1");
        assert_eq!(name(0x1B), "USB-C");
    }

    #[test]
    fn the_table_lists_every_entry_in_order() {
        let table = table();
        assert_eq!(table.len(), INPUT_SOURCES.len());
        assert_eq!(table[0].name, "VGA");
        assert_eq!(table[5], InputSource { code: 0x11, name: "HDMI 1".into() });
    }

    #[test]
    fn an_unknown_code_reads_as_hex() {
        assert_eq!(name(0x1E), "Input 0x1E");
        assert_eq!(name(5), "Input 0x05");
        assert_eq!(name(0x100), "Input 0x100");
    }
}
