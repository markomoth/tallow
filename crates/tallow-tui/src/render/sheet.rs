//! The character sheet (`@`) and the level-up draft.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph, Wrap};
use tallow_core::progress::insight_for_next;
use tallow_core::skills::MAX_RANK;
use tallow_core::{Skill, World};

use super::palette;
use crate::names::{boon_text, skill_name, technique_text};

const BAR: usize = 10;

fn dim() -> Style {
    Style::new().fg(palette::TEXT_DIM)
}

fn text() -> Style {
    Style::new().fg(palette::TEXT)
}

/// A bordered popup centered in `area`, sized to its wrapped text.
pub fn popup(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line<'static>>, width: u16) {
    let width = area.width.min(width);
    let inner = usize::from(width.saturating_sub(4)).max(1);
    let rows: usize = lines.iter().map(|l| l.width().max(1).div_ceil(inner)).sum();
    let height = (rows as u16 + 2).min(area.height);
    let [_, column, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, rect, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(column);
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::bordered()
                .title(format!(" {title} "))
                .border_style(Style::new().fg(palette::BORDER))
                .padding(Padding::horizontal(1))
                .style(Style::new().bg(palette::VOID)),
        ),
        rect,
    );
}

pub fn draw_draft(frame: &mut Frame, area: Rect, world: &World) {
    let Some(draft) = world.pending_draft() else {
        return;
    };
    let mut lines = vec![
        Line::styled(
            format!(
                "You have grown: level {}. Choose one.",
                world.player().level - world.pending_drafts() as u32 + 1
            ),
            text(),
        ),
        Line::default(),
    ];
    for (i, &boon) in draft.iter().enumerate() {
        let (title, what) = boon_text(boon);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}  ", i + 1),
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(title, text().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(format!("   {what}"), dim()));
        lines.push(Line::default());
    }
    lines.push(Line::styled("1 / 2 / 3: choose", dim()));
    popup(frame, area, "Level up", lines, 64);
}

pub fn draw_sheet(frame: &mut Frame, area: Rect, world: &World) {
    let player = world.player();
    let (lo, hi) = world.player_damage();
    let mut lines = vec![
        Line::styled(
            format!(
                "Level {} · Insight {} / {}",
                player.level,
                player.insight,
                insight_for_next(player.level)
            ),
            text().add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                "Health {} · Accuracy {} · Defense {} · Damage {lo}–{hi}",
                player.max_health,
                world.player_accuracy(),
                world.player_defense()
            ),
            dim(),
        ),
        Line::default(),
        Line::styled("─ skills ─ (grow from use)", dim()),
    ];
    for skill in Skill::ALL {
        let def = world.content().skill(skill);
        let rank = world.rank(skill);
        let xp = player.skills.xp(skill);
        let (from, to) = (def.xp_for(rank), def.xp_for((rank + 1).min(MAX_RANK)));
        let filled = if rank >= MAX_RANK || to == from {
            BAR
        } else {
            ((xp - from) as usize * BAR / (to - from) as usize).min(BAR)
        };
        let next = def
            .techniques
            .iter()
            .find(|u| u.rank > rank)
            .map(|u| format!("  next at {}: {}", u.rank, technique_text(u.technique).0))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(format!("{:<10} ", skill_name(skill)), text()),
            Span::styled("█".repeat(filled), Style::new().fg(palette::ACCENT)),
            Span::styled("░".repeat(BAR - filled), Style::new().fg(palette::BORDER)),
            Span::styled(format!(" rank {rank}"), text()),
            Span::styled(next, dim()),
        ]));
    }

    let techniques: Vec<_> = Skill::ALL
        .iter()
        .flat_map(|&s| world.techniques(s))
        .collect();
    if !techniques.is_empty() {
        lines.push(Line::default());
        lines.push(Line::styled("─ techniques ─", dim()));
        for t in techniques {
            let (name, what) = technique_text(t);
            lines.push(Line::from(vec![
                Span::styled(format!("{name}: "), text()),
                Span::styled(what, dim()),
            ]));
        }
    }

    lines.push(Line::default());
    lines.push(Line::styled("─ boons ─", dim()));
    if player.boons.is_empty() {
        lines.push(Line::styled("None yet. Each level offers three.", dim()));
    }
    for &boon in &player.boons {
        let (title, what) = boon_text(boon);
        lines.push(Line::from(vec![
            Span::styled(format!("{title}: "), text()),
            Span::styled(what, dim()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::styled("Esc: close", dim()));
    popup(frame, area, "The acolyte", lines, 76);
}
