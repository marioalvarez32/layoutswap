//! The Tauri command seam.
//!
//! Commands are shallow: deserialize arguments, call one function on [`App`], return its
//! `Result<T, AppError>`. Everything worth testing lives behind that call.
//! `src/tauri/commands.ts` is the only TypeScript that calls these by name.
//!
//! Commands that run a script or touch many files are `async` and hop to a blocking
//! thread so the window stays responsive.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::app::{App, SwitchEvent, SwitchResult};
use crate::config::layouts::{CaptureOutcome, Layout, ScriptStatus};
use crate::config::{Config, WindowSize};
use crate::error::AppError;
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
