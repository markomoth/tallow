//! The Look card: what's under the cursor, in a card on the map beside it,
//! joined to it by a line. It never covers the thing it describes.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use tallow_core::{Faction, Mind, Point, World};

use super::hud::{faction_line, tile_line, trait_line, wound_word};
use super::palette::{self, rgb};
use super::sheet::frame_block;
use super::text::wrap_all;
use crate::log::capitalize;
use crate::names::{item_phrase, item_stats};

/// The card's full width, borders included.
const CARD_WIDTH: u16 = 48;
/// The narrowest the card gets beside its target before it moves above or
/// below instead.
const MIN_WIDTH: u16 = 32;
/// Columns between the cursor and the card, for the leader line.
const GAP: u16 = 3;
const BAR: usize = 10;

/// What a card says: its title, an optional tag on the right of the top
/// border, and the body.
struct Card {
    title: Line<'static>,
    tag: Option<Line<'static>>,
    body: Vec<Line<'static>>,
}

fn dim() -> Style {
    Style::new().fg(palette::TEXT_DIM)
}

fn text() -> Style {
    Style::new().fg(palette::TEXT)
}

fn warn() -> Style {
    Style::new().fg(palette::DANGER)
}

/// Where the card goes, relative to the thing it describes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Right,
    Left,
    Above,
    Below,
}

/// Draws the card for `cursor` beside it, inside the map `area`. Beside it
/// if either side has room, else above or below it: never over it.
pub fn draw(frame: &mut Frame, area: Rect, world: &World, cursor: Point) {
    let Some((cx, cy)) = super::map::screen_pos(world, area, cursor) else {
        return;
    };
    let card = card(world, cursor);

    let room_right = area.right().saturating_sub(cx + GAP + 1);
    let room_left = (cx + 1).saturating_sub(area.x + GAP);
    let (side, width) = if room_right.max(room_left) >= MIN_WIDTH {
        if room_right >= CARD_WIDTH || room_right >= room_left {
            (Side::Right, room_right.min(CARD_WIDTH))
        } else {
            (Side::Left, room_left.min(CARD_WIDTH))
        }
    } else {
        let above = cy.saturating_sub(area.y);
        let below = area.bottom().saturating_sub(cy + 1);
        let side = if below >= above {
            Side::Below
        } else {
            Side::Above
        };
        (side, area.width.min(CARD_WIDTH))
    };
    let room = match side {
        Side::Right | Side::Left => area.height,
        Side::Above => cy.saturating_sub(area.y),
        Side::Below => area.bottom().saturating_sub(cy + 1),
    };
    if room < 3 {
        return;
    }
    let mut rows = wrap_all(&card.body, usize::from(width.saturating_sub(4)));
    let fits = usize::from(room - 2);
    if rows.len() > fits {
        let hidden = rows.len() - fits + 1;
        rows.truncate(fits - 1);
        rows.push(Line::styled(format!("▾ {hidden} more lines"), dim()));
    }
    let height = rows.len() as u16 + 2;

    let x = match side {
        Side::Right => cx + GAP,
        Side::Left => cx + 1 - GAP - width,
        Side::Above | Side::Below => cx
            .saturating_sub(width / 2)
            .clamp(area.x, area.right().saturating_sub(width)),
    };
    let y = match side {
        // The target's row meets the card's first line of text.
        Side::Right | Side::Left => cy
            .saturating_sub(1)
            .clamp(area.y, area.bottom().saturating_sub(height)),
        Side::Above => cy - height,
        Side::Below => cy + 1,
    };
    let rect = Rect::new(x, y, width, height);

    let mut block = frame_block("", "Tab next · Esc done")
        .title(card.title)
        .border_style(Style::new().fg(palette::FRAME));
    if let Some(tag) = card.tag {
        block = block.title(tag.right_aligned());
    }
    frame.render_widget(Clear, rect);
    frame.render_widget(Paragraph::new(rows).block(block), rect);

    // The leader: from beside the cursor to the card's edge.
    let buf = frame.buffer_mut();
    let frame_color = palette::FRAME;
    let (from, to, joint) = match side {
        Side::Right => (cx + 1, x, (x, '┤')),
        Side::Left => (x + width, cx, (x + width - 1, '├')),
        // Above or below, the card already touches the target's row.
        Side::Above | Side::Below => return,
    };
    for lx in from..to {
        if let Some(cell) = buf.cell_mut((lx, cy)) {
            cell.set_char('─').set_fg(frame_color);
        }
    }
    if cy > y
        && cy + 1 < y + height
        && let Some(cell) = buf.cell_mut((joint.0, cy))
    {
        cell.set_char(joint.1).set_fg(frame_color);
    }
}

