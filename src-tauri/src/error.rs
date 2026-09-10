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
        AppErrorPayload {
            message: error.to_string(),
            log_path: None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AppErrorPayload::from(self).serialize(serializer)
    }
}
