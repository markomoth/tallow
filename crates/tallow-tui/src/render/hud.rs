//! The status sidebar: who you are, where you are, what you can see.
//! While looking, it shows what's under the cursor instead.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap};
use tallow_core::{Biome, MAX_DEPTH, Mind, MonsterInfo, Point, Tile, Trait, World};

use super::palette::{self, rgb};
use crate::app::{App, Mode};
use crate::log::capitalize;

const BAR_WIDTH: usize = 10;
const MAX_IN_VIEW: usize = 5;

pub struct Hud<'a> {
    app: &'a App,
}

impl<'a> Hud<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
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

fn dim() -> Style {
    Style::new().fg(palette::TEXT_DIM)
}

fn text() -> Style {
    Style::new().fg(palette::TEXT)
}

impl Widget for Hud<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let world = self.app.world();
        let player = world.player();
        let depth = world.depth();

        let mut lines = vec![
            Line::styled(
                "T A L L O W",
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            Line::styled("Acolyte", text()),
            health_bar(player.health, player.max_health),
            Line::styled(format!("Floor {depth} of {MAX_DEPTH}"), text()),
            Line::styled(biome_name(Biome::for_depth(depth)), dim()),
            Line::styled(format!("Turn {}", world.turn()), dim()),
            Line::default(),
        ];
        match self.app.mode() {
            Mode::Look { cursor } => lines.extend(look_panel(world, cursor)),
            Mode::Play | Mode::Dead => {
                lines.extend(in_view(world));
                lines.extend(keys());
            }
        }

        let block = Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::new().fg(palette::BORDER))
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        block.render(area, buf);

        let [body, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .render(body, buf);
        Line::styled(format!("seed {}", world.seed()), dim()).render(footer, buf);
    }
}

fn health_bar(health: u32, max: u32) -> Line<'static> {
    let filled = (health as usize * BAR_WIDTH)
        .div_ceil(max.max(1) as usize)
        .min(BAR_WIDTH);
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::new().fg(palette::HEALTH)),
        Span::styled(
            "░".repeat(BAR_WIDTH - filled),
            Style::new().fg(palette::HEALTH_EMPTY),
        ),
        Span::styled(format!(" {health}/{max}"), text()),
    ])
}

fn in_view(world: &World) -> Vec<Line<'static>> {
    let ids = world.floor().visible_monsters(world.player().pos);
    if ids.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![Line::styled("─ in view ─", dim())];
    for info in ids
        .iter()
        .filter_map(|&id| world.inspect(id))
        .take(MAX_IN_VIEW)
    {
        let def = world.content().monster(info.kind);
        let mut spans = vec![
            Span::styled(
                format!("{} ", def.glyph),
                Style::new().fg(rgb(def.color)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(def.name.clone(), text()),
        ];
        if info.winding_up.is_some() {
            spans.push(Span::styled(
                " !",
                Style::new()
                    .fg(palette::DANGER)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if let Some(state) = wound_word(&info) {
            spans.push(Span::styled(format!(" ({state})"), dim()));
        }
        lines.push(Line::from(spans));
    }
    if ids.len() > MAX_IN_VIEW {
        lines.push(Line::styled(
            format!("…and {} more", ids.len() - MAX_IN_VIEW),
            dim(),
        ));
    }
    lines.push(Line::default());
    lines
}

fn keys() -> Vec<Line<'static>> {
    let key = |k: &'static str, what: &'static str| {
        Line::from(vec![
            Span::styled(format!("{k:<6}"), text()),
            Span::styled(what, dim()),
        ])
    };
    vec![
        Line::styled("─ keys ─", dim()),
        key("hjkl", "move"),
        key("yubn", "diagonal"),
        key("HJKL", "run"),
        key(". >", "wait, descend"),
        key("x", "look"),
        key("q", "quit"),
    ]
}

fn wound_word(info: &MonsterInfo) -> Option<&'static str> {
    let left = info.health * 4 / info.max_health.max(1);
    match left {
        _ if info.health == info.max_health => None,
        3.. => Some("hurt"),
        2 => Some("wounded"),
        1 => Some("badly hurt"),
        0 => Some("near death"),
    }
}

fn look_panel(world: &World, cursor: Point) -> Vec<Line<'static>> {
    let floor = world.floor();
    let mut lines = vec![Line::styled("─ look ─", dim())];
    let monster = floor.monster_at(cursor).and_then(|id| world.inspect(id));

    if let Some(info) = monster {
        let def = world.content().monster(info.kind);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", def.glyph),
                Style::new().fg(rgb(def.color)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(capitalize(&def.name), text().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(def.description.clone(), dim()));
        let mind = match info.mind {
            Mind::Unaware => "unaware of you",
            Mind::Hunting { .. } => "hunting you",
            Mind::Fleeing => "fleeing",
        };
        let health = wound_word(&info).unwrap_or("unhurt");
        lines.push(Line::styled(
            format!("{} · {mind}", capitalize(health)),
            text(),
        ));
        lines.push(Line::styled(
            format!("You hit it   {}%", info.your_hit_chance),
            text(),
        ));
        let (lo, hi) = info.its_damage;
        lines.push(Line::styled(
            format!("It hits you  {}% · {lo}–{hi}", info.its_hit_chance),
            text(),
        ));
        if info.winding_up.is_some() {
            lines.push(Line::styled(
                "Its blow is raised. Get off the red tile!",
                Style::new()
                    .fg(palette::DANGER)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        for t in &info.known_traits {
            lines.push(Line::styled(format!("Seen: {}", trait_line(t)), dim()));
        }
    } else if cursor == world.player().pos {
        lines.push(Line::styled(
            "You. Acolyte of the Low Bell, a candle in one hand.",
            text(),
        ));
    } else if !floor.is_explored(cursor) {
        lines.push(Line::styled("You don't know what is there.", dim()));
    } else {
        lines.push(Line::styled(tile_line(floor.map().tile(cursor)), text()));
        if !floor.is_visible(cursor) {
            lines.push(Line::styled("(remembered, not in view)", dim()));
        }
    }
    lines.push(Line::default());
    lines.push(Line::styled("Tab next · Esc done", dim()));
    lines
}

fn trait_line(t: &Trait) -> String {
    match *t {
        Trait::PackCourage => "flees when no packmate is near.".into(),
        Trait::Pleads => "speaks. It may still be someone.".into(),
        Trait::ShunsLight => "won't cross brazier light.".into(),
        Trait::HeavyBlow {
            damage: (lo, hi), ..
        } => {
            format!("heavy blow, {lo}–{hi}. Aimed a turn ahead at a marked tile.")
        }
    }
}

fn tile_line(tile: Tile) -> &'static str {
    match tile {
        Tile::Floor => "Flagstones, worn smooth by centuries of feet.",
        Tile::Wall => "Old stone, sweating in the cold.",
        Tile::Door => "A doorway. The door hangs open.",
        Tile::StairsDown => "Stairs down. There is no coming back up.",
        Tile::StairsUp => "The way you came. Sealed now.",
        Tile::Brazier => {
            "A brazier, burning without fuel. Some things in the dark will not cross its light."
        }
    }
}
