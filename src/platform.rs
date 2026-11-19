//! What this platform offers, so Settings can leave out what it can't do.

/// A tray (or menu bar) icon to close to.
pub const HAS_TRAY: bool = cfg!(not(target_os = "android"));

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// A system dialog to pick a folder or files with.
pub const HAS_FILE_DIALOG: bool = cfg!(not(target_os = "android"));

/// Phone layout: bottom navigation and a mini player instead of the desktop frame.
/// Always on Android; elsewhere `YTM_PHONE=1` forces it, to see and test it on a desktop
/// (with `YTM_SIZE=412,900`).
pub fn phone() -> bool {
    static PHONE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *PHONE.get_or_init(|| phone_from(cfg!(target_os = "android"), std::env::var("YTM_PHONE").ok().as_deref()))
}

fn phone_from(android: bool, forced: Option<&str>) -> bool {
    android || forced.is_some_and(|v| !v.is_empty() && v != "0")
}

/// How far the system's bars, the notch and the gesture area reach into the window, in
/// points: the content keeps clear of them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

/// `px` (left, top, right, bottom) in pixels as insets in points at `ppp` pixels per
/// point; a negative size is none.
pub fn insets_to_points(px: [i32; 4], ppp: f32) -> Insets {
    let pts = |v: i32| (v as f32 / ppp).max(0.0);
    Insets { left: pts(px[0]), top: pts(px[1]), right: pts(px[2]), bottom: pts(px[3]) }
}

/// What Android's window insets last said, in pixels: left, top, right, bottom.
static INSETS_PX: [AtomicI32; 4] = [AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0)];

/// Android: the system bars' and the notch's size, from the activity (its window insets).
pub fn set_insets_px(left: i32, top: i32, right: i32, bottom: i32) {
    for (slot, v) in INSETS_PX.iter().zip([left, top, right, bottom]) {
        slot.store(v, Ordering::Relaxed);
    }
    wake();
}

/// The window's insets now: Android's, or elsewhere only the preview hook
/// `YTM_INSETS=top,bottom` (points) gives any.
pub fn insets(ctx: &egui::Context) -> Insets {
    let px = INSETS_PX.each_ref().map(|a| a.load(Ordering::Relaxed));
    if px != [0; 4] {
        return insets_to_points(px, ctx.pixels_per_point());
    }
    static PREVIEW: std::sync::OnceLock<Insets> = std::sync::OnceLock::new();
    *PREVIEW.get_or_init(|| {
        let v: Vec<f32> = std::env::var("YTM_INSETS").unwrap_or_default().split(',').filter_map(|p| p.trim().parse().ok()).collect();
        Insets { top: v.first().copied().unwrap_or(0.0), bottom: v.get(1).copied().unwrap_or(0.0), ..Insets::default() }
    })
}

static CAN_GO_BACK: AtomicBool = AtomicBool::new(false);
static BACK_REQUESTED: AtomicBool = AtomicBool::new(false);
static WAKE: std::sync::Mutex<Option<egui::Context>> = std::sync::Mutex::new(None);

/// Let the activity wake the UI (a request made while no frame is being drawn).
pub fn set_wake(ctx: &egui::Context) {
    let mut w = WAKE.lock().unwrap_or_else(|e| e.into_inner());
    if w.is_none() {
        *w = Some(ctx.clone());
    }
}

fn wake() {
    if let Some(ctx) = WAKE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        ctx.request_repaint();
    }
}

/// The app says whether Back has anything to close or go back to.
pub fn set_can_go_back(can: bool) {
    CAN_GO_BACK.store(can, Ordering::Relaxed);
}

/// Does Back have anything to do? (Android's back button asks: when not, the app goes to
/// the background instead.)
pub fn can_go_back() -> bool {
    CAN_GO_BACK.load(Ordering::Relaxed)
}

/// Android's back button was pressed.
pub fn request_back() {
    BACK_REQUESTED.store(true, Ordering::SeqCst);
    wake();
}

/// Was Back pressed since the last call?
pub fn take_back_request() -> bool {
    BACK_REQUESTED.swap(false, Ordering::SeqCst)
}

type TextSource = Box<dyn Fn() -> Option<String> + Send + Sync>;
static TEXT_SOURCE: std::sync::OnceLock<TextSource> = std::sync::OnceLock::new();

/// Android: where the on-screen keyboard's text can be read from (winit 0.30 drops it,
/// so the activity's crate registers a reader); called once.
pub fn set_text_source(source: impl Fn() -> Option<String> + Send + Sync + 'static) {
    let _ = TEXT_SOURCE.set(Box::new(source));
}

/// The text typed on the on-screen keyboard since the last call, if any.
pub fn take_typed_text() -> Option<String> {
    TEXT_SOURCE.get().and_then(|source| source())
}

static ENTER_PRESSED: AtomicBool = AtomicBool::new(false);

/// Android: the Enter key was pressed (winit reports it as the character `\n`, which
/// egui ignores, so the activity sends it here to become a real Enter key).
pub fn press_enter() {
    ENTER_PRESSED.store(true, Ordering::SeqCst);
    wake();
}

/// Was Enter pressed since the last call?
pub fn take_enter() -> bool {
    ENTER_PRESSED.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insets_in_pixels_become_points() {
        // A status bar of 80 px and a gesture area of 100 px at 2.625 px per point.
        let i = insets_to_points([0, 80, 0, 100], 2.625);
        assert!((i.top - 80.0 / 2.625).abs() < 0.01);
        assert!((i.bottom - 100.0 / 2.625).abs() < 0.01);
        assert_eq!((i.left, i.right), (0.0, 0.0));
    }

    #[test]
    fn no_insets_stay_zero() {
        assert_eq!(insets_to_points([0, 0, 0, 0], 2.625), Insets::default());
    }

    #[test]
    fn negative_insets_are_clamped_to_zero() {
        assert_eq!(insets_to_points([-20, -5, -1, -9], 2.625), Insets::default());
    }

    #[test]
    fn a_landscape_notch_gives_side_insets() {
        let i = insets_to_points([90, 0, 100, 0], 2.625);
        assert!((i.left - 90.0 / 2.625).abs() < 0.01);
        assert!((i.right - 100.0 / 2.625).abs() < 0.01);
    }

    #[test]
    fn the_desktop_is_not_a_phone_unless_asked() {
        assert!(!phone_from(false, None));
        assert!(phone_from(false, Some("1")));
        assert!(phone_from(true, None));
    }

    #[test]
    fn text_from_the_keyboard_comes_from_the_registered_source() {
        assert_eq!(take_typed_text(), None, "no source yet");
        let said = std::sync::atomic::AtomicBool::new(false);
        set_text_source(move || (!said.swap(true, Ordering::SeqCst)).then(|| "lo".to_owned()));
        assert_eq!(take_typed_text().as_deref(), Some("lo"));
        assert_eq!(take_typed_text(), None, "taken once");
    }

    #[test]
    fn an_enter_press_is_taken_once() {
        assert!(!take_enter());
        press_enter();
        assert!(take_enter());
        assert!(!take_enter());
    }

    #[test]
    fn a_back_request_is_taken_once() {
        assert!(!take_back_request());
        request_back();
        assert!(take_back_request());
        assert!(!take_back_request());
    }

    #[test]
    fn the_app_tells_whether_it_can_go_back() {
        set_can_go_back(true);
        assert!(can_go_back());
        set_can_go_back(false);
        assert!(!can_go_back());
    }
}
