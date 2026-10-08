//! Screen layout: map on the left, status on the right, log along the bottom.

mod hud;
mod map;
mod pack;
mod palette;
mod sheet;
mod simple;
mod title;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};
use tallow_core::{Cause, MAX_DEPTH, World};

use crate::app::{App, Mode};
use crate::log::{MessageLog, Tone, with_article};

/// How many log lines the death screen replays.
const LAST_MOMENTS: usize = 5;

pub const MIN_WIDTH: u16 = 100;
pub const MIN_HEIGHT: u16 = 30;
const SIDEBAR_WIDTH: u16 = 26;
const LOG_HEIGHT: u16 = 6;

/// Draws one frame. `time` is seconds since start, for visual effects only.
pub fn draw(frame: &mut Frame, app: &App, time: f32) {
    // Simple mode holds still: no flicker, pulse or shimmer.
    let time = if app.simple() { 0.0 } else { time };
    draw_screen(frame, app, time);
    if app.simple() {
        simple::simplify(frame.buffer_mut());
    }
}

fn draw_screen(frame: &mut Frame, app: &App, time: f32) {
    let area = frame.area();
    frame.render_widget(Block::new().style(Style::new().bg(palette::VOID)), area);

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    match app.mode() {
        Mode::Title(choice) => return title::draw(frame, area, app, choice, time),
        // The journal opened from the start menu sits over the church.
        Mode::Journal(page) if app.on_title() => {
            title::draw(frame, area, app, crate::app::TitleChoice::Journal, time);
            return sheet::draw_journal(frame, area, app, page);
        }
        _ => {}
    }

    let [top, log_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(LOG_HEIGHT)]).areas(area);
    let [map_area, hud_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(SIDEBAR_WIDTH)]).areas(top);

    let cursor = match app.mode() {
        Mode::Look { cursor } | Mode::Target { cursor, .. } => Some(cursor),
        Mode::Title(_)
        | Mode::Play
        | Mode::Dead
        | Mode::Won
        | Mode::Help
        | Mode::Journal(_)
        | Mode::Pack { .. }
        | Mode::Draft
        | Mode::Forget
        | Mode::Sheet
        | Mode::Corpse
        | Mode::Rites
        | Mode::Leaving(_) => None,
    };
    frame.render_widget(
        map::MapView::new(app.world(), time, cursor, app.aim_path()),
        map_area,
    );
    frame.render_widget(hud::Hud::new(app), hud_area);
    draw_log(frame, log_area, app.log());
    match app.mode() {
        Mode::Dead => draw_death(frame, map_area, app),
        Mode::Won => draw_victory(frame, map_area, app),
        Mode::Help => sheet::draw_help(frame, map_area),
        Mode::Journal(page) => sheet::draw_journal(frame, map_area, app, page),
        Mode::Pack { purpose, selected } => {
            pack::draw(frame, map_area, app.world(), purpose, selected)
        }
        Mode::Draft => sheet::draw_draft(frame, map_area, app.world()),
        Mode::Forget => sheet::draw_forget(frame, map_area, app.world()),
        Mode::Sheet => sheet::draw_sheet(frame, map_area, app.world()),
        Mode::Rites => sheet::draw_rites(frame, map_area, app.world()),
        Mode::Corpse => sheet::draw_corpse(frame, map_area, app.world()),
        Mode::Leaving(id) => pack::draw_leaving(frame, map_area, app.world(), id),
        Mode::Title(_) | Mode::Play | Mode::Look { .. } | Mode::Target { .. } => {}
    }
}

