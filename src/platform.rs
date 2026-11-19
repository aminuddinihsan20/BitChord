//! What this platform offers, so Settings can leave out what it can't do.

/// A tray (or menu bar) icon to close to.
pub const HAS_TRAY: bool = cfg!(not(target_os = "android"));

/// A system dialog to pick a folder or files with.
pub const HAS_FILE_DIALOG: bool = cfg!(not(target_os = "android"));
