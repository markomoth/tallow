//! Screen layout: map on the left, status on the right, log along the bottom.

mod hud;
mod map;
mod palette;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::log::MessageLog;

pub const MIN_WIDTH: u16 = 100;
pub const MIN_HEIGHT: u16 = 30;
const SIDEBAR_WIDTH: u16 = 26;
const LOG_HEIGHT: u16 = 6;

/// Draws one frame. `time` is seconds since start, for visual effects only.
pub fn draw(frame: &mut Frame, app: &App, time: f32) {
    let area = frame.area();
    frame.render_widget(Block::new().style(Style::new().bg(palette::VOID)), area);

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    let [top, log_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(LOG_HEIGHT)]).areas(area);
    let [map_area, hud_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(SIDEBAR_WIDTH)]).areas(top);

    frame.render_widget(map::MapView::new(app.world(), time), map_area);
    frame.render_widget(hud::Hud::new(app.world()), hud_area);
    draw_log(frame, log_area, app.log());
}

fn draw_log(frame: &mut Frame, area: Rect, log: &MessageLog) {
    let block = Block::new()
        .borders(Borders::TOP)
        .border_style(Style::new().fg(palette::BORDER));
    let rows = block.inner(area).height as usize;

    // Newest line at the bottom, in full color; older lines fade.
    let entries: Vec<_> = log.entries().rev().take(rows).collect();
    let lines: Vec<Line> = entries
        .iter()
        .rev()
        .enumerate()
        .map(|(i, entry)| {
            let newest = i + 1 == entries.len();
            let color = if newest {
                palette::TEXT
            } else {
                palette::TEXT_DIM
            };
            let text = if entry.count > 1 {
                format!("{} (×{})", entry.text, entry.count)
            } else {
                entry.text.clone()
            };
            Line::styled(text, Style::new().fg(color))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let message = format!(
        "Tallow needs a terminal of at least {MIN_WIDTH}×{MIN_HEIGHT}.\n\
         This one is {}×{}. Enlarge the window or shrink the font.",
        area.width, area.height
    );
    let [_, middle, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::new().fg(palette::TEXT))
            .centered()
            .wrap(Wrap { trim: true }),
        middle,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render(width: u16, height: u16, app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, app, 0.0)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn full_screen_shows_player_hud_and_log() {
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &App::new(7));
        assert!(screen.contains('@'));
        assert!(screen.contains("T A L L O W"));
        assert!(screen.contains("floorboards"));
        assert!(screen.contains("Floor 1 of 12"));
        assert!(screen.contains("seed 7"));
    }

    #[test]
    fn descending_updates_the_hud_and_log() {
        use crate::input::Action;
        use tallow_core::{Command, Direction, Tile, map::path};

        let mut app = App::new(7);
        let stairs = app.world().map().find(Tile::StairsDown).next().unwrap();
        let dist = path::distances(app.world().map(), stairs);
        while app.world().player() != stairs {
            let here = app.world().player();
            let dir = Direction::ALL
                .into_iter()
                .find(|&d| dist.at(here + d).is_some_and(|n| Some(n) < dist.at(here)))
                .unwrap();
            app.handle(Action::Game(Command::Move(dir)));
        }
        app.handle(Action::Game(Command::Descend));

        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("Floor 2 of 12"));
        assert!(screen.contains("The stair turns more times than it should."));
    }

    #[test]
    fn small_terminal_gets_a_message_instead_of_a_broken_layout() {
        let screen = render(60, 20, &App::new(7));
        assert!(screen.contains("at least 100×30"));
        assert!(!screen.contains('@'));
    }
}
