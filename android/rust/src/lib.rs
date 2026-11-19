//! The library GameActivity loads (`libbitchord_android.so`): it starts BitChord in
//! the activity it is handed.
#![cfg(target_os = "android")]

use winit::platform::android::activity::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("bitchord"));
    if let Err(e) = bitchord::run_with(move |options| options.android_app = Some(app)) {
        log::error!("bitchord stopped: {e}");
    }
}