fn draw_death(frame: &mut Frame, area: Rect, app: &App) {
    let world = app.world();
    let Some(death) = world.death() else { return };
    let dim = Style::new().fg(palette::TEXT_DIM);
    let text = Style::new().fg(palette::TEXT);

    let mut lines = vec![
        Line::styled(
            "Your candle goes out.",
            Style::new()
                .fg(palette::DANGER)
                .add_modifier(Modifier::BOLD),
        ),
        Line::default(),
        Line::styled(
            if death.ascent > 0 {
                format!(
                    "{} on the way up, ascent {} of {}, turn {}.",
                    cause_line(world, death.cause),
                    death.ascent,
                    tallow_core::throne::ASCENT_FLOORS,
                    death.turn
                )
            } else {
                format!(
                    "{} on floor {} of {MAX_DEPTH}, turn {}.",
                    cause_line(world, death.cause),
                    death.depth,
                    death.turn
                )
            },
            text,
        ),
        Line::default(),
        Line::styled("─ last moments ─", dim),
    ];
    let recent: Vec<_> = app.log().entries().rev().take(LAST_MOMENTS).collect();
    lines.extend(
        recent
            .into_iter()
            .rev()
            .map(|e| Line::styled(e.text.clone(), dim)),
    );
    lines.extend([
        Line::default(),
        Line::styled(format!("seed {}", world.seed()), dim),
        Line::styled("Enter  begin again        q  quit", text),
    ]);

    let width = area.width.min(72);
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
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_style(Style::new().fg(palette::BORDER))
                .padding(Padding::horizontal(1))
                .style(Style::new().bg(palette::VOID)),
        ),
        popup,
    );
}

fn draw_victory(frame: &mut Frame, area: Rect, app: &App) {
    let world = app.world();
    let Some(v) = world.victory() else { return };
    let text = Style::new().fg(palette::TEXT);
    let dim = Style::new().fg(palette::TEXT_DIM);
    let mut lines = vec![
        Line::styled(
            "The Vigil Candle burns on the altar.",
            Style::new().fg(palette::GOOD).add_modifier(Modifier::BOLD),
        ),
        Line::default(),
    ];
    // The epilogue follows how you went down and came back.
    lines.push(Line::styled(
        "Morning comes to the parish of the Low Bell. One by one, the dreamers wake.",
        text,
    ));
    if v.exorcised > 0 {
        lines.push(Line::styled(
            format!(
                "{} of the Taken you freed below find their way home over the next days, thin and silent, and are taken in.",
                v.exorcised
            ),
            text,
        ));
    } else {
        lines.push(Line::styled(
            "Some never come back up. The parish buries empty coffins for them, and does not talk about it.",
            text,
        ));
    }
    lines.push(Line::styled(
        match v.dread {
            0..=39 => "You sleep that night, and dream of nothing at all.",
            40..=69 => "You sleep badly for a year. Then less badly.",
            _ => "You do not sleep well again. You keep a candle lit, always, and no one asks why.",
        },
        text,
    ));
    if v.rites >= 8 {
        lines.push(Line::styled(
            "What you learned below stays with you. The Collegium had a new scholar, for a while, though it never knew it.",
            text,
        ));
    }
    lines.extend([
        Line::default(),
        Line::styled(
            format!(
                "Level {} · turn {} · seed {}",
                v.level,
                v.turn,
                world.seed()
            ),
            dim,
        ),
        Line::styled("Enter  begin again        q  quit", text),
    ]);
    sheet::popup(frame, area, "Home", lines, 72);
}

fn cause_line(world: &World, cause: Cause) -> String {
    // The great ones are named, not counted: "the Sexton", not "a Sexton".
    let who = |kind| {
        let def = world.content().monster(kind);
        if def.boss {
            format!("the {}", def.name)
        } else {
            with_article(&def.name)
        }
    };
    match cause {
        Cause::Attack(kind) => format!("Killed by {}", who(kind)),
        Cause::HeavyBlow(kind) => format!("Crushed by {}'s heavy blow", who(kind)),
        Cause::Fire => "Burned to death".to_string(),
        Cause::Chant(kind) => format!("Undone by {}'s chanted rite", who(kind)),
        Cause::Fall => "Killed by a fall".to_string(),
        Cause::Leaving => "Taken by a Leaving's price".to_string(),
    }
}

