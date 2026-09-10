//! layoutswap backend.
//!
//! Module map (see `CODING_STANDARDS.md`, "Modules and seams"):
//!
//! - `commands`: the Tauri command seam. Thin: deserialize, call one module, map the error.
//! - `config`: the config store. `load() -> Config`, `save(Config)`, one JSON file with a
//!   schema version under the machine-local app root.
//! - `script::render`: Layout + Config -> Script text. The deep module.
//! - `script::run`: the `ScriptRunner` trait and the only place `powershell.exe` is spawned.
//! - `hardware`: `probe() -> Inventory` through a `ScriptRunner`.
//! - `shortcuts`: per-layout `.lnk` files and the scheduled task, through generated scripts.

pub mod commands;
pub mod config;
pub mod error;
pub mod hardware;
pub mod script;
pub mod shortcuts;

use std::sync::Mutex;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::AppState;
use crate::config::store::ConfigStore;

/// The window is created here, not in `tauri.conf.json`, so it can open at the size the
/// user left it (DESIGN.md, Q7) without a resize flash.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let store = ConfigStore::new(config::app_root()?);
            // A config that fails to load still gets a window: the renderer calls
            // `load_config` on start and shows that error where the user can read it.
            let size = store.load().map(|c| c.window).unwrap_or_default();

            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("layoutswap")
                .inner_size(f64::from(size.width), f64::from(size.height))
                .min_inner_size(
                    f64::from(config::MIN_WINDOW_SIZE.width),
                    f64::from(config::MIN_WINDOW_SIZE.height),
                )
                .resizable(true)
                .build()?;

            app.manage(AppState {
                store: Mutex::new(store),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_window_size
        ])
        .run(tauri::generate_context!())
        .expect("layoutswap failed to start");
}
