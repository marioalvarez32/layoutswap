//! The Tauri command seam.
//!
//! Commands are shallow: deserialize arguments, call one function on [`App`], return its
//! `Result<T, AppError>`. Everything worth testing lives behind that call.
//! `src/tauri/commands.ts` is the only TypeScript that calls these by name.
//!
//! Commands that run a script or touch many files are `async` and hop to a blocking
//! thread so the window stays responsive.

use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::app::{App, SwitchEvent, SwitchResult};
use crate::config::capabilities::Capabilities;
use crate::config::layouts::{CaptureOutcome, Layout, LayoutEdits, ScriptStatus};
use crate::config::{Config, WindowSize};
use crate::error::AppError;
use crate::hardware::input_source::InputSource;
use crate::hardware::Inventory;

/// Managed state shared by every command.
pub struct AppState {
    pub app: Arc<App>,
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> Result<Config, AppError> {
    state.app.load_config()
}

#[tauri::command]
pub fn save_window_size(state: State<'_, AppState>, size: WindowSize) -> Result<(), AppError> {
    state.app.update_window_size(size)
}

#[tauri::command]
pub async fn probe(state: State<'_, AppState>) -> Result<Inventory, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || app.probe()).await
}

/// Re-check: reads every Active monitor's capabilities and returns the stored map.
#[tauri::command]
pub async fn read_capabilities(
    state: State<'_, AppState>,
) -> Result<BTreeMap<String, Capabilities>, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || app.read_capabilities()).await
}

/// First sight: reads capabilities only when the latest probe shows an Active
/// monitor without an entry; the renderer calls this after a probe, without waiting.
#[tauri::command]
pub async fn read_missing_capabilities(
    state: State<'_, AppState>,
) -> Result<Option<BTreeMap<String, Capabilities>>, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || app.read_missing_capabilities()).await
}

#[tauri::command]
pub async fn capture_layout(
    state: State<'_, AppState>,
    name: String,
    replace_id: Option<String>,
) -> Result<CaptureOutcome, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || app.capture(&name, replace_id.as_deref())).await
}

#[tauri::command]
pub async fn save_layout(
    state: State<'_, AppState>,
    layout_id: String,
    edits: LayoutEdits,
) -> Result<Layout, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || app.save_layout(&layout_id, edits)).await
}

/// The fixed input source table the step editor offers.
#[tauri::command]
pub fn input_sources(state: State<'_, AppState>) -> Result<Vec<InputSource>, AppError> {
    Ok(state.app.input_sources())
}

#[tauri::command]
pub fn script_states(state: State<'_, AppState>) -> Result<Vec<ScriptStatus>, AppError> {
    state.app.script_states()
}

#[tauri::command]
pub async fn regenerate_script(
    state: State<'_, AppState>,
    layout_id: String,
) -> Result<Layout, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || app.regenerate_script(&layout_id)).await
}

#[tauri::command]
pub fn open_script(state: State<'_, AppState>, layout_id: String) -> Result<(), AppError> {
    state.app.open_script(&layout_id)
}

/// The event a running switch emits to the window; `src/tauri/commands.ts` listens by
/// this name. The payload is a [`SwitchEvent`].
pub const SWITCH_EVENT: &str = "switch-event";

/// Stays pending until the script exits; progress arrives as [`SWITCH_EVENT`] events.
#[tauri::command]
pub async fn switch_layout(
    handle: AppHandle,
    state: State<'_, AppState>,
    layout_id: String,
) -> Result<SwitchResult, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || {
        app.switch(&layout_id, &|event: SwitchEvent| {
            // A window that has gone away has nobody to tell; the switch still finishes.
            let _ = handle.emit(SWITCH_EVENT, &event);
        })
    })
    .await
}

#[tauri::command]
pub fn cancel_switch(state: State<'_, AppState>) -> Result<(), AppError> {
    state.app.cancel_switch()
}

#[tauri::command]
pub fn open_log(state: State<'_, AppState>, layout_id: String) -> Result<(), AppError> {
    state.app.open_log(&layout_id)
}

#[tauri::command]
pub fn open_display_settings(state: State<'_, AppState>) -> Result<(), AppError> {
    state.app.open_display_settings()
}

/// Asks where to save the config, then copies it there. Resolves with the path, or
/// null when the user cancelled the dialog.
#[tauri::command]
pub async fn export_config(state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || {
        app.export_config(|file_name| {
            crate::dialog::ask_where_to_save("Export config", file_name, "layoutswap config", "json")
        })
        .map(|path| path.map(|p| p.display().to_string()))
    })
    .await
}

/// Asks which file to import, then replaces the config with it. Resolves with the new
/// config, or null when the user cancelled the dialog.
#[tauri::command]
pub async fn import_config(state: State<'_, AppState>) -> Result<Option<Config>, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || {
        app.import_config(|| crate::dialog::ask_which_file_to_open("Import config", "layoutswap config", "json"))
    })
    .await
}

/// Asks where to save, then writes the zip there. Resolves with the path, or null when
/// the user cancelled the dialog.
#[tauri::command]
pub async fn save_diagnostics(
    state: State<'_, AppState>,
    layout_id: String,
) -> Result<Option<String>, AppError> {
    let app = Arc::clone(&state.app);
    blocking(move || {
        app.save_diagnostics(&layout_id, |file_name| {
            crate::dialog::ask_where_to_save("Save diagnostics", file_name, "Zip archive", "zip")
        })
        .map(|path| path.map(|p| p.display().to_string()))
    })
    .await
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or_else(|join_error| {
            Err(AppError::Internal {
                detail: join_error.to_string(),
            })
        })
}
