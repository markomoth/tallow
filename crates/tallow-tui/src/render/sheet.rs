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
use tallow_core::corpse::{RENDER_TURNS, study_turns};

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
                "Dread {dread} · rites at {}% strength. Rites are paid in dread, and the more you hold, the stronger they are.",
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
        let cost = world.rite_cost(rite);
        let recharge = world.rite_recharge(rite);
        let warn = if recharge > 0 {
            Span::styled(
                format!("  (ready in {recharge} turns)"),
                Style::new().fg(palette::DANGER),
            )
        } else if dread < cost {
            Span::styled(
                format!("  (needs {cost} dread; gather it in the dark)"),
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
    lines.push(Line::styled(
        format!(
            "You hold {} of {} rites you have room for. More room comes at levels 4, 7 and 10.",
            world.known_rites().len(),
            world.rite_slots()
        ),
        dim(),
    ));
    lines.push(Line::styled("letter: cast · Esc: close", dim()));
    popup(frame, area, "Rites", lines, 72);
}

/// A rite learned with no room for it: forget one, or let it go.
pub fn draw_forget(frame: &mut Frame, area: Rect, world: &World) {
    let Some(offered) = world.pending_rite() else {
        return;
    };
    let new = world.content().rite(offered);
    let mut lines = vec![
        Line::styled(
            format!(
                "The rite of {} ({}) is yours if you make room.",
                new.name,
                school_name(new.school)
            ),
            text().add_modifier(Modifier::BOLD),
        ),
        Line::styled(format!("   {}", rite_numbers(world, offered)), text()),
        Line::styled(format!("   {}", new.description), dim()),
        Line::default(),
        Line::styled(
            format!(
                "Your mind holds {} rites. Forget one for it:",
                world.rite_slots()
            ),
            text(),
        ),
    ];
    for (i, &rite) in world.known_rites().iter().enumerate() {
        let def = world.content().rite(rite);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}  ", (b'a' + i as u8) as char),
                Style::new().fg(palette::ACCENT),
            ),
            Span::styled(def.name.clone(), text().add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", rite_numbers(world, rite)), dim()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::styled(
        "letter: forget that one · Esc: let the new rite go",
        dim(),
    ));
    popup(frame, area, "No room", lines, 72);
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
    let render = if corpse.spoiled {
        "r  render it: you opened it up to study it, and the fat is spoiled".to_string()
    } else {
        format!(
            "r  render it ({left} turns): +{} tallow. The body is gone after.",
            world.corpse_tallow(corpse)
        )
    };
    let lines = vec![
        Line::styled(
            format!("The body of {}.", crate::log::with_article(&def.name)),
            text().add_modifier(Modifier::BOLD),
        ),
        Line::default(),
        Line::styled(study, text()),
        Line::styled(render, text()),
        Line::default(),
        Line::styled(
            "Study or render, not both: studying opens the body up and spoils the fat. Rendering smells: things nearby come to it. Left alone a body rots, and big ones hatch flies. Either task stops if anything appears; your work keeps.",
            dim(),
        ),
        Line::styled("Esc: leave it", dim()),
    ];
    popup(frame, area, "A body", lines, 64);
}

