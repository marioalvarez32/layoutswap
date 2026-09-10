//! The script renderer: `render(layout, config) -> Script`.
//!
//! This is the deep module of the app. Every Windows PowerShell 5.1 quirk, escaping rule,
//! step ordering and logging convention lives here so every feature gets them for free
//! (ADR-0001, ADR-0004, `docs/windows-behaviour.md`). Templates under
//! `src-tauri/templates/*.ps1.tmpl` are included at compile time and rendered output is
//! checked against golden files under `src-tauri/tests/golden/`.
//!
//! The `Layout` type and the first templates arrive with the capture ticket; until then
//! the module declares only the output type so the runner seam can name it.

/// Rendered script text, ready to be written to a layout's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub text: String,
}
