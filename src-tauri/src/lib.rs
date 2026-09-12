//! layoutswap backend.
//!
//! Module map (see `CODING_STANDARDS.md`, "Modules and seams"):
//!
//! - `commands`: the Tauri command seam. Thin: deserialize, call one function on `App`.
//! - `app`: the composition root: config store, script runner, the latest probe and the
//!   running switch under one app root. Capture (probe plus store) and the switch
//!   (`app::switch`) are orchestrated here.
//! - `config`: the config store and the layout types. `load() -> Config`, `save(Config)`,
//!   one JSON file with a schema version under the machine-local app root.
//! - `script::render`: templates -> Script text. The deep module.
//! - `script::run`: the `ScriptRunner` trait and the only place `powershell.exe` is spawned.
//! - `script::lock`: the `switch.lock` file a running switch script holds.
//! - `dialog`: the one place a native file dialog is shown.
//! - `hardware`: `probe() -> Inventory` through a `ScriptRunner`.
//! - `shortcuts`: per-layout `.lnk` files and the scheduled task, through generated scripts.

pub mod app;
pub mod commands;
pub mod config;
pub mod dialog;
pub mod error;
pub mod hardware;
pub mod script;
pub mod shortcuts;

use std::sync::Arc;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::app::App;
use crate::commands::AppState;
use crate::script::run::PowerShellRunner;

/// The window is created here, not in `tauri.conf.json`, so it can open at the size the
/// user left it (DESIGN.md, Q7) without a resize flash.
pub fn run() {
    tauri::Builder::default()
        .setup(|tauri_app| {
            let app = App::new(config::app_root()?, Arc::new(PowerShellRunner));
            // The probe on disk always matches the running app: rewrite it every start,
            // and bring every layout's switch script up to this template.
            let rendered_at = chrono::Local::now().to_rfc3339();
            app.write_probe_script(&rendered_at)?;
            app.write_capabilities_script(&rendered_at)?;
            if let Err(error) = app.regenerate_stale_scripts() {
                // The window still opens; the detail shows the script as stale.
                eprintln!("layoutswap: could not regenerate stale scripts: {error}");
            }
            // A config that fails to load still gets a window: the renderer calls
            // `load_config` on start and shows that error where the user can read it.
            let size = app.load_config().map(|c| c.window).unwrap_or_default();

            WebviewWindowBuilder::new(tauri_app, "main", WebviewUrl::default())
                .title("layoutswap")
                .inner_size(f64::from(size.width), f64::from(size.height))
                .min_inner_size(
                    f64::from(config::MIN_WINDOW_SIZE.width),
                    f64::from(config::MIN_WINDOW_SIZE.height),
                )
                .resizable(true)
                .build()?;

            tauri_app.manage(AppState { app: Arc::new(app) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_window_size,
            commands::probe,
            commands::set_alias,
            commands::read_capabilities,
            commands::capture_layout,
            commands::save_layout,
            commands::input_sources,
            commands::script_states,
            commands::regenerate_script,
            commands::open_script,
            commands::switch_layout,
            commands::cancel_switch,
            commands::open_log,
            commands::open_display_settings,
            commands::save_diagnostics,
            commands::export_config,
            commands::import_config
        ])
        .run(tauri::generate_context!())
        .expect("layoutswap failed to start");
}