/// Keys, and the few rules worth knowing before you go down.
pub fn draw_help(frame: &mut Frame, area: Rect) {
    let key = |k: &'static str, what: &'static str| {
        Line::from(vec![
            Span::styled(format!("{k:<14}"), Style::new().fg(palette::ACCENT)),
            Span::styled(what, text()),
        ])
    };
    let lines = vec![
        key(
            "hjkl yubn",
            "move (arrows and numpad too); walk into things to fight, open, ring, light",
        ),
        key("HJKL…", "run until something happens"),
        key("o", "explore until something new is in view"),
        key(". R", "wait a turn · rest until healed"),
        key(
            "g i",
            "pick up · pack (letters for items, numbers for Leavings)",
        ),
        key("t f", "throw · fire your sling or crossbow"),
        key("s z", "study or render the body underfoot · cast a rite"),
        key("c C", "snuff or light your candle · shut doors beside you"),
        key(
            "S G",
            "shove a creature back (then a direction) · guard: wait braced, answer misses",
        ),
        key("> <", "go down · go up (only with the Vigil Candle)"),
        key(
            "F",
            "flare the Vigil Candle: drive the Following back (once a floor)",
        ),
        key("x @ M", "look · your sheet · the journal"),
        key("q", "save and quit"),
        Line::default(),
        Line::styled("─ how it works ─", dim()),
        Line::styled(
            "Your candle is your light and your clock: a turn of tallow each turn. Little lies about; render bodies for it (the Taken render richest). Past 400 it is carried as heavy lumps.",
            dim(),
        ),
        Line::styled(
            "Light is seen from afar: burn it long on one floor and new things come looking. In the dark you strike worse and they strike harder.",
            dim(),
        ),
        Line::styled(
            "Dread gathers in the dark and in fights there. Rites are paid in dread, and the more you hold the stronger they are. At 100, a Manifestation hunts you.",
            dim(),
        ),
        Line::styled(
            "Walk into a lit brazier to give it dread for health. With your candle out, some things glow: letters on walls, Leavings, the Dreaming.",
            dim(),
        ),
        Line::styled(
            "Anything that can kill you is announced first: red tiles, a chant, a warning in the log. Look (x) shows real hit chances.",
            dim(),
        ),
        Line::default(),
        Line::styled("Esc: close", dim()),
    ];
    popup(frame, area, "Help", lines, 92);
}

/// The journal: what all your runs have taught you, this one included.
pub fn draw_journal(frame: &mut Frame, area: Rect, app: &crate::app::App, page: Option<usize>) {
    if let Some(page) = page.and_then(|i| crate::journal::PAGES.get(i)) {
        let lines = vec![
            Line::styled(page.title, text().add_modifier(Modifier::BOLD)),
            Line::default(),
            Line::styled(page.text, text()),
            Line::default(),
            Line::styled("Esc: back", dim()),
        ];
        popup(frame, area, "Journal", lines, 70);
        return;
    }
    let world = app.world();
    let mut journal = app.journal().clone();
    journal.record(world);
    let content = world.content();
    let mut lines = vec![
        Line::styled(
            format!(
                "Runs {} · deaths {} · candles brought home {} · deepest floor {}",
                journal.runs, journal.deaths, journal.wins, journal.deepest
            ),
            text().add_modifier(Modifier::BOLD),
        ),
        Line::default(),
        Line::styled("─ creatures met ─ (* studied)", dim()),
    ];
    let names: Vec<String> = journal
        .creatures
        .iter()
        .filter_map(|(id, studied)| {
            let kind = content.kind_by_id(id)?;
            let name = &content.monster(kind).name;
            Some(if *studied {
                format!("{name}*")
            } else {
                name.clone()
            })
        })
        .collect();
    lines.push(Line::styled(
        if names.is_empty() {
            "None yet.".into()
        } else {
            names.join(", ")
        },
        text(),
    ));
    lines.push(Line::default());
    lines.push(Line::styled("─ rites known in some life ─", dim()));
    lines.push(Line::styled(
        if journal.rites.is_empty() {
            "None yet.".to_string()
        } else {
            journal.rites.iter().cloned().collect::<Vec<_>>().join(", ")
        },
        text(),
    ));
    lines.push(Line::default());
    lines.push(Line::styled("─ Leavings held ─", dim()));
    lines.push(Line::styled(
        if journal.leavings.is_empty() {
            "None yet.".to_string()
        } else {
            journal
                .leavings
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        },
        text(),
    ));
    lines.push(Line::default());
    lines.push(Line::styled("─ pages of the town's history ─", dim()));
    for (i, page) in crate::journal::PAGES
        .iter()
        .filter(|p| journal.pages.contains(p.id))
        .enumerate()
    {
        lines.push(Line::from(vec![
            Span::styled(format!("{}  ", i + 1), Style::new().fg(palette::ACCENT)),
            Span::styled(page.title, text()),
        ]));
    }
    let missing =
        crate::journal::PAGES.len() - journal.pages.len().min(crate::journal::PAGES.len());
    if missing > 0 {
        lines.push(Line::styled(
            format!("{missing} pages still missing. Go deeper."),
            dim(),
        ));
    }
    lines.push(Line::default());
    lines.push(Line::styled("number: read a page · Esc: close", dim()));
    popup(frame, area, "Journal", lines, 90);
}
