//! Android: the app behind a lock, so a second thread can keep the music going while
//! the activity is away. Then the window is gone and eframe draws nothing, which also
//! stops `App::logic`, the one place that starts the next song; this thread does that
//! part of it (`App::background_tick`) until the activity is back.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::App;

struct Held(App);

// SAFETY: the app is used by one thread at a time (the lock), and what it holds that is
// tied to the window's thread (the GL context, the window) is never touched by the
// background tick: that only handles the player's queue, audio events and requests.
unsafe impl Send for Held {}

pub struct Shared(Arc<Mutex<Held>>);

impl Shared {
    pub fn new(app: App) -> Self {
        let held = Arc::new(Mutex::new(Held(app)));
        let (pump, start) = (held.clone(), Instant::now());
        let spawned = std::thread::Builder::new().name("player-background".into()).spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(400));
                if !crate::platform::in_background() {
                    continue;
                }
                // The window's thread holds the lock while it draws; away, nothing does.
                let guard = pump.try_lock();
                if let Ok(mut app) = guard {
                    app.0.background_tick(start.elapsed().as_secs_f64());
                }
            }
        });
        if let Err(e) = spawned {
            log::error!("the background player thread didn't start: {e}");
        }
        Self(held)
    }

    fn app(&self) -> MutexGuard<'_, Held> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl eframe::App for Shared {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app().0.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app().0.ui(ui, frame);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app().0.raw_input_hook(ctx, raw);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.app().0.save(storage);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.app().0.on_exit(gl);
    }
}
