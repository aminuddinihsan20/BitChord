//! The phone frame: a top row (back and account), bottom navigation with four tabs, and
//! a mini player above it that opens Now Playing. The pages are the desktop's; only what
//! surrounds them differs (see `platform::phone`).

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

use super::widgets::{self, named, text_at};
use super::{Action, App, LibTab, Page, glyphs};
use crate::platform::Insets;
use crate::theme::{self, icon};

/// The bottom navigation's tabs. A page opened from one (an album from Library) keeps
/// that tab lit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Search,
    Library,
    Downloads,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Home, Tab::Search, Tab::Library, Tab::Downloads];

    /// The tab `page` belongs to; pages that belong to none (album, artist, playlist,
    /// settings, transfer) stay with `last`, the tab they were opened from.
    pub fn of(page: &Page, last: Tab) -> Tab {
        match page {
            Page::Home | Page::New => Tab::Home,
            Page::Search(_) => Tab::Search,
            Page::Library(_) => Tab::Library,
            Page::Downloads => Tab::Downloads,
            Page::Album(_) | Page::Playlist(_) | Page::Artist(_) | Page::Settings | Page::Transfer => last,
        }
    }

    /// The page a tab opens.
    pub fn page(self) -> Page {
        match self {
            Tab::Home => Page::Home,
            Tab::Search => Page::Search(String::new()),
            Tab::Library => Page::Library(LibTab::Playlists),
            Tab::Downloads => Page::Downloads,
        }
    }

    /// Is `page` a tab's own page (so there's nothing to go back to)?
    pub fn is_root(page: &Page) -> bool {
        matches!(page, Page::Home | Page::Search(_) | Page::Library(_) | Page::Downloads)
    }

    fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Search => "Search",
            Tab::Library => "Library",
            Tab::Downloads => "Downloads",
        }
    }

    fn glyph(self) -> &'static [u8] {
        match self {
            Tab::Home => glyphs::HOME,
            Tab::Search => glyphs::SEARCH,
            Tab::Library => glyphs::LIBRARY,
            Tab::Downloads => glyphs::DOWNLOAD,
        }
    }
}

/// The bottom navigation's height above the gesture area, and the mini player's.
pub const NAV_H: f32 = 64.0;
pub const MINI_H: f32 = 64.0;
/// The top row: the back button on the left, the account on the right.
pub const TOP_H: f32 = 48.0;
/// The gap between the screen's side and the pages.
pub const SIDE: f32 = 16.0;

