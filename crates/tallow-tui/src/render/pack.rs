//! The pack screen: a list of what you carry, or one item opened up.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph, Wrap};
use tallow_core::item::{LIGHT_LOAD, MAX_LOAD, TALLOW_PER_TENTH};
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
        PackPurpose::Browse => " Pack ",
        PackPurpose::Throw => " Throw what? ",
    };
    let width = area.width.min(64);
    // Borders and padding take four columns; long lines wrap onto more rows.
    let inner = usize::from(width.saturating_sub(4)).max(1);
    let rows: usize = lines.iter().map(|l| l.width().max(1).div_ceil(inner)).sum();
    let height = (rows as u16 + 2).min(area.height);
    let [_, column, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, popup, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(column);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::bordered()
                .title(title)
                .border_style(Style::new().fg(palette::BORDER))
                .padding(Padding::horizontal(1))
                .style(Style::new().bg(palette::VOID)),
        ),
        popup,
    );
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
                    format!("tallow, {} turns of light", player.candle.tallow())
                ),
                dim(),
            ),
            Span::styled(
                format!("{:>5}", tenths(player.candle.tallow() / TALLOW_PER_TENTH)),
                dim(),
            ),
        ]));
    }
    if lines.len() == 2 {
        lines.push(Line::styled("You have nothing you could throw.", dim()));
    }
    lines.push(Line::default());
    lines.push(Line::styled("letter: choose · Esc: close", dim()));
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
        ItemClass::Melee { .. } | ItemClass::Ranged { .. } | ItemClass::Vestment { .. } => {
            actions.push(if equipped { "e take off" } else { "e equip" });
        }
        ItemClass::Ammo | ItemClass::Throwable => {}
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
