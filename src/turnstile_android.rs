//! Android can't run `turnstile.rs`'s webview window in a child process, so the Better
//! Lyrics provider's challenge isn't available; the other lyrics providers still are.

use std::path::PathBuf;

pub fn obtain(_profile_dir: PathBuf) -> anyhow::Result<String> {
    anyhow::bail!("lyrics verification needs a webview window, which Android doesn't have here yet")
}