/// Draw the whole phone frame and the current page in it.
pub fn frame(app: &mut App, ui: &mut Ui, now: f64, screen: Rect) {
    let ctx = ui.ctx().clone();
    let insets = crate::platform::insets(&ctx);
    app.phone_tab = Tab::of(&app.page, app.phone_tab);

    // While typing the keyboard takes the bottom (its height is in `insets.bottom`): the
    // navigation and the mini player give way to it.
    let typing = ctx.egui_wants_keyboard_input();
    // Always added, only emptied: a panel that comes and goes shifts the automatic ids
    // of the widgets after it, and the text field being typed in would lose its focus.
    egui::Panel::bottom("phone-nav")
        .resizable(false)
        .exact_size(if typing { 0.0 } else { NAV_H + insets.bottom })
        .frame(egui::Frame::new().fill(theme::panel(0.94)))
        .show_separator_line(false)
        .show(ui, |ui| {
            if !typing {
                nav_bar(app, ui, insets);
            }
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::new().inner_margin(egui::Margin {
            left: (SIDE + insets.left) as i8,
            right: (SIDE + insets.right) as i8,
            top: (insets.top as i8).max(0),
            bottom: if typing { insets.bottom as i8 } else { 0 },
        }))
        .show(ui, |ui| {
            top_row(app, ui);
            super::pages::page(app, ui, now);
        });

    if app.current().is_some() && !typing {
        let nav_top = screen.bottom() - NAV_H - insets.bottom;
        let strip = Rect::from_min_size(
            pos2(screen.left() + 8.0 + insets.left, nav_top - MINI_H - 6.0),
            vec2(screen.width() - 16.0 - insets.left - insets.right, MINI_H),
        );
        egui::Area::new(ui.scope_id().with("phone-mini"))
            .fixed_pos(strip.min)
            .order(egui::Order::Middle)
            .show(&ctx, |ui| {
                ui.scope_builder(egui::UiBuilder::new().max_rect(strip), |ui| mini_player(app, ui, strip));
            });
    }
}

/// The top row: back when the page isn't a tab's own, the account on the right.
fn top_row(app: &mut App, ui: &mut Ui) {
    let panel = ui.max_rect();
    if !Tab::is_root(&app.page) {
        let r = Rect::from_min_size(pos2(panel.left() - 8.0, panel.top()), Vec2::splat(TOP_H));
        let resp = named(ui.interact(r, ui.scope_id().with("phone-back"), Sense::click()), "Back");
        widgets::icon_at(ui, r.center(), icon::CHEVRON_LEFT, 26.0, theme::text());
        if resp.clicked() {
            app.nav_back();
        }
    }
    super::shell::top_bar(app, ui);
    ui.add_space(TOP_H - 2.0);
}

fn nav_bar(app: &mut App, ui: &mut Ui, insets: Insets) {
    let full = ui.max_rect();
    ui.painter().hline(full.x_range(), full.top(), Stroke::new(1.0, theme::glass(0.1)));
    let row = Rect::from_min_max(pos2(full.left() + insets.left, full.top()), pos2(full.right() - insets.right, full.top() + NAV_H));
    let w = row.width() / Tab::ALL.len() as f32;
    let accent = app.accent_color();
    for (i, tab) in Tab::ALL.into_iter().enumerate() {
        let cell = Rect::from_min_size(pos2(row.left() + w * i as f32, row.top()), vec2(w, NAV_H));
        let resp = named(ui.interact(cell, ui.scope_id().with(("phone-tab", i)), Sense::click()), tab.label());
        let on = app.phone_tab == tab;
        let tint = if on { accent } else { theme::text_dim() };
        let k = theme::anim_bool(ui.ctx(), resp.id.with("on"), on, 0.15);
        if k > 0.0 {
            // The lit tab's pill behind its icon.
            let pill = Rect::from_center_size(pos2(cell.center().x, cell.top() + 22.0), vec2(56.0, 30.0));
            ui.painter().rect_filled(pill, CornerRadius::same(15), theme::with_alpha(accent, 0.18 * k));
        }
        glyphs::draw(ui, pos2(cell.center().x, cell.top() + 22.0), 24.0, tab.glyph(), tint);
        ui.painter().text(pos2(cell.center().x, cell.top() + 48.0), Align2::CENTER_CENTER, tab.label(), if on { theme::semibold(12.0) } else { theme::regular(12.0) }, tint);
        if resp.clicked() {
            app.actions.push(Action::Go(tab.page()));
        }
    }
}

/// The song playing, with play/pause and next; a tap anywhere else opens Now Playing.
fn mini_player(app: &mut App, ui: &mut Ui, rect: Rect) {
    let Some(track) = app.current().cloned() else { return };
    let playing = app.playing();
    let accent = app.accent_color();
    let painter = ui.painter();
    widgets::soft_shadow(ui, rect, 16.0, 16.0, 0.4);
    painter.rect_filled(rect, CornerRadius::same(16), theme::panel(0.92));
    painter.rect_stroke(rect, CornerRadius::same(16), Stroke::new(1.0, theme::glass(0.14)), egui::StrokeKind::Inside);

    let open = named(ui.interact(rect, ui.scope_id().with("mini-open"), Sense::click()), "Open now playing");
    if open.clicked() {
        app.np_open = true;
    }

    let cover = Rect::from_center_size(pos2(rect.left() + 12.0 + 22.0, rect.center().y - 1.0), Vec2::splat(44.0));
    widgets::paint_cover(ui, cover, &track.thumbs, 8.0, 1.0);
    let btns = 2.0 * 48.0;
    let text_l = cover.right() + 12.0;
    let text_w = (rect.right() - btns - 4.0 - text_l).max(20.0);
    text_at(ui, pos2(text_l, rect.center().y - 19.0), &track.title, theme::semibold(15.0), theme::text(), text_w);
    text_at(ui, pos2(text_l, rect.center().y + 2.0), &track.artist_line(), theme::regular(13.0), theme::text_dim(), text_w);

    let tap = |ui: &mut Ui, x: f32, key: &str, name: &str| {
        let r = Rect::from_center_size(pos2(x, rect.center().y - 1.0), Vec2::splat(48.0));
        named(ui.interact(r, ui.scope_id().with(("mini", key)), Sense::click()), name)
    };
    let r = tap(ui, rect.right() - 4.0 - 72.0, "play", if playing { "Pause" } else { "Play" });
    glyphs::draw(ui, r.rect.center(), 30.0, if playing { glyphs::PAUSE } else { glyphs::PLAY }, theme::text());
    if r.clicked() {
        app.toggle();
    }
    let r = tap(ui, rect.right() - 4.0 - 24.0, "next", "Next");
    glyphs::draw(ui, r.rect.center(), 28.0, glyphs::NEXT, theme::text());
    if r.clicked() {
        app.next();
    }

    // Progress: a thin line along the bottom edge.
    let status = app.audio.status.clone();
    let total = status.duration().map(|d| d.as_secs_f32()).or(track.duration.map(|d| d as f32)).unwrap_or(0.0);
    if total > 0.0 {
        let frac = (status.position_smooth().as_secs_f32() / total).clamp(0.0, 1.0);
        let y = rect.bottom() - 2.0;
        let line = Rect::from_min_max(pos2(rect.left() + 14.0, y - 1.5), pos2(rect.right() - 14.0, y));
        ui.painter().rect_filled(line, CornerRadius::same(1), theme::glass(0.18));
        let done = Rect::from_min_max(line.min, pos2(line.left() + line.width() * frac, line.bottom()));
        ui.painter().rect_filled(done, CornerRadius::same(1), accent);
        if playing {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
    let _ = Color32::TRANSPARENT;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_pages_belong_to_their_tab() {
        assert_eq!(Tab::of(&Page::Home, Tab::Library), Tab::Home);
        assert_eq!(Tab::of(&Page::New, Tab::Library), Tab::Home);
        assert_eq!(Tab::of(&Page::Search("x".into()), Tab::Home), Tab::Search);
        assert_eq!(Tab::of(&Page::Library(LibTab::Songs), Tab::Home), Tab::Library);
        assert_eq!(Tab::of(&Page::Downloads, Tab::Home), Tab::Downloads);
    }

    #[test]
    fn a_page_opened_from_a_tab_keeps_that_tab() {
        for page in [Page::Album("a".into()), Page::Artist("a".into()), Page::Playlist("p".into()), Page::Settings, Page::Transfer] {
            assert_eq!(Tab::of(&page, Tab::Library), Tab::Library, "{page:?}");
            assert_eq!(Tab::of(&page, Tab::Search), Tab::Search, "{page:?}");
        }
    }

    #[test]
    fn each_tab_opens_a_page_that_belongs_to_it() {
        for tab in Tab::ALL {
            assert_eq!(Tab::of(&tab.page(), Tab::Home), tab);
            assert!(Tab::is_root(&tab.page()));
        }
    }

    #[test]
    fn only_tab_pages_are_roots() {
        assert!(!Tab::is_root(&Page::Album("a".into())));
        assert!(!Tab::is_root(&Page::Settings));
        assert!(!Tab::is_root(&Page::New));
    }
}
