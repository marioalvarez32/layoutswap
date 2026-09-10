//! The Tauri command seam.
//!
//! Commands are shallow: deserialize arguments, call one function on [`App`], return its
//! `Result<T, AppError>`. Everything worth testing lives behind that call.
//! `src/tauri/commands.ts` is the only TypeScript that calls these by name.
//!
//! Commands that run a script are `async` and hop to a blocking thread so the window
//! stays responsive while PowerShell runs.

use std::sync::Arc;

use tauri::State;

use crate::app::App;
use crate::config::layouts::CaptureOutcome;
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
