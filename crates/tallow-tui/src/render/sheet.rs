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
use crate::names::{boon_text, rite_numbers, school_name, skill_name, technique_text};
use tallow_core::corpse::{RENDER_TURNS, render_yield, study_turns};

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
            .map(|u| {
                format!(
                    "  next at {}: {}",
                    u.rank,
                    technique_text(skill, u.technique).0
                )
            })
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
        .flat_map(|&s| world.techniques(s).into_iter().map(move |t| (s, t)))
        .collect();
    if !techniques.is_empty() {
        lines.push(Line::default());
        lines.push(Line::styled("─ techniques ─", dim()));
        for (s, t) in techniques {
            let (name, what) = technique_text(s, t);
            lines.push(Line::from(vec![
                Span::styled(format!("{name}: "), text()),
                Span::styled(what, dim()),
            ]));
        }
    }

    lines.push(Line::default());
    lines.push(Line::styled("─ rites ─ (z to cast)", dim()));
    if world.known_rites().is_empty() {
        lines.push(Line::styled(
            "None yet. Read pages and study bodies to learn them.",
            dim(),
        ));
    }
    for &rite in world.known_rites() {
        let def = world.content().rite(rite);
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", def.name), text()),
            Span::styled(format!("({}) ", school_name(def.school)), dim()),
            Span::styled(rite_numbers(world, rite), dim()),
        ]));
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

/// The rite list: pick one by letter.
pub fn draw_rites(frame: &mut Frame, area: Rect, world: &World) {
    let dread = world.player().dread.value();
    let mut lines = vec![
        Line::styled(
            format!(
                "Dread {dread} · rites at {}% strength. Dread pays for rites, and dread makes them stronger.",
                world
                    .known_rites()
                    .first()
                    .map_or(100, |&r| world.rite_potency(r))
            ),
            dim(),
        ),
        Line::default(),
    ];
    for (i, &rite) in world.known_rites().iter().enumerate() {
        let def = world.content().rite(rite);
        let letter = (b'a' + i as u8) as char;
        let after = dread + world.rite_cost(rite);
        let warn = if after >= 100 {
            Span::styled(
                "  (dread would reach 100: a Manifestation comes)",
                Style::new().fg(palette::DANGER),
            )
        } else {
            Span::raw("")
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{letter}  "), Style::new().fg(palette::ACCENT)),
            Span::styled(def.name.clone(), text().add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", school_name(def.school)), dim()),
        ]));
        lines.push(Line::from(vec![
            Span::styled(format!("   {}", rite_numbers(world, rite)), text()),
            warn,
        ]));
        lines.push(Line::styled(format!("   {}", def.description), dim()));
    }
    lines.push(Line::default());
    lines.push(Line::styled("letter: cast · Esc: close", dim()));
    popup(frame, area, "Rites", lines, 72);
}

/// Standing on a body: what studying or rendering it would take and give.
pub fn draw_corpse(frame: &mut Frame, area: Rect, world: &World) {
    let Some(corpse) = world.corpse_here() else {
        return;
    };
    let def = world.content().monster(corpse.kind);
    let study = if world.study_would_teach(corpse.kind) {
        let left = study_turns(def.threat).saturating_sub(corpse.studied);
        let what = if world.has_studied(corpse.kind) {
            "another rite it can teach"
        } else {
            "its ways, Insight, and perhaps a rite"
        };
        format!("s  study it ({left} turns): {what}")
    } else {
        "s  study it: it has nothing more to teach you".into()
    };
    let left = RENDER_TURNS.saturating_sub(corpse.rendered);
    let lines = vec![
        Line::styled(
            format!("The body of {}.", crate::log::with_article(&def.name)),
            text().add_modifier(Modifier::BOLD),
        ),
        Line::default(),
        Line::styled(study, text()),
        Line::styled(
            format!(
                "r  render it ({left} turns): +{} tallow. The body is gone after.",
                render_yield(def.health)
            ),
            text(),
        ),
        Line::default(),
        Line::styled(
            "Left alone it rots, and big bodies hatch flies. Either task stops if anything appears; your work keeps.",
            dim(),
        ),
        Line::styled("Esc: leave it", dim()),
    ];
    popup(frame, area, "A body", lines, 64);
}
