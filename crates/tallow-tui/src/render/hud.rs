//! The status sidebar.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use tallow_core::World;

use super::palette;

pub struct Hud<'a> {
    world: &'a World,
}

impl<'a> Hud<'a> {
    pub fn new(world: &'a World) -> Self {
        Self { world }
    }
}

impl Widget for Hud<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dim = Style::new().fg(palette::TEXT_DIM);
        let text = Style::new().fg(palette::TEXT);
        let key = |k: &'static str, what: &'static str| {
            Line::from(vec![
                Span::styled(format!("{k:<9}"), text),
                Span::styled(what, dim),
            ])
        };

        let lines = vec![
            Line::styled(
                "T A L L O W",
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            Line::styled("Acolyte", text),
            Line::styled("The Undercroft", dim),
            Line::styled(format!("Turn {}", self.world.turn()), dim),
            Line::default(),
            Line::styled("─ keys ─", dim),
            key("hjkl ←→", "move"),
            key("yubn", "diagonal"),
            key(". 5", "wait"),
            key("q", "quit"),
        ];

        Paragraph::new(lines)
            .block(
                Block::new()
                    .borders(Borders::LEFT)
                    .border_style(Style::new().fg(palette::BORDER))
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .render(area, buf);
    }
}
