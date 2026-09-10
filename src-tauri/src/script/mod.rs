//! Generated scripts: rendering them and running them.
//!
//! Script text never crosses the Tauri command seam in either direction (ADR-0003).
//! `render` produces it, `run` executes it from a file on disk.

pub mod render;
pub mod run;
