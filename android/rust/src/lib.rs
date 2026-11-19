//! The library GameActivity loads (`libbitchord_android.so`): it starts BitChord in
//! the activity it is handed.
#![cfg(target_os = "android")]

use winit::platform::android::activity::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("bitchord"));
    // An app has no home directory, and std's temp dir (/data/local/tmp) isn't writable
    // by it: point both at the app's own storage, `files` and `cache` under its folder.
    if let Some(files) = app.internal_data_path() {
        let cache = files.parent().map_or_else(|| files.join("cache"), |app_dir| app_dir.join("cache"));
        let _ = std::fs::create_dir_all(&files);
        let _ = std::fs::create_dir_all(&cache);
        // SAFETY: nothing else reads the environment yet; the app's threads start below.
        unsafe {
            std::env::set_var("YTM_DATA_DIR", &files);
            std::env::set_var("TMPDIR", &cache);
        }
        log::info!("data in {}, temporary files in {}", files.display(), cache.display());
    }
    if let Err(e) = bitchord::run_with(move |options| options.android_app = Some(app)) {
        log::error!("bitchord stopped: {e}");
    }
}
