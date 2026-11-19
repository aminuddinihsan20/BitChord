//! The library GameActivity loads (`libbitchord_android.so`): it starts BitChord in
//! the activity it is handed.
#![cfg(target_os = "android")]

use std::sync::atomic::{AtomicBool, Ordering};

use winit::platform::android::activity::input::{ImeOptions, InputType, TextInputAction, TextInputState, TextSpan};
use winit::platform::android::activity::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    // The desktop's default filter: our own crate at info, everything else warnings.
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("bitchord")
            .with_filter(android_logger::FilterBuilder::new().parse("warn,bitchord=info,bitchord_android=info").build()),
    );
    // Android drops stderr, where Rust reports a panic; and the app keeps running after
    // one on a worker thread (panic = "unwind"), so log every one.
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic on thread {:?}: {info}", std::thread::current().name());
        report(info);
    }));

    // winit makes one event loop per process, and the glue calls this again for every
    // new activity in the same process: that second run could never start. Ending the
    // process lets the next launch start a fresh one.
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        log::warn!("the activity was created again in a running process: exiting so the next launch starts afresh");
        std::process::exit(0);
    }

    // An app has no home directory, and std's temp dir (/data/local/tmp) isn't writable
    // by it: point both at the app's own storage, `files` and `cache` under its folder.
    if let Some(files) = app.internal_data_path() {
        let cache = files.parent().map_or_else(|| files.join("cache"), |app_dir| app_dir.join("cache"));
        let _ = std::fs::create_dir_all(&files);
        let _ = std::fs::create_dir_all(&cache);
        // SAFETY: the first and only run in this process (STARTED above), before the app
        // starts any thread of its own; nothing else reads the environment yet.
        unsafe {
            std::env::set_var("YTM_DATA_DIR", &files);
            std::env::set_var("TMPDIR", &cache);
        }
        log::info!("data in {}, temporary files in {}", files.display(), cache.display());
    } else {
        log::warn!("Android gave no app storage folder");
    }
    // The keyboard's action key (✓) arrives as an editor action, which winit 0.30 drops
    // too; "no action" makes it an Enter key, which the text fields take as "submit".
    app.set_ime_editor_info(InputType::TYPE_CLASS_TEXT, TextInputAction::None, ImeOptions::empty());
    // winit 0.30 drops what the on-screen keyboard types: read it from the input method
    // state and clear that, so each poll sees only what was typed since the last.
    let ime = app.clone();
    bitchord::platform::set_text_source(move || {
        let state = ime.text_input_state();
        if state.text.is_empty() {
            return None;
        }
        ime.set_text_input_state(TextInputState { text: String::new(), selection: TextSpan { start: 0, end: 0 }, compose_region: None });
        Some(state.text)
    });
    match bitchord::run_with(move |options| options.android_app = Some(app)) {
        Ok(()) => log::info!("closed"),
        Err(e) => log::error!("bitchord stopped: {e}"),
    }
    // The app closed itself (Quit, or "Restart now" in Settings): end the process too,
    // since its event loop can't be made again in it (see STARTED).
    std::process::exit(0);
}

// What MainActivity.java reports (JNI: package io.github.aminuddinihsan20.bitchord). The
// environment and class arguments are never used, so they stay untyped.
type Jni = *mut std::ffi::c_void;

#[unsafe(no_mangle)]
extern "system" fn Java_io_github_aminuddinihsan20_bitchord_MainActivity_nativeInsets(_env: Jni, _class: Jni, left: i32, top: i32, right: i32, bottom: i32) {
    bitchord::platform::set_insets_px(left, top, right, bottom);
}

#[unsafe(no_mangle)]
extern "system" fn Java_io_github_aminuddinihsan20_bitchord_MainActivity_nativeCanGoBack(_env: Jni, _class: Jni) -> u8 {
    u8::from(bitchord::platform::can_go_back())
}

#[unsafe(no_mangle)]
extern "system" fn Java_io_github_aminuddinihsan20_bitchord_MainActivity_nativeBack(_env: Jni, _class: Jni) {
    bitchord::platform::request_back();
}

#[unsafe(no_mangle)]
extern "system" fn Java_io_github_aminuddinihsan20_bitchord_MainActivity_nativeEnter(_env: Jni, _class: Jni) {
    bitchord::platform::press_enter();
}