fn draw_log(frame: &mut Frame, area: Rect, log: &MessageLog) {
    let block = Block::new()
        .borders(Borders::TOP)
        .border_style(Style::new().fg(palette::BORDER));
    let inner = block.inner(area);
    let (rows, width) = (usize::from(inner.height), usize::from(inner.width));

    // Newest entry at the bottom in full color; older ones fade. Long entries wrap.
    let mut lines: Vec<Line> = Vec::new();
    for (age, entry) in log.entries().rev().enumerate() {
        if lines.len() >= rows {
            break;
        }
        let color = tone_color(entry.tone, age == 0);
        let text = if entry.count > 1 {
            format!("{} (×{})", entry.text, entry.count)
        } else {
            entry.text.clone()
        };
        for row in wrap(&text, width).into_iter().rev() {
            lines.push(Line::styled(row, Style::new().fg(color)));
        }
    }
    lines.truncate(rows);
    lines.reverse();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Log colors: each tone has a bright form for the newest line and a dim one after.
fn tone_color(tone: Tone, newest: bool) -> ratatui::style::Color {
    let bright = match tone {
        Tone::Normal => palette::TEXT,
        Tone::Dread => palette::DREAD,
        Tone::Danger => palette::DANGER,
        Tone::Good => palette::GOOD,
    };
    if newest {
        bright
    } else if tone == Tone::Normal {
        palette::TEXT_DIM
    } else {
        dim(bright)
    }
}

fn dim(color: ratatui::style::Color) -> ratatui::style::Color {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => {
            let f = |c: u8| (u16::from(c) * 3 / 5) as u8;
            ratatui::style::Color::Rgb(f(r), f(g), f(b))
        }
        other => other,
    }
}

