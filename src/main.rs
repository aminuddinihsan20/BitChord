#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    // Child-process modes for the webview windows (see login.rs / turnstile.rs).
    if bitchord::run_child_window() {
        return Ok(());
    }
    bitchord::run_with(|_| {})
}
