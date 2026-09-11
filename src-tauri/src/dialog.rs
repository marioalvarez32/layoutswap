//! The one place a native file dialog is shown. Save dialogs default to the Desktop
//! (DESIGN.md, Save diagnostics). Nothing here is unit-tested: the callers take the
//! chosen path, so everything after the dialog is.

use std::path::PathBuf;

/// The user's Desktop as Windows knows it, which follows a OneDrive redirection.
pub fn desktop_dir() -> Option<PathBuf> {
    dirs::desktop_dir().filter(|desktop| desktop.is_dir())
}

/// Asks where to save a file named `file_name` of the given kind. `None` when the user
/// cancels.
pub fn ask_where_to_save(
    title: &str,
    file_name: &str,
    kind: &str,
    extension: &str,
) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(file_name)
        .add_filter(kind, &[extension]);
    if let Some(desktop) = desktop_dir() {
        dialog = dialog.set_directory(desktop);
    }
    dialog.save_file()
}

/// Asks which file of the given kind to open. `None` when the user cancels.
pub fn ask_which_file_to_open(title: &str, kind: &str, extension: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .add_filter(kind, &[extension]);
    if let Some(desktop) = desktop_dir() {
        dialog = dialog.set_directory(desktop);
    }
    dialog.pick_file()
}
