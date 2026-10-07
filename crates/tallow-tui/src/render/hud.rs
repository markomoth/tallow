//! The status sidebar.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget};
use tallow_core::{Biome, MAX_DEPTH, World};

use super::palette;

pub struct Hud<'a> {
    world: &'a World,
}

impl<'a> Hud<'a> {
    pub fn new(world: &'a World) -> Self {
        Self { world }
    }
}

pub fn biome_name(biome: Biome) -> &'static str {
    match biome {
        Biome::Crypts => "The Crypts",
        Biome::Collegium => "The Collegium",
        Biome::DrownedStacks => "The Drowned Stacks",
        Biome::RotCourt => "The Rot Court",
        Biome::Throne => "The Throne",
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
        let depth = self.world.depth();

        let lines = vec![
            Line::styled(
                "T A L L O W",
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            Line::styled("Acolyte", text),
            Line::styled(format!("Floor {depth} of {MAX_DEPTH}"), text),
            Line::styled(biome_name(Biome::for_depth(depth)), dim),
            Line::styled(format!("Turn {}", self.world.turn()), dim),
            Line::default(),
            Line::styled("─ keys ─", dim),
            key("hjkl ←→", "move"),
            key("yubn", "diagonal"),
            key("HJKL ⇧←", "run"),
            key(". 5", "wait"),
            key(">", "descend"),
            key("q", "quit"),
        ];

        let block = Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::new().fg(palette::BORDER))
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        block.render(area, buf);

        let [body, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
        Paragraph::new(lines).render(body, buf);
        Line::styled(format!("seed {}", self.world.seed()), dim).render(footer, buf);
    }
}
