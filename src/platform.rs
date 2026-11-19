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

/// Which service the Android sign-in activity is asked to sign in to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginSite {
    Google,
    Spotify,
}

impl LoginSite {
    /// The number the activity (Java) knows it by.
    pub fn code(self) -> i32 {
        match self {
            LoginSite::Google => 1,
            LoginSite::Spotify => 2,
        }
    }

    pub fn from_code(code: i32) -> Option<LoginSite> {
        match code {
            1 => Some(LoginSite::Google),
            2 => Some(LoginSite::Spotify),
            _ => None,
        }
    }
}

/// What the activity answers when the person closes the sign-in without signing in.
pub const LOGIN_CANCELLED: &str = "cancelled";

static LOGIN_REQUEST: AtomicI32 = AtomicI32::new(0);
static LOGIN_REPLY: std::sync::Mutex<Option<std::sync::mpsc::Sender<Result<String, String>>>> = std::sync::Mutex::new(None);

/// Android: ask the activity to open the sign-in page and wait for what it finds: the
/// session cookie, or why not. The request is withdrawn if nothing answers in `timeout`.
pub fn request_login(site: LoginSite, timeout: std::time::Duration) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    *LOGIN_REPLY.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
    LOGIN_REQUEST.store(site.code(), Ordering::SeqCst);
    let answer = rx.recv_timeout(timeout).unwrap_or_else(|_| Err("sign-in timed out".to_owned()));
    LOGIN_REQUEST.store(0, Ordering::SeqCst);
    *LOGIN_REPLY.lock().unwrap_or_else(|e| e.into_inner()) = None;
    answer
}

/// The activity polls this: a sign-in asked for since the last call.
pub fn take_login_request() -> Option<LoginSite> {
    LoginSite::from_code(LOGIN_REQUEST.swap(0, Ordering::SeqCst))
}

/// The activity's answer to [`request_login`].
pub fn finish_login(answer: Result<String, String>) {
    if let Some(tx) = LOGIN_REPLY.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = tx.send(answer);
    }
}

static BACKGROUND: AtomicBool = AtomicBool::new(false);

/// Android: the activity left the screen (stopped) or came back. While it is away the
/// window is gone and no frame is drawn, so the player's bookkeeping runs without one.
pub fn set_background(away: bool) {
    BACKGROUND.store(away, Ordering::SeqCst);
}

pub fn in_background() -> bool {
    BACKGROUND.load(Ordering::SeqCst)
}

/// What the media notification shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    /// The artwork's address; the notification fetches it.
    pub cover: String,
    pub playing: bool,
    pub duration_ms: i64,
    pub position_ms: i64,
}

const FIELD: char = '\u{1f}';

impl NowPlaying {
    /// One line for the activity to read: fields joined by U+001F; empty when nothing is
    /// loaded. A title can't break it: the separator and newlines in text become spaces.
    pub fn to_line(&self) -> String {
        if self.title.is_empty() {
            return String::new();
        }
        let clean = |t: &str| t.replace([FIELD, '\n', '\r'], " ");
        [clean(&self.title), clean(&self.artist), clean(&self.cover), u8::from(self.playing).to_string(), self.duration_ms.to_string(), self.position_ms.to_string()].join(&FIELD.to_string())
    }

    pub fn from_line(line: &str) -> Option<NowPlaying> {
        let mut f = line.split(FIELD);
        let (title, artist, cover, playing, duration, position) = (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
        Some(NowPlaying { title: title.to_owned(), artist: artist.to_owned(), cover: cover.to_owned(), playing: playing == "1", duration_ms: duration.parse().ok()?, position_ms: position.parse().ok()? })
    }
}

static NOW_PLAYING: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Publish what plays, for the media notification (cheap when unchanged).
pub fn set_now_playing(now: &NowPlaying) {
    let line = now.to_line();
    let mut slot = NOW_PLAYING.lock().unwrap_or_else(|e| e.into_inner());
    if *slot != line {
        *slot = line;
    }
}

/// The line the activity polls (see [`NowPlaying::to_line`]).
pub fn now_playing_line() -> String {
    NOW_PLAYING.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// A button on the notification, the lock screen or a headset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    /// To this position, in milliseconds.
    Seek(i64),
}

impl MediaCommand {
    /// The code and argument the activity (Java) sends.
    pub fn to_code(self) -> (i32, i64) {
        match self {
            MediaCommand::Play => (1, 0),
            MediaCommand::Pause => (2, 0),
            MediaCommand::Toggle => (3, 0),
            MediaCommand::Next => (4, 0),
            MediaCommand::Previous => (5, 0),
            MediaCommand::Seek(ms) => (6, ms),
        }
    }