/// Greedy word wrap to `width` columns. Words longer than a line are split.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > width {
            if !row.is_empty() {
                rows.push(std::mem::take(&mut row));
            }
            let split: String = word.chars().take(width).collect();
            word = word.chars().skip(width).collect();
            rows.push(split);
        }
        let needed = row.chars().count() + usize::from(!row.is_empty()) + word.chars().count();
        if needed > width && !row.is_empty() {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(&word);
    }
    if !row.is_empty() || rows.is_empty() {
        rows.push(row);
    }
    rows
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let message = format!(
        "Tallow needs a terminal of at least {MIN_WIDTH}×{MIN_HEIGHT}.\n\
         This one is {}×{}. Enlarge the window or shrink the font.",
        area.width, area.height
    );
    let [_, middle, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::new().fg(palette::TEXT))
            .centered()
            .wrap(Wrap { trim: true }),
        middle,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Action;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Renders a frame to text, one line per row. Set `SHOW_SCREEN=1` to print it.
    fn render(width: u16, height: u16, app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, app, 0.0)).unwrap();
        let buffer = terminal.backend().buffer();
        let screen: String = buffer
            .content()
            .chunks(usize::from(width))
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>() + "\n")
            .collect();
        if std::env::var_os("SHOW_SCREEN").is_some() {
            println!("{screen}");
        }
        screen
    }

    /// A cleared floor with one monster of `id` hunting the player from the east.
    fn scene(id: &str) -> App {
        use tallow_core::Direction;
        let mut app = App::new(7);
        app.clear_floor_for_test();
        let world = app.world_mut();
        let here = world.player().pos;
        let spot = Direction::ALL
            .into_iter()
            .map(|d| here + d)
            .find(|&p| world.map().is_walkable(p))
            .unwrap();
        let kind = world.content().kind_by_id(id).unwrap();
        world.spawn_monster(kind, spot);
        app
    }

    #[test]
    fn a_raised_blow_is_flagged_in_view() {
        let mut app = scene("pallbearer");
        for _ in 0..20 {
            if app.world().telegraphs().next().is_some() {
                break;
            }
            app.handle(Action::Wait);
        }
        assert!(
            app.world().telegraphs().next().is_some(),
            "the pallbearer wound up"
        );
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("─ in view ─"));
        assert!(screen.contains("pallbearer !"));
        assert!(
            screen.contains("A pallbearer. A big man"),
            "introduced on first sight"
        );
        assert!(screen.contains("raises something heavy over you. Move!"));
    }

    #[test]
    fn look_shows_hit_chances() {
        let mut app = scene("gnawer");
        app.handle(Action::Look);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("─ look ─"));
        assert!(screen.contains("You hit it   70%"));
        assert!(screen.contains("It hits you  50% · 1–2"));
    }

    #[test]
    fn death_screen_recaps_the_end() {
        let mut app = scene("parishioner");
        for _ in 0..500 {
            if app.mode() == Mode::Dead {
                break;
            }
            app.handle(Action::Wait);
        }
        assert_eq!(app.mode(), Mode::Dead);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("Your candle goes out."));
        assert!(screen.contains("Killed by a Taken parishioner on floor 1 of 12"));
        assert!(screen.contains("─ last moments ─"));
        assert!(screen.contains("Enter  begin again"));
        app.handle(Action::Confirm);
        assert!(app.wants_restart());
    }

    #[test]
    fn full_screen_shows_player_hud_and_log() {
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &App::new(7));
        assert!(screen.contains('@'));
        assert!(screen.contains("T A L L O W"));
        assert!(screen.contains("floorboards"));
        assert!(screen.contains("Floor 1/12 · Level 1"));
        assert!(screen.contains("seed 7"));
    }

    #[test]
    fn the_start_menu_fits_the_smallest_screen() {
        let mut app = App::new(7);
        app.show_title();
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        let rows: Vec<&str> = screen.lines().collect();
        assert!(rows[0].contains('|'), "the spire's cross isn't cut off");
        assert!(screen.contains("T A L L O W"));
        assert!(screen.contains("▸ p  Play now"));
        assert!(screen.contains("q  Quit"));
        assert!(!screen.contains("floorboards"), "the run isn't shown yet");
    }

    #[test]
    fn descending_updates_the_hud_and_log() {
        use tallow_core::{Direction, Tile, map::path};

        let mut app = App::new(7);
        app.clear_floor_for_test();
        let stairs = app.world().map().find(Tile::StairsDown).next().unwrap();
        let dist = path::distances(app.world().map(), stairs);
        while app.world().player().pos != stairs {
            let here = app.world().player().pos;
            let dir = Direction::ALL
                .into_iter()
                .find(|&d| dist.at(here + d).is_some_and(|n| Some(n) < dist.at(here)))
                .unwrap();
            app.handle(Action::Move(dir));
        }
        app.handle(Action::Descend);

        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("Floor 2/12"));
        assert!(screen.contains("The stair turns more times than it should."));
    }

    #[test]
    fn the_pack_lists_gear_weight_and_tallow() {
        let mut app = App::new(7);
        app.handle(Action::Pack);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains(" Pack "));
        assert!(screen.contains("iron candlestick (in hand)"));
        assert!(screen.contains("cassock (worn)"));
        assert!(screen.contains("mending tincture (untried)"));
        assert!(screen.contains("tallow, 300 turns of light"));
        assert!(screen.contains("Load 8.3"));
    }

    #[test]
    fn an_opened_item_shows_stats_and_actions() {
        let mut app = App::new(7);
        app.handle(Action::Pack);
        app.handle_key(ratatui::crossterm::event::KeyEvent::new(
            ratatui::crossterm::event::KeyCode::Char('a'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        ));
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("Damage 2–5 · accuracy +5 · Bludgeon"));
        assert!(screen.contains("e take off"));
    }

    #[test]
    fn aiming_shows_the_flight_path() {
        let mut app = scene("parishioner");
        let world = app.world_mut();
        let stone = world.content().item_by_id("stone").unwrap();
        let here = world.player().pos;
        world.place_item(here, stone, 2);
        app.handle(Action::PickUp);
        app.handle(Action::Throw);
        let letter = (b'a' + app.world().player().inventory.len() as u8 - 1) as char;
        app.handle_key(ratatui::crossterm::event::KeyEvent::new(
            ratatui::crossterm::event::KeyCode::Char(letter),
            ratatui::crossterm::event::KeyModifiers::NONE,
        ));
        assert!(!app.aim_path().is_empty());
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("─ aim ─"));
        assert!(screen.contains("Throwing a stone."));
        assert!(screen.contains("At the Taken"));
    }

    #[test]
    fn the_draft_offers_three_numbered_boons() {
        let mut app = App::new(7);
        app.grant_insight_for_test(tallow_core::progress::insight_for_next(1));
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains(" Level up "));
        assert!(screen.contains("level 2. Choose one."));
        assert!(screen.contains("1  ") && screen.contains("2  ") && screen.contains("3  "));
    }

    #[test]
    fn the_sheet_shows_skills_and_what_unlocks_next() {
        let mut app = App::new(7);
        app.handle(Action::Sheet);
        let screen = render(MIN_WIDTH + 10, MIN_HEIGHT + 4, &app);
        assert!(screen.contains("Level 1 · Insight"));
        assert!(screen.contains("Blades"));
        assert!(screen.contains("next at 3: Riposte (50%)"));
        assert!(screen.contains("None yet."));
    }

    #[test]
    fn the_rite_list_shows_cost_and_strength() {
        let mut app = App::new(7);
        let compel = app.world().content().rite_by_id("compel").unwrap();
        app.world_mut().teach_rite(compel);
        app.handle(Action::Rites);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains(" Rites "));
        assert!(screen.contains("Compel"));
        assert!(screen.contains("costs 15 dread · range 6 · 12 actions"));
    }

    #[test]
    fn the_ascent_shows_the_following_and_the_altar_wins() {
        let mut app = App::new(7);
        app.world_mut().dev_ascent(1);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("Ascent 1/4"));
        assert!(screen.contains("The Following:"));
        app.world_mut().dev_ascent(5);
        let world = app.world_mut();
        let altar = world.map().find(tallow_core::Tile::Altar).next().unwrap();
        while world.player().pos.x + 1 < altar.x {
            world.apply(tallow_core::Command::Move(tallow_core::Direction::E));
        }
        let dy = (altar.y - world.player().pos.y).signum();
        let dir = match dy {
            -1 => tallow_core::Direction::NE,
            1 => tallow_core::Direction::SE,
            _ => tallow_core::Direction::E,
        };
        app.handle(Action::Move(dir));
        assert_eq!(app.mode(), Mode::Won);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("The Vigil Candle burns on the altar."));
    }

    #[test]
    fn simple_mode_uses_plain_colors_and_the_terminals_background() {
        let mut app = scene("pallbearer");
        app.set_simple(true);
        let mut terminal = Terminal::new(TestBackend::new(MIN_WIDTH, MIN_HEIGHT)).unwrap();
        terminal.draw(|frame| draw(frame, &app, 3.7)).unwrap();
        for cell in terminal.backend().buffer().content() {
            assert!(
                !matches!(cell.fg, ratatui::style::Color::Rgb(..)),
                "{cell:?}"
            );
            assert!(
                !matches!(cell.bg, ratatui::style::Color::Rgb(..)),
                "{cell:?}"
            );
        }
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(screen.contains("T A L L O W") && screen.contains('@'));
    }

    #[test]
    fn wrap_breaks_on_words_and_splits_long_ones() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("", 5), vec![""]);
    }

    #[test]
    fn long_log_lines_wrap_instead_of_being_cut() {
        let mut app = scene("parishioner");
        app.handle(Action::Wait);
        let screen = render(MIN_WIDTH, MIN_HEIGHT, &app);
        assert!(
            screen.contains("strikes without anger."),
            "the end of the description is on screen"
        );
    }

    #[test]
    fn small_terminal_gets_a_message_instead_of_a_broken_layout() {
        let screen = render(60, 20, &App::new(7));
        assert!(screen.contains("at least 100×30"));
        assert!(!screen.contains('@'));
    }
}
