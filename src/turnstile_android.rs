//! Android: the Better Lyrics API's Cloudflare Turnstile challenge, run by the activity in
//! an Android WebView (`Turnstile.java`) since a child-process window can't be opened
//! here. Like the desktop's (`turnstile.rs`), it normally passes unseen, and is shown
//! only if Cloudflare wants an interaction. Nothing here solves or forges the challenge.

use std::path::PathBuf;
use std::time::Duration;

/// Run the challenge; returns the Turnstile token.
pub fn obtain(_profile_dir: PathBuf) -> anyhow::Result<String> {
    crate::platform::call("turnstile", "", Duration::from_secs(130)).map_err(|e| anyhow::anyhow!("lyrics verification failed: {e}"))
}
