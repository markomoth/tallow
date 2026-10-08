//! The pack screen: a list of what you carry, or one item opened up.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use tallow_core::item::{LIGHT_LOAD, MAX_LOAD};
use tallow_core::{ItemClass, ItemId, World};

use super::palette::{self, rgb};
use crate::app::PackPurpose;
use crate::log::capitalize;
use crate::names::{item_name, item_stats, tenths};

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    world: &World,
    purpose: PackPurpose,
    selected: Option<ItemId>,
) {
    let lines = match selected.and_then(|id| world.inventory_item(id)) {
        Some(item) => detail(world, item.id),
        None => list(world, purpose),
    };
    let title = match purpose {
        PackPurpose::Browse => "Pack",
        PackPurpose::Throw => "Throw what?",
    };
    let footer = if selected.is_some() {
        ""
    } else {
        "letter choose · Esc close"
    };
    super::sheet::popup(frame, area, title, lines, 64, footer);
}

fn dim() -> Style {
    Style::new().fg(palette::TEXT_DIM)
}

fn text() -> Style {
    Style::new().fg(palette::TEXT)
}

fn list(world: &World, purpose: PackPurpose) -> Vec<Line<'static>> {
    let player = world.player();
    let mut lines = vec![
        Line::styled(
            format!(
                "Load {} · burdened over {} · can't move over {}",
                tenths(world.load()),
                tenths(LIGHT_LOAD),
                tenths(MAX_LOAD)
            ),
            dim(),
        ),
        Line::default(),
    ];
    for (i, item) in player.inventory.iter().enumerate() {
        let def = world.content().item(item.kind);
        if purpose == PackPurpose::Throw && def.thrown.is_none() {
            continue;
        }
        let letter = (b'a' + i as u8) as char;
        let worn = if player.equipment.contains(item.id) {
            match def.class {
                ItemClass::Vestment { .. } => " (worn)",
                ItemClass::Ranged { .. } => " (ready)",
                _ => " (in hand)",
            }
        } else {
            ""
        };
        let name = format!("{}{worn}", item_name(world, item.kind, item.count));
        lines.push(Line::from(vec![
            Span::styled(format!("{letter}  "), Style::new().fg(palette::ACCENT)),
            Span::styled(format!("{} ", def.glyph), Style::new().fg(rgb(def.color))),
            Span::styled(format!("{name:<42}"), text()),
            Span::styled(format!("{:>5}", tenths(def.weight * item.count)), dim()),
        ]));
    }
    if purpose == PackPurpose::Browse {
        lines.push(Line::from(vec![
            Span::styled("   ", dim()),
            Span::styled(
                format!(
                    "{:<44}",
                    match player.candle.spare() {
                        0 => format!("tallow, {} turns of light", player.candle.tallow()),
                        spare => format!(
                            "tallow, {} turns of light ({spare} in heavy lumps)",
                            player.candle.tallow()
                        ),
                    }
                ),
                dim(),
            ),
            Span::styled(format!("{:>5}", tenths(player.candle.weight())), dim()),
        ]));
    }
    if lines.len() == 2 {
        lines.push(Line::styled("You have nothing you could throw.", dim()));
    }
    if purpose == PackPurpose::Browse && !world.carried_leavings().is_empty() {
        lines.push(Line::default());
        lines.push(Line::styled("─ Leavings ─ (number to open)", dim()));
        for (i, &id) in world.carried_leavings().iter().enumerate() {
            let l = world.leaving(id);
            let name = crate::names::leaving_name(world, id);
            lines.push(Line::from(vec![
                Span::styled(format!("{}  ", i + 1), Style::new().fg(palette::ACCENT)),
                Span::styled("* ", Style::new().fg(rgb(palette::tier_color(l.tier())))),
                Span::styled(
                    format!(
                        "{:<42}",
                        format!("{name} ({})", crate::names::tier_name(l.tier()))
                    ),
                    text(),
                ),
                Span::styled(format!("{:>5}", tenths(l.weight)), dim()),
            ]));
        }
    }
    lines
}

fn detail(world: &World, id: ItemId) -> Vec<Line<'static>> {
    let item = world.inventory_item(id).copied().expect("caller checked");
    let def = world.content().item(item.kind);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{} ", def.glyph), Style::new().fg(rgb(def.color))),
            Span::styled(
                capitalize(&item_name(world, item.kind, item.count)),
                text().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::styled(def.description.clone(), dim()),
        Line::default(),
    ];
    lines.extend(
        item_stats(world, item.kind)
            .into_iter()
            .map(|s| Line::styled(s, text())),
    );
    lines.push(Line::default());

    let equipped = world.player().equipment.contains(id);
    let mut actions: Vec<&str> = Vec::new();
    match def.class {
        ItemClass::Tincture { .. } => actions.push("a drink"),
        ItemClass::Text { .. } => actions.push("a read"),
        ItemClass::Bell { .. } => actions.push("a ring"),
        ItemClass::Melee { .. } | ItemClass::Ranged { .. } | ItemClass::Vestment { .. } => {
            actions.push(if equipped { "e take off" } else { "e equip" });
        }
        ItemClass::Ammo | ItemClass::Throwable | ItemClass::Relic => {}
    }
    if def.thrown.is_some() {
        actions.push("t throw");
    }
    actions.push("d drop");
    actions.push("Esc back");
    lines.push(Line::styled(
        actions.join(" · "),
        Style::new().fg(palette::ACCENT),
    ));
    lines
}

/// One carried Leaving, opened up.
pub fn draw_leaving(
    frame: &mut Frame,
    area: Rect,
    world: &World,
    id: tallow_core::leavings::LeavingId,
) {
    let l = world.leaving(id);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("* ", Style::new().fg(rgb(palette::tier_color(l.tier())))),
            Span::styled(
                capitalize(&crate::names::leaving_name(world, id)),
                text().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::styled(
            format!(
                "A Leaving ({}). Weight {}.",
                crate::names::tier_name(l.tier()),
                tenths(l.weight)
            ),
            dim(),
        ),
        Line::default(),
        Line::styled(crate::names::leaving_tell(world, id), dim()),
        Line::default(),
    ];
    if l.known {
        lines.push(Line::styled(crate::names::leaving_rule(world, id), text()));
    } else {
        lines.push(Line::styled(
            "You don't know what wakes it, what it does, or what it takes. You'll learn when it wakes.",
            text(),
        ));
    }
    if l.armed {
        lines.push(Line::styled(
            "It is about to take more than you have. Drop it!",
            Style::new()
                .fg(palette::DANGER)
                .add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::default());
    lines.push(Line::styled(
        "a use · d drop · Esc back",
        Style::new().fg(palette::ACCENT),
    ));
    super::sheet::popup(frame, area, "Leaving", lines, 64, "");
}
