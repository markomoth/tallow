//! The browser loop. Keys come from the page, frames from
//! `requestAnimationFrame`; the run and the journal live in localStorage.
//!
//! Saving differs from the terminal in one way: a tab can close at any
//! moment, so the run is written after every key instead of on quit, and is
//! cleared when it ends. Reloading resumes exactly where you were, so it
//! undoes nothing (BUILD_GUIDE.md §11).

use std::cell::RefCell;
use std::rc::Rc;

use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::ratatui::Terminal;
use ratzilla::{CellSized, FontAtlasConfig, WebGl2Backend, WebRenderer};
use tallow_tui::input::{KeyCode, KeyEvent, KeyModifiers};
use tallow_tui::journal::Journal;
use tallow_tui::save::Save;
use tallow_tui::{App, render};
use web_sys::wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::{JsCast, JsValue};

/// localStorage keys.
const SAVE: &str = "tallow.save";
const JOURNAL: &str = "tallow.journal";

/// The page element the game is drawn into. Its size bounds the font size;
/// its `data-font` names the font family. The game sets its `data-state` to
/// `ready` once the screen is drawn, or `failed` if WebGL2 isn't there.
const SCREEN: &str = "tallow";

/// The screen in cells: the game's minimum, and all the page shows.
const COLS: f32 = 100.0;
const ROWS: f32 = 30.0;

struct Game {
    app: App,
    journal: Journal,
    /// The finished run is in the journal.
    recorded: bool,
}

impl Game {
    /// The run in progress if there is one, at the start menu.
    fn load() -> Game {
        let journal = read(JOURNAL)
            .map(|text| Journal::from_ron(&text))
            .unwrap_or_default();
        let mut app = match read(SAVE).and_then(|text| Save::from_ron(&text)) {
            Some(save) => App::resume(&save),
            None => App::new(random_seed()),
        };
        app.set_journal(journal.clone());
        app.show_title();
        Game {
            app,
            journal,
            recorded: false,
        }
    }

    fn press(&mut self, key: KeyEvent) {
        self.app.handle_key(key);
        if self.app.wants_restart() {
            self.app = App::new(random_seed());
            self.app.set_journal(self.journal.clone());
            self.recorded = false;
        }
        if self.app.is_over() {
            remove(SAVE);
            // A finished run goes into the journal once.
            if !self.recorded {
                self.recorded = true;
                self.journal.end_run(self.app.world());
                write(JOURNAL, &self.journal.to_ron());
                self.app.set_journal(self.journal.clone());
            }
        } else if self.app.started() {
            write(SAVE, &self.app.save().to_ron());
        }
        // A page can't close itself: quitting goes back to the start menu,
        // with the run kept and what you learned in the journal.
        if self.app.should_quit() {
            if self.app.started() && !self.app.is_over() {
                self.journal.record(self.app.world());
                write(JOURNAL, &self.journal.to_ron());
            }
            *self = Game::load();
        }
    }
}

pub fn start() {
    console_error_panic_hook::set_once();
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let font = screen()
        .and_then(|e| e.get_attribute("data-font"))
        .unwrap_or_else(|| "monospace".into());
    // The dynamic font atlas draws with the font as it is when the game
    // starts, so a web font has to finish loading first.
    let loaded = document.fonts().load(&format!("16px \"{font}\""));
    let draw = Closure::<dyn FnMut(JsValue)>::new(move |_| run(&font));
    // Start either way: a font that fails to load falls back to monospace.
    let _ = loaded.then2(&draw, &draw);
    draw.forget();
}

fn run(font: &str) {
    let Some(terminal) = backend(font).ok().and_then(|b| Terminal::new(b).ok()) else {
        set_state("failed");
        return;
    };
    let game = Rc::new(RefCell::new(Game::load()));
    listen(game.clone());
    let started = now();
    terminal.draw_web(move |frame| {
        let time = ((now() - started) / 1000.0) as f32;
        render::draw(frame, &game.borrow().app, time);
    });
    set_state("ready");
}

fn screen() -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(SCREEN)
}

fn set_state(state: &str) {
    if let Some(screen) = screen() {
        screen.set_attribute("data-state", state).ok();
    }
}

/// A canvas of exactly 100×30 cells, at the largest font that fits the
/// screen element.
fn backend(font: &str) -> Result<WebGl2Backend, ratzilla::error::Error> {
    let (width, height) = screen().map_or((1000.0, 600.0), |e| {
        (e.client_width() as f32, e.client_height() as f32)
    });
    // Monospace cells run about 0.6 of the font size wide and 1.36 tall.
    // A guess over is no harm: the page scales the screen down to fit.
    let size = (width / (COLS * 0.6))
        .min(height / (ROWS * 1.36))
        .floor()
        .clamp(8.0, 32.0);
    let mut backend = WebGl2Backend::new_with_options(
        WebGl2BackendOptions::new()
            .grid_id(SCREEN)
            .font_atlas_config(FontAtlasConfig::dynamic(&[font, "monospace"], size)),
    )?;
    let (cell_w, cell_h) = backend.cell_size_css_px();
    backend.set_size((COLS * cell_w).ceil() as u32, (ROWS * cell_h).ceil() as u32)?;
    Ok(backend)
}

/// Keys go to the game, except the browser's own shortcuts (reload, copy,
/// close the tab), which stay the browser's.
fn listen(game: Rc<RefCell<Game>>) {
    let on_key =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            if event.ctrl_key() || event.meta_key() || event.alt_key() || event.is_composing() {
                return;
            }
            let Some(key) = key(&event) else {
                return;
            };
            // Arrows, space and Tab would otherwise scroll or move focus.
            event.prevent_default();
            game.borrow_mut().press(key);
        });
    if let Some(window) = web_sys::window() {
        window
            .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
            .ok();
    }
    on_key.forget();
}

/// The keys Tallow listens to, out of the browser's.
fn key(event: &web_sys::KeyboardEvent) -> Option<KeyEvent> {
    let name = event.key();
    let mut chars = name.chars();
    let code = match (chars.next(), chars.next()) {
        (Some(c), None) => KeyCode::Char(c),
        _ => match name.as_str() {
            "ArrowUp" => KeyCode::Up,
            "ArrowDown" => KeyCode::Down,
            "ArrowLeft" => KeyCode::Left,
            "ArrowRight" => KeyCode::Right,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            "Tab" => KeyCode::Tab,
            "Escape" => KeyCode::Esc,
            "Enter" => KeyCode::Enter,
            _ => return None,
        },
    };
    let modifiers = KeyModifiers {
        shift: event.shift_key(),
        control: false,
    };
    Some(KeyEvent::new(code, modifiers))
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn read(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

fn write(key: &str, value: &str) {
    if let Some(storage) = storage() {
        storage.set_item(key, value).ok();
    }
}

fn remove(key: &str) {
    if let Some(storage) = storage() {
        storage.remove_item(key).ok();
    }
}

/// Milliseconds since the page opened.
fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

fn random_seed() -> u64 {
    let half = || (js_sys::Math::random() * f64::from(u32::MAX)) as u64;
    (half() << 32) | half()
}
