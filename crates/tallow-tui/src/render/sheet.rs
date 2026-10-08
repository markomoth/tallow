//! The character sheet (`@`) and the level-up draft.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};
use tallow_core::progress::insight_for_next;
use tallow_core::skills::MAX_RANK;
use tallow_core::{Skill, World};

use super::palette;
use super::text::wrap_all;
use crate::names::{boon_text, rite_numbers, school_name, skill_name, technique_text};
use tallow_core::corpse::{RENDER_TURNS, study_turns};

const BAR: usize = 10;

fn dim() -> Style {
    Style::new().fg(palette::TEXT_DIM)
}

fn text() -> Style {
    Style::new().fg(palette::TEXT)
}

/// A bordered popup centered in `area`, sized to its wrapped text. Wrapped
/// lines hang under their text; `footer` (the keys) sits in the bottom
/// border, and text that doesn't fit is counted there instead of lost.
pub fn popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    lines: Vec<Line<'static>>,
    width: u16,
    footer: &str,
) {
    // A column clear of the sidebar's rule on either side.
    let width = area.width.saturating_sub(2).min(width);
    let inner = usize::from(width.saturating_sub(4)).max(1);
    let mut rows = wrap_all(&lines, inner);
    let room = usize::from(area.height.saturating_sub(2));
    let footer = if rows.len() > room {
        let hidden = rows.len() - room + 1;
        rows.truncate(room - 1);
        rows.push(Line::styled(format!("▾ {hidden} more lines"), dim()));
        format!("{footer} · make the window taller to see all")
    } else {
        footer.to_string()
    };
    let height = (rows.len() as u16 + 2).min(area.height);
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
        Paragraph::new(rows).block(frame_block(title, &footer)),
        rect,
    );
}

