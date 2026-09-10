//! The Tauri command seam.
//!
//! Commands are shallow: deserialize arguments, call one module function, return its
//! `Result<T, AppError>`. Everything worth testing lives in the module behind the command.
//! `src/tauri/commands.ts` is the only TypeScript that calls these by name.

use std::sync::{Mutex, PoisonError};

use tauri::State;

use crate::config::store::ConfigStore;
use crate::config::{Config, WindowSize};
use crate::error::AppError;

/// Managed state shared by every command.
pub struct AppState {
    /// The mutex serializes load-modify-save cycles; the store itself is stateless.
    pub store: Mutex<ConfigStore>,
}

impl AppState {
    fn store(&self) -> std::sync::MutexGuard<'_, ConfigStore> {
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> Result<Config, AppError> {
    state.store().load()
}

#[tauri::command]
pub fn save_window_size(state: State<'_, AppState>, size: WindowSize) -> Result<(), AppError> {
    state.store().update_window_size(size)
}
