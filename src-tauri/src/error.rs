//! The one error type of the crate.
//!
//! Every message is user-facing and says what to do next before what went wrong
//! (DESIGN.md, "Copy voice"). Commands return `Result<T, AppError>`; the webview receives
//! the serialized [`AppErrorPayload`].

use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(
        "Update layoutswap, or move the config file at {path} aside to start fresh: \
         it uses schema version {found} and this version of layoutswap reads up to {supported}."
    )]
    UnknownSchemaVersion {
        found: u64,
        supported: u32,
        path: PathBuf,
    },

    #[error(
        "Move the config file at {path} aside to start fresh, or restore one with Import: \
         the file is not valid ({source})."
    )]
    ConfigInvalid {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "Check that the config file at {path} is readable, then open layoutswap again \
         ({source})."
    )]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Check that the folder of {path} is writable, then try again ({source}).")]
    ConfigWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "Sign in with a normal Windows user account and open layoutswap again: \
         the local application data folder (LOCALAPPDATA) is not set."
    )]
    AppRootUnavailable,

    #[error("Check that Windows PowerShell is installed, then try again: could not start {path} ({source}).")]
    ScriptSpawn {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Check that the folder of {path} is writable, then open layoutswap again ({source}).")]
    ScriptWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "Check that no other program is changing the monitors, then try again: \
         the probe exited with exit code {exit_code} ({stderr})."
    )]
    ProbeFailed {
        exit_code: i32,
        stderr: String,
        /// `probe.log` in the app root, once the app has written the failure there.
        log_path: Option<PathBuf>,
    },

    #[error(
        "Update layoutswap: the probe reported something this version cannot read ({source})."
    )]
    ProbeUnreadable {
        #[source]
        source: serde_json::Error,
    },

    #[error("Wait for layoutswap to finish reading the monitors, then try again.")]
    NoProbeYet,

    #[error("{reason}")]
    InvalidLayoutName { reason: String },

    #[error("Pick the layout again from the sidebar: no layout has the id {id} any more.")]
    LayoutNotFound { id: String },

    #[error("Check that the folder {path} is writable, then try again ({source}).")]
    LayoutFolder {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Regenerate the script from the layout, then try again: {path} is not on disk.")]
    ScriptMissing { path: PathBuf },

    #[error("Open {path} yourself: Windows could not start the program for it ({source}).")]
    FileOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Switch to the layout once, then open its log: {path} is not on disk yet.")]
    LogMissing { path: PathBuf },

    #[error("Open Settings > Display yourself, from the Start menu: Windows could not open it ({source}).")]
    SettingsOpen {
        #[source]
        source: std::io::Error,
    },

    #[error("Pick another place to save the diagnostics: could not write {path} ({source}).")]
    DiagnosticsWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Pick another place to save the config: could not write {path} ({source}).")]
    ExportWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Pick another file to import: {path} was refused, {problem}.")]
    ImportRefused { path: PathBuf, problem: String },

    #[error("Wait for the switch to {layout} to finish, then try again.")]
    SwitchRunning { layout: String },

    #[error(
        "Wait for the switch to {layout} to finish, then try again. If no switch is running, delete {path} first."
    )]
    SwitchLocked { layout: String, path: PathBuf },

    #[error("Nothing to cancel: no switch is running.")]
    NoSwitchRunning,

    #[error(
        "Wait for the switch to {layout} to finish: the arrangement is already being applied and cannot be stopped."
    )]
    CancelTooLate { layout: String },

    #[error(
        "Open layoutswap again and try once more: something inside the app failed ({detail})."
    )]
    Internal { detail: String },
}

impl AppError {
    /// Why a config file cannot be imported, as a clause after the file name.
    pub fn import_problem(&self) -> String {
        match self {
            AppError::UnknownSchemaVersion {
                found, supported, ..
            } => format!(
                "it uses schema version {found} and this version of layoutswap reads up to {supported}"
            ),
            AppError::ConfigInvalid { source, .. } => {
                format!("the file is not a layoutswap config: {source}")
            }
            AppError::ConfigRead { source, .. } => {
                format!("the file could not be read: {source}")
            }
            other => other.to_string(),
        }
    }
}

/// What the webview receives when a command fails.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct AppErrorPayload {
    pub message: String,
    /// The log to open for this failure, once a variant exists that ran a script.
    pub log_path: Option<String>,
}

impl From<&AppError> for AppErrorPayload {
    fn from(error: &AppError) -> Self {
        let log_path = match error {
            AppError::ProbeFailed { log_path, .. } => log_path.clone(),
            _ => None,
        };
        AppErrorPayload {
            message: error.to_string(),
            log_path: log_path.map(|p| p.display().to_string()),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AppErrorPayload::from(self).serialize(serializer)
    }
}
