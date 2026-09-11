//! The raw shape the probe script prints. Facts only, as Windows reports them; the
//! meaning (state, connector name, scale) is derived in [`super::parse`], where it is
//! tested without hardware.

use serde::Deserialize;

use super::ArrangementBlob;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    pub probed_at: String,
    pub monitors: Vec<ReportMonitor>,
    pub arrangement: ArrangementBlob,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMonitor {
    pub device_path: String,
    #[serde(default)]
    pub friendly_name: String,
    #[serde(default)]
    pub gdi_name: String,
    #[serde(default)]
    pub adapter_device_path: String,
    #[serde(default)]
    pub gpu_name: String,
    /// DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY as an unsigned value.
    pub output_technology: u32,
    #[serde(default)]
    pub connector_instance: u32,
    pub active: bool,
    pub available: bool,
    #[serde(default)]
    pub has_mode: bool,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub refresh_num: u32,
    #[serde(default)]
    pub refresh_den: u32,
    /// DISPLAYCONFIG_ROTATION: 1 identity, 2 rotate 90, 3 rotate 180, 4 rotate 270.
    #[serde(default)]
    pub rotation: u32,
    /// Effective DPI of the monitor, 0 when unknown.
    #[serde(default)]
    pub dpi: u32,
    /// The current input source as VCP code 0x60 reports it, null when not read.
    #[serde(default)]
    pub input_source: Option<u32>,
    /// The power mode as VCP code 0xD6 reports it (1 awake, 2 standby, 4 off, 5 power
    /// off), null when the read did not answer or the probe predates it.
    #[serde(default)]
    pub power_mode: Option<u32>,
    /// `answered`, `notAnswering` or `notRead`; missing or unknown reads as not read.
    #[serde(default)]
    pub ddc_ci: String,
}
