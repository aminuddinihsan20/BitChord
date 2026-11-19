//! Android has no tray icon: `tray.rs`'s API, refusing to make one (Settings hides
//! "Close to tray", see `crate::platform::HAS_TRAY`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Show,
    Quit,
}

pub struct Tray;

impl Tray {
    pub fn new(_icon: &crate::appicon::Variant, _wake: impl Fn() + Send + Sync + Clone + 'static) -> anyhow::Result<Self> {
        anyhow::bail!("Android has no tray icon")
    }
}

/// Clicks since the last call: never any.
pub fn take() -> Vec<Cmd> {
    Vec::new()
}