    fn from_code(code: i32, arg: i64) -> Option<MediaCommand> {
        Some(match code {
            1 => MediaCommand::Play,
            2 => MediaCommand::Pause,
            3 => MediaCommand::Toggle,
            4 => MediaCommand::Next,
            5 => MediaCommand::Previous,
            6 => MediaCommand::Seek(arg),
            _ => return None,
        })
    }
}

static MEDIA_COMMANDS: std::sync::Mutex<Vec<MediaCommand>> = std::sync::Mutex::new(Vec::new());

/// The activity: a notification button was pressed.
pub fn push_media_command(code: i32, arg: i64) {
    if let Some(c) = MediaCommand::from_code(code, arg) {
        MEDIA_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
        wake();
    }
}

/// The buttons pressed since the last call, in order.
pub fn take_media_commands() -> Vec<MediaCommand> {
    std::mem::take(&mut *MEDIA_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The login tests share the login statics.
    static LOGIN: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    fn a_login_request_reaches_the_activity_and_its_answer_comes_back() {
        let _one = LOGIN.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(take_login_request(), None);
        let waiting = std::thread::spawn(|| request_login(LoginSite::Spotify, std::time::Duration::from_secs(5)));
        // The activity polls for the request, then reports.
        let site = loop {
            if let Some(site) = take_login_request() {
                break site;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(site, LoginSite::Spotify);
        assert_eq!(take_login_request(), None, "taken once");
        finish_login(Ok("sp_dc-value".to_owned()));
        assert_eq!(waiting.join().unwrap(), Ok("sp_dc-value".to_owned()));
    }

    #[test]
    fn a_login_nobody_answers_times_out_and_is_withdrawn() {
        let _one = LOGIN.lock().unwrap_or_else(|e| e.into_inner());
        let r = request_login(LoginSite::Google, std::time::Duration::from_millis(50));
        assert!(r.is_err(), "{r:?}");
        assert_eq!(take_login_request(), None, "nothing is left for the activity to find");
    }

    #[test]
    fn a_login_site_has_a_code_for_the_activity_and_back() {
        for site in [LoginSite::Google, LoginSite::Spotify] {
            assert_eq!(LoginSite::from_code(site.code()), Some(site));
        }
        assert_eq!(LoginSite::from_code(0), None);
    }

    #[test]
    fn what_is_playing_is_told_to_the_notification_as_one_line() {
        let now = NowPlaying { title: "Tabun".into(), artist: "YOASOBI".into(), cover: "https://x/y.jpg".into(), playing: true, duration_ms: 261_000, position_ms: 15_500 };
        let line = now.to_line();
        assert_eq!(NowPlaying::from_line(&line), Some(now));
    }

    #[test]
    fn nothing_playing_is_an_empty_line() {
        assert_eq!(NowPlaying::default().to_line(), "");
        assert_eq!(NowPlaying::from_line(""), None);
    }

    #[test]
    fn a_title_with_the_separator_in_it_is_still_one_line() {
        let now = NowPlaying { title: "a\u{1f}b\nc".into(), ..NowPlaying::default() };
        let back = NowPlaying::from_line(&now.to_line()).unwrap();
        assert!(!back.title.contains('\u{1f}') && !back.title.contains('\n'), "{:?}", back.title);
    }

    #[test]
    fn media_commands_cross_the_activity_as_numbers_and_back_in_order() {
        assert!(take_media_commands().is_empty());
        for c in [MediaCommand::Toggle, MediaCommand::Next, MediaCommand::Seek(42_000), MediaCommand::Previous, MediaCommand::Play, MediaCommand::Pause] {
            let (code, arg) = c.to_code();
            push_media_command(code, arg);
        }
        push_media_command(99, 0);
        assert_eq!(
            take_media_commands(),
            [MediaCommand::Toggle, MediaCommand::Next, MediaCommand::Seek(42_000), MediaCommand::Previous, MediaCommand::Play, MediaCommand::Pause],
            "an unknown code is dropped"
        );
        assert!(take_media_commands().is_empty());
    }

    #[test]
    fn the_app_knows_when_it_is_in_the_background() {
        set_background(true);
        assert!(in_background());
        set_background(false);
        assert!(!in_background());
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
