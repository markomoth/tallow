//! Tallow: a dark-fantasy roguelike for the terminal.

mod app;
mod input;
mod log;
mod render;

use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;

fn main() -> Result<()> {
    // `init` enters the alternate screen and installs a panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    while !app.should_quit() {
        terminal.draw(|frame| render::draw(frame, &app))?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && let Some(action) = input::map_key(key)
        {
            app.handle(action);
        }
    }
    Ok(())
}