/// The frame every popup shares: a warm border, the title in amber, the keys
/// along the bottom.
pub fn frame_block(title: &str, footer: &str) -> Block<'static> {
    let mut block = Block::bordered()
        .border_style(Style::new().fg(palette::FRAME))
        .padding(Padding::horizontal(1))
        .style(Style::new().bg(palette::PANEL));
    if !title.is_empty() {
        block = block.title(Line::styled(
            format!(" {title} "),
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if !footer.is_empty() {
        block = block.title_bottom(Line::styled(format!(" {footer} "), dim()));
    }
    block
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
    lines.pop();
    popup(frame, area, "Level up", lines, 64, "1 / 2 / 3 choose");
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
    popup(frame, area, "The acolyte", lines, 76, "Esc close");
}

/// Width of the rite list's left pane.
const RITE_LIST: u16 = 32;

/// The rite list: read one with ↑↓, cast it by letter or Enter.
pub fn draw_rites(frame: &mut Frame, area: Rect, world: &World, selected: usize) {
    let known = world.known_rites();
    let Some(&chosen) = known.get(selected.min(known.len().saturating_sub(1))) else {
        return;
    };
    let dread = world.player().dread.value();
    let width = area.width.saturating_sub(2).min(80);
    let detail_width = usize::from(width.saturating_sub(4 + RITE_LIST + 2)).max(1);
    let detail = wrap_all(&rite_detail(world, chosen), detail_width);
    // Sized for the longest rite, so moving through the list never moves
    // the window.
    let longest = known
        .iter()
        .map(|&r| wrap_all(&rite_detail(world, r), detail_width).len())
        .max()
        .unwrap_or(0);
    let list_rows = 2 + known.len() * 2 + 2;
    let height = (list_rows.max(longest) as u16 + 2).min(area.height);

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
    let strength = known.first().map_or(100, |&r| world.rite_potency(r));
    let block = frame_block("Rites", "↑↓ read · letter or Enter cast · Esc close").title(
        Line::styled(
            format!(" dread {dread} · rites at {strength}% "),
            Style::new().fg(palette::DREAD),
        )
        .right_aligned(),
    );
    let inner = block.inner(rect);
    frame.render_widget(Clear, rect);
    frame.render_widget(block, rect);
    let [list, rule, _, right] = Layout::horizontal([
        Constraint::Length(RITE_LIST),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);

    // The list keeps the selection in view, two rows a rite.
    let mut left = vec![
        Line::styled(
            format!("your mind holds {} of {}", known.len(), world.rite_slots()),
            dim(),
        ),
        Line::styled(
            "─".repeat(usize::from(RITE_LIST)),
            Style::new().fg(palette::BORDER),
        ),
    ];
    let fits = (usize::from(list.height).saturating_sub(3) / 2).max(1);
    let first = selected.saturating_sub(fits - 1);
    for (i, &rite) in known.iter().enumerate().skip(first).take(fits) {
        let def = world.content().rite(rite);
        let recharge = world.rite_recharge(rite);
        let cost = world.rite_cost(rite);
        let (state, color) = if recharge > 0 {
            (format!("{recharge}t"), palette::TEXT_DIM)
        } else if dread < cost {
            ("low".to_string(), palette::DANGER)
        } else {
            ("ready".to_string(), palette::GOOD)
        };
        let here = i == selected;
        let bg = if here {
            palette::SELECTED
        } else {
            palette::PANEL
        };
        let name = Style::new()
            .fg(if state == "ready" {
                palette::TEXT
            } else {
                palette::TEXT_DIM
            })
            .add_modifier(if here {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        left.push(
            Line::from(vec![
                Span::styled(
                    if here { "▸ " } else { "  " },
                    Style::new().fg(palette::ACCENT),
                ),
                Span::styled(
                    format!("{} ", (b'a' + i as u8) as char),
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<15}", def.name), name),
                Span::styled(format!("{cost:>3}  "), Style::new().fg(palette::DREAD)),
                Span::styled(format!("{state:<6}"), Style::new().fg(color)),
            ])
            .style(Style::new().bg(bg)),
        );
        left.push(Line::styled(
            format!("    {}", school_name(def.school)),
            dim(),
        ));
    }
    let hidden = known.len().saturating_sub(first + fits);
    if hidden > 0 {
        left.push(Line::styled(format!("  ▾ {hidden} more"), dim()));
    } else if first > 0 {
        left.push(Line::styled(format!("  ▴ {first} above"), dim()));
    } else if let Some(next) = tallow_core::rites::RITE_SLOT_LEVELS
        .iter()
        .find(|&&level| level > world.player().level)
    {
        left.push(Line::default());
        left.push(Line::styled(
            format!("  room for one more at level {next}"),
            dim(),
        ));
    }
    frame.render_widget(Paragraph::new(left), list);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled("│", Style::new().fg(palette::BORDER));
            usize::from(rule.height)
        ]),
        rule,
    );
    frame.render_widget(Paragraph::new(detail), right);
}

/// One rite opened up: its numbers, what it does, and what stands in the way.
fn rite_detail(world: &World, rite: tallow_core::RiteId) -> Vec<Line<'static>> {
    let def = world.content().rite(rite);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                def.name.clone(),
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" · {}", school_name(def.school)), dim()),
        ]),
        Line::default(),
    ];
    lines.extend(
        rite_numbers(world, rite)
            .split(" · ")
            .map(|part| Line::styled(part.to_string(), text())),
    );
    lines.push(Line::default());
    lines.push(Line::styled(def.description.clone(), dim()));
    let recharge = world.rite_recharge(rite);
    let cost = world.rite_cost(rite);
    if recharge > 0 {
        lines.push(Line::default());
        lines.push(Line::styled(
            format!("! ready again in {recharge} turns"),
            Style::new().fg(palette::DANGER),
        ));
    } else if world.player().dread.value() < cost {
        lines.push(Line::default());
        lines.push(Line::styled(
            format!("! needs {cost} dread: gather it in the dark"),
            Style::new().fg(palette::DANGER),
        ));
    }
    lines
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
    popup(
        frame,
        area,
        "No room",
        lines,
        72,
        "letter forget that one · Esc let the new rite go",
    );
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
    ];
    popup(
        frame,
        area,
        "A body",
        lines,
        64,
        "s study · r render · Esc leave it",
    );
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
    ];
    popup(frame, area, "Help", lines, 96, "Esc close");
}

/// The journal: what all your runs have taught you, this one included.
pub fn draw_journal(frame: &mut Frame, area: Rect, app: &crate::app::App, page: Option<usize>) {
    if let Some(page) = page.and_then(|i| crate::journal::PAGES.get(i)) {
        let lines = vec![
            Line::styled(page.title, text().add_modifier(Modifier::BOLD)),
            Line::default(),
            Line::styled(page.text, text()),
        ];
        popup(frame, area, "Journal", lines, 70, "Esc back");
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
    popup(
        frame,
        area,
        "Journal",
        lines,
        90,
        "number read a page · Esc close",
    );
}
