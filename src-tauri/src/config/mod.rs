//! The config store: typed [`Config`], one JSON file under the app root, a schema version
//! and forward migrations.
//!
//! Nothing roams. Layouts will hold device paths and GPU identities, so the whole app root
//! is machine-bound under `%LOCALAPPDATA%\layoutswap`. Export config is the way to move
//! between machines.

pub mod capabilities;
pub mod layouts;
pub mod store;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use capabilities::Capabilities;
use layouts::Layout;

/// The schema version this build writes. Reading a newer version is an error; older
/// versions are migrated forward in [`store::migrate`].
pub const SCHEMA_VERSION: u32 = 3;

/// The size the window opens at on first run (DESIGN.md, Q7).
pub const DEFAULT_WINDOW_SIZE: WindowSize = WindowSize {
    width: 1280,
    height: 860,
};

/// The smallest window the layout still fits in: the 212 px sidebar plus a usable content
/// column. Sizes below this are never persisted because the window cannot reach them.
pub const MIN_WINDOW_SIZE: WindowSize = WindowSize {
    width: 900,
    height: 600,
};

/// Everything layoutswap remembers between launches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Config {
    pub schema_version: u32,
    /// The window size the user left, in logical pixels.
    pub window: WindowSize,
    /// The user's alias per monitor, keyed by device path (ADR-0005).
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    /// What each monitor declared it can do, keyed by device path, as last read
    /// (CONTEXT.md: Capabilities). Entries for monitors this machine has never seen
    /// stay, so an import from another machine keeps them.
    #[serde(default)]
    pub capabilities: BTreeMap<String, Capabilities>,
    #[serde(default)]
    pub layouts: Vec<Layout>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            schema_version: SCHEMA_VERSION,
            window: DEFAULT_WINDOW_SIZE,
            aliases: BTreeMap::new(),
            capabilities: BTreeMap::new(),
            layouts: Vec::new(),
        }
    }
}

/// A window size in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSize {
    fn default() -> Self {
        DEFAULT_WINDOW_SIZE
    }
}

/// The machine-local folder every file the app writes lives under.
pub fn app_root() -> Result<PathBuf, AppError> {
    std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(|local| PathBuf::from(local).join("layoutswap"))
        .ok_or(AppError::AppRootUnavailable)
}