fn glyph_title(glyph: char, color: Color, name: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("{glyph}  "),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", capitalize(name)),
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn plain_title(name: &str, color: Color) -> Line<'static> {
    Line::styled(
        format!(" {} ", capitalize(name)),
        Style::new().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn faction_name(faction: Faction) -> &'static str {
    match faction {
        Faction::Dreaming => "The Dreaming",
        Faction::Taken => "The Taken",
        Faction::Swarm => "The Swarm",
        Faction::Remnant => "The Remnant",
    }
}

/// A row of numbers: a label, a chance, a range.
fn odds(label: &str, chance: u32, (lo, hi): (u32, u32)) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<14}"), dim()),
        Span::styled(
            format!("{chance:>3}%  "),
            text().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{lo}–{hi}"), text().add_modifier(Modifier::BOLD)),
    ])
}

fn card(world: &World, cursor: Point) -> Card {
    let floor = world.floor();
    let monster = floor.monster_at(cursor).and_then(|id| world.inspect(id));

    if let Some(info) = monster.as_ref().filter(|info| info.phantom) {
        let def = world.content().monster(info.kind);
        return Card {
            title: glyph_title(def.glyph, rgb(def.color), &def.name),
            tag: None,
            body: vec![Line::styled(
                "It casts no shadow in your light. It isn't really there, but if it reaches you your fear will make it hurt (1–2, or +5 dread).",
                Style::new().fg(palette::DREAD),
            )],
        };
    }
    if let Some(info) = monster {
        let def = world.content().monster(info.kind);
        let mind = match info.mind {
            _ if info.compelled > 0 => ("bound to your will", palette::GOOD),
            _ if info.terrified > 0 => ("fleeing your dread", palette::GOOD),
            _ if info.unseeing > 0 => ("can't see you", palette::GOOD),
            Mind::Unaware => ("unaware of you", palette::TEXT_DIM),
            Mind::Hunting { .. } => ("hunting you", palette::DANGER),
            Mind::Fleeing => ("fleeing", palette::TEXT_DIM),
        };
        let filled = (info.health as usize * BAR)
            .div_ceil(info.max_health.max(1) as usize)
            .min(BAR);
        let mut body = vec![
            Line::from(vec![
                Span::styled("█".repeat(filled), Style::new().fg(palette::HEALTH)),
                Span::styled(
                    "░".repeat(BAR - filled),
                    Style::new().fg(palette::HEALTH_EMPTY),
                ),
                Span::styled(
                    format!(" {} · ", wound_word(&info).unwrap_or("unhurt")),
                    dim(),
                ),
                Span::styled(mind.0, Style::new().fg(mind.1)),
            ]),
            Line::default(),
            odds("You hit it", info.your_hit_chance, info.your_damage),
            odds("It hits you", info.its_hit_chance, info.its_damage),
        ];
        let dread = Style::new().fg(palette::DREAD);
        if info.resists {
            body.push(Line::styled(
                match def.faction {
                    Faction::Swarm => "Half damage: blunt is wasted.",
                    Faction::Remnant => "Half damage: blades only nick.",
                    _ => "Half damage until light holds it.",
                },
                dread,
            ));
        }
        if info.in_the_dark {
            body.push(Line::styled(
                "In the dark: you strike worse, it strikes harder.",
                dread,
            ));
        }
        if info.flankers > 0 {
            body.push(Line::styled(
                format!("Flanking you with its kin: +{} to hit.", info.flankers * 10),
                warn(),
            ));
        }
        if info.winding_up.is_some() {
            body.push(Line::styled(
                if info.lunging {
                    "! It crouches to leap. Get off the red tile!"
                } else {
                    "! Its blow is raised. Get off the red tile!"
                },
                warn().add_modifier(Modifier::BOLD),
            ));
        }
        if def.boss {
            body.push(Line::styled(
                "One of the great ones below. It resists Binding.",
                warn(),
            ));
        }
        if info.compelled > 0 {
            body.push(Line::styled(
                format!("Compelled for {} more of its actions.", info.compelled),
                Style::new().fg(palette::GOOD),
            ));
        }
        if info.pinned > 0 {
            body.push(Line::styled(
                format!("Can't move for {} actions.", info.pinned),
                text(),
            ));
        }
        if info.carries_dread {
            body.push(Line::styled("It carries some of your dread.", dim()));
        }
        for t in &info.known_traits {
            body.push(Line::from(vec![
                Span::styled("Seen: ", dim()),
                Span::styled(trait_line(t), text()),
            ]));
        }
        // The numbers and warnings first: on a small screen the prose is
        // what gets cut.
        body.push(Line::default());
        body.push(Line::styled(def.description.clone(), dim()));
        body.push(Line::default());
        body.push(Line::styled(
            faction_line(def.faction),
            Style::new().fg(rgb(def.color)),
        ));
        if world.has_studied(info.kind) {
            body.push(Line::styled("You have studied its kind.", dim()));
        }
        return Card {
            title: glyph_title(def.glyph, rgb(def.color), &def.name),
            tag: Some(Line::styled(
                format!(" {} ", faction_name(def.faction)),
                Style::new().fg(rgb(def.color)),
            )),
            body,
        };
    }

    if cursor == world.player().pos {
        return Card {
            title: glyph_title('@', palette::PLAYER, "you"),
            tag: None,
            body: vec![Line::styled(
                "Acolyte of the Low Bell, a candle in one hand.",
                text(),
            )],
        };
    }
    if let Some(top) = floor
        .items_at(cursor)
        .last()
        .filter(|_| floor.is_explored(cursor))
    {
        let count = floor.items_at(cursor).count();
        let def = world.content().item(top.item.kind);
        let mut body = vec![Line::styled(def.description.clone(), dim())];
        body.extend(
            item_stats(world, top.item.kind)
                .into_iter()
                .map(|s| Line::styled(s, text())),
        );
        if count > 1 {
            body.push(Line::styled(
                format!("…and {} more things here.", count - 1),
                dim(),
            ));
        }
        return Card {
            title: glyph_title(
                def.glyph,
                rgb(def.color),
                &item_phrase(world, top.item.kind, top.item.count),
            ),
            tag: None,
            body,
        };
    }
    if let Some(&(_, id)) = floor
        .leavings()
        .iter()
        .find(|&&(at, _)| at == cursor && floor.is_explored(cursor))
    {
        let l = world.leaving(id);
        let mut body = vec![
            Line::styled(
                format!("A Leaving ({}).", crate::names::tier_name(l.tier())),
                text(),
            ),
            Line::styled(crate::names::leaving_tell(world, id), dim()),
        ];
        if l.known {
            body.push(Line::styled(crate::names::leaving_rule(world, id), text()));
        }
        return Card {
            title: plain_title(
                &crate::names::leaving_name(world, id),
                rgb(palette::tier_color(l.tier())),
            ),
            tag: None,
            body,
        };
    }
    if let Some(a) = world
        .anomaly_at(cursor)
        .filter(|a| a.revealed && floor.is_explored(cursor))
    {
        let (name, what) = crate::names::anomaly_text(a.kind);
        return Card {
            title: plain_title(name, rgb(palette::ANOMALY_FG)),
            tag: None,
            body: vec![Line::styled(what, text())],
        };
    }
    if let Some(corpse) = floor
        .corpse_at(cursor)
        .filter(|_| floor.is_explored(cursor))
    {
        let def = world.content().monster(corpse.kind);
        let state = match corpse.decay(world.turn()) {
            tallow_core::Decay::Fresh => "Fresh.",
            tallow_core::Decay::Swelling if def.health >= tallow_core::corpse::HATCH_HEALTH => {
                "Swelling. Flies will hatch from it soon."
            }
            tallow_core::Decay::Swelling => "Rotting. It will soon be gone.",
        };
        return Card {
            title: plain_title(
                &format!("the body of {}", crate::log::with_article(&def.name)),
                palette::TEXT,
            ),
            tag: None,
            body: vec![
                Line::styled(state, text()),
                Line::styled("Stand on it and press s to study or render it.", dim()),
            ],
        };
    }
    if let Some(tallow) = floor
        .tallow_at(cursor)
        .filter(|_| floor.is_explored(cursor))
    {
        return Card {
            title: plain_title("tallow", rgb(palette::TALLOW_FG)),
            tag: None,
            body: vec![Line::styled(
                format!(
                    "Enough for about {} turns of light. Walk over it to take it.",
                    tallow.amount
                ),
                text(),
            )],
        };
    }
    if !floor.is_explored(cursor) {
        return Card {
            title: plain_title("unknown", palette::TEXT_DIM),
            tag: None,
            body: vec![Line::styled("You don't know what is there.", dim())],
        };
    }

    let mut body = vec![Line::styled(tile_line(floor.map().tile(cursor)), text())];
    if floor.is_burning(cursor) && floor.is_visible(cursor) {
        body.push(Line::styled(
            "On fire! It burns whatever stands in it, the Swarm worst of all.",
            warn(),
        ));
    }
    if floor.has_oil(cursor) {
        body.push(Line::styled(
            "Spilled lamp oil: slippery, and it burns fast.",
            text(),
        ));
    }
    if floor.is_seep(cursor) {
        body.push(Line::styled(
            "The air here ripples like heat over a road. Something unseen is wrong with it: throw something through before you walk in.",
            Style::new().fg(rgb(palette::ANOMALY_FG)),
        ));
    }
    if floor.is_sanctified(cursor) {
        body.push(Line::styled(
            "Holy ground. The Dreaming can't cross it; the Taken flinch.",
            Style::new().fg(palette::GOOD),
        ));
    }
    if floor.decoy().is_some_and(|d| d.at == cursor) {
        body.push(Line::styled(
            "Your false flame. Creatures near it go to it.",
            Style::new().fg(palette::GOOD),
        ));
    }
    if !floor.is_visible(cursor) {
        body.push(Line::styled("(remembered, not in view)", dim()));
    }
    Card {
        title: plain_title(
            if floor.is_visible(cursor) {
                "here"
            } else {
                "remembered"
            },
            palette::ACCENT,
        ),
        tag: None,
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn the_card_never_covers_what_it_describes() {
        let world = crate::app::App::new(7).world().clone();
        let map = world.floor().map();
        let area = Rect::new(0, 0, 74, 24);
        let mut terminal = Terminal::new(TestBackend::new(74, 24)).unwrap();
        for y in (0..map.height()).step_by(2) {
            for x in (0..map.width()).step_by(3) {
                let p = Point::new(x, y);
                let Some((cx, cy)) = super::super::map::screen_pos(&world, area, p) else {
                    continue;
                };
                terminal.clear().unwrap();
                terminal.draw(|frame| draw(frame, area, &world, p)).unwrap();
                let cell = &terminal.backend().buffer()[(cx, cy)];
                assert_eq!(cell.symbol(), " ", "covered at {cx},{cy}");
            }
        }
    }
}
