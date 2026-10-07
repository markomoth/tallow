//! Top-level app state: the world, the log, and which screen is up.

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use tallow_core::inventory::THROW_RANGE;
use tallow_core::{Command, Event, ItemId, MonsterId, Point, RiteId, RiteTarget, World};

use crate::input::{Action, map_key};
use crate::journal::Journal;
use crate::log::{MessageLog, Tone, narrate};
use crate::save::Save;

/// How far a Shift+direction moves a cursor.
const CURSOR_JUMP: i32 = 5;

/// What the pack screen is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackPurpose {
    /// Browsing: pick an item to see it and act on it.
    Browse,
    /// Choosing something to throw.
    Throw,
}

/// What a target cursor will do when confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim {
    Throw(ItemId),
    Fire,
    Rite(RiteId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Play,
    /// Examining the map. The world is paused.
    Look {
        cursor: Point,
    },
    /// The pack screen, optionally with one item opened.
    Pack {
        purpose: PackPurpose,
        selected: Option<ItemId>,
    },
    /// Choosing where a projectile goes.
    Target {
        aim: Aim,
        cursor: Point,
    },
    /// A level-up draft is waiting. Pick 1, 2 or 3.
    Draft,
    /// The character sheet.
    Sheet,
    /// Standing on a body: study it or render it?
    Corpse,
    /// Choosing a rite to cast.
    Rites,
    /// One carried Leaving, opened from the pack.
    Leaving(tallow_core::leavings::LeavingId),
    /// The run is over; waiting for "again" or "quit".
    Dead,
    /// The candle is home.
    Won,
    /// Keys and how the game works.
    Help,
    /// What you've learned across all your runs; one history page may be open.
    Journal(Option<usize>),
}

pub struct App {
    world: World,
    log: MessageLog,
    mode: Mode,
    quit: bool,
    restart: bool,
    /// Every command given, for the save.
    commands: Vec<Command>,
    journal: Journal,
}

impl App {
    pub fn new(seed: u64) -> Self {
        let mut log = MessageLog::default();
        log.push(
            "You pry up the floorboards behind the altar and climb down.",
            Tone::Normal,
        );
        log.push(
            "The air tastes of tallow and old water. Somewhere below, a bell.",
            Tone::Normal,
        );
        Self {
            world: World::new(seed),
            log,
            mode: Mode::Play,
            quit: false,
            restart: false,
            commands: Vec::new(),
            journal: Journal::default(),
        }
    }

    /// Picks a saved run back up by replaying it.
    pub fn resume(save: &Save) -> Self {
        let mut app = App::new(save.seed);
        for &command in &save.commands {
            app.play(command);
        }
        app.log.push(
            "You wake where you left off. The candle is still burning.",
            Tone::Normal,
        );
        app
    }

    /// The run so far, as a save.
    pub fn save(&self) -> Save {
        Save {
            version: crate::save::VERSION,
            seed: self.world.seed(),
            commands: self.commands.clone(),
        }
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    pub fn set_journal(&mut self, journal: Journal) {
        self.journal = journal;
    }

    /// The run has ended in death or victory.
    pub fn is_over(&self) -> bool {
        self.world.death().is_some() || self.world.victory().is_some()
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn log(&self) -> &MessageLog {
        &self.log
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// The player asked for a new run from the death screen.
    pub fn wants_restart(&self) -> bool {
        self.restart
    }

    /// The tiles a projectile would cross, while aiming.
    pub fn aim_path(&self) -> Vec<Point> {
        match self.mode {
            Mode::Target {
                aim: Aim::Rite(_), ..
            } => Vec::new(),
            Mode::Target { aim, cursor } => self.world.aim(cursor, self.range(aim)),
            _ => Vec::new(),
        }
    }

    /// Handles a raw key. The pack screen reads letters; everything else goes
    /// through the key bindings.
    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Pack { purpose, selected } => self.handle_pack_key(key, purpose, selected),
            Mode::Draft => {
                if let KeyCode::Char(c @ '1'..='3') = key.code {
                    self.play(Command::ChooseBoon(c as usize - '1' as usize));
                }
            }
            Mode::Corpse => match key.code {
                KeyCode::Char('s') => {
                    self.mode = Mode::Play;
                    self.play(Command::Study);
                }
                KeyCode::Char('r') => {
                    self.mode = Mode::Play;
                    self.play(Command::Render);
                }
                KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Play,
                _ => {}
            },
            Mode::Rites => self.handle_rites_key(key),
            Mode::Journal(open) => match key.code {
                KeyCode::Char(c @ '1'..='9') => {
                    let mut journal = self.journal.clone();
                    journal.record(&self.world);
                    let found: Vec<usize> = crate::journal::PAGES
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| journal.pages.contains(p.id))
                        .map(|(i, _)| i)
                        .collect();
                    if let Some(&page) = found.get(c as usize - '1' as usize) {
                        self.mode = Mode::Journal(Some(page));
                    }
                }
                KeyCode::Esc if open.is_some() => self.mode = Mode::Journal(None),
                _ => {
                    if let Some(action) = map_key(key) {
                        self.handle(action);
                    }
                }
            },
            Mode::Leaving(id) => match key.code {
                KeyCode::Char('a') => {
                    self.mode = Mode::Play;
                    self.play(Command::UseLeaving(id));
                }
                KeyCode::Char('d') => {
                    self.mode = Mode::Play;
                    self.play(Command::DropLeaving(id));
                }
                KeyCode::Esc => {
                    self.mode = Mode::Pack {
                        purpose: PackPurpose::Browse,
                        selected: None,
                    }
                }
                _ => {}
            },
            _ => {
                if let Some(action) = map_key(key) {
                    self.handle(action);
                }
            }
        }
    }

    pub fn handle(&mut self, action: Action) {
        match self.mode {
            Mode::Play => self.handle_play(action),
            Mode::Look { cursor } => {
                if let Some(cursor) = self.move_cursor(action, cursor) {
                    self.mode = Mode::Look { cursor };
                } else if matches!(
                    action,
                    Action::Look | Action::Cancel | Action::Confirm | Action::Quit
                ) {
                    self.mode = Mode::Play;
                }
            }
            Mode::Target { aim, cursor } => self.handle_target(action, aim, cursor),
            Mode::Pack { .. } | Mode::Draft | Mode::Corpse | Mode::Rites | Mode::Leaving(_) => {}
            Mode::Sheet | Mode::Help | Mode::Journal(_) => {
                if matches!(
                    action,
                    Action::Cancel
                        | Action::Sheet
                        | Action::Quit
                        | Action::Confirm
                        | Action::Help
                        | Action::Journal
                ) {
                    self.mode = Mode::Play;
                }
            }
            Mode::Dead | Mode::Won => match action {
                Action::Confirm => self.restart = true,
                Action::Quit => self.quit = true,
                _ => {}
            },
        }
    }

    fn handle_play(&mut self, action: Action) {
        let command = match action {
            Action::Move(dir) => Command::Move(dir),
            Action::Run(dir) => Command::Run(dir),
            Action::Wait => Command::Wait,
            Action::Descend => Command::Descend,
            Action::Ascend => Command::Ascend,
            Action::Candle => Command::ToggleCandle,
            Action::Rest => Command::Rest,
            Action::PickUp => Command::PickUp,
            Action::CloseDoor => Command::CloseDoor,
            Action::Explore => Command::Explore,
            Action::Help => {
                self.mode = Mode::Help;
                return;
            }
            Action::Journal => {
                self.mode = Mode::Journal(None);
                return;
            }
            Action::Pack => {
                self.mode = Mode::Pack {
                    purpose: PackPurpose::Browse,
                    selected: None,
                };
                return;
            }
            Action::Throw => {
                self.mode = Mode::Pack {
                    purpose: PackPurpose::Throw,
                    selected: None,
                };
                return;
            }
            Action::Fire => {
                // A shot at your own feet is refused for free; the refusal says
                // whether the problem is the weapon, the ammunition, or the aim.
                let here = self.world.player().pos;
                let check = self.world.clone().apply(Command::Fire { target: here });
                if check == [Event::BadTarget] {
                    self.mode = Mode::Target {
                        aim: Aim::Fire,
                        cursor: self.first_target(),
                    };
                } else {
                    self.play(Command::Fire { target: here });
                }
                return;
            }
            Action::Look => {
                self.mode = Mode::Look {
                    cursor: self.first_target(),
                };
                return;
            }
            Action::Sheet => {
                self.mode = Mode::Sheet;
                return;
            }
            Action::Study => {
                if self.world.corpse_here().is_some() {
                    self.mode = Mode::Corpse;
                    return;
                }
                Command::Study
            }
            Action::Rites => {
                if self.world.known_rites().is_empty() {
                    self.log.push(
                        "You know no rites yet. Read pages (?) and study bodies to learn them.",
                        Tone::Normal,
                    );
                } else {
                    self.mode = Mode::Rites;
                }
                return;
            }
            Action::Quit => {
                self.quit = true;
                return;
            }
            Action::NextTarget | Action::Cancel | Action::Confirm => return,
        };
        self.play(command);
    }

    fn handle_target(&mut self, action: Action, aim: Aim, cursor: Point) {
        if let Some(cursor) = self.move_cursor(action, cursor) {
            self.mode = Mode::Target { aim, cursor };
            return;
        }
        match action {
            Action::Confirm | Action::Throw | Action::Fire => {
                self.mode = Mode::Play;
                self.play(match aim {
                    Aim::Throw(item) => Command::Throw {
                        item,
                        target: cursor,
                    },
                    Aim::Fire => Command::Fire { target: cursor },
                    Aim::Rite(rite) => Command::Cast {
                        rite,
                        target: cursor,
                    },
                });
            }
            Action::Cancel | Action::Quit => self.mode = Mode::Play,
            _ => {}
        }
    }

    fn handle_rites_key(&mut self, key: KeyEvent) {
        let KeyCode::Char(c) = key.code else {
            if key.code == KeyCode::Esc {
                self.mode = Mode::Play;
            }
            return;
        };
        let index = (c as u32).wrapping_sub('a' as u32) as usize;
        let Some(&rite) = self.world.known_rites().get(index) else {
            return;
        };
        match self.world.content().rite(rite).effect.target() {
            RiteTarget::Myself => {
                self.mode = Mode::Play;
                let here = self.world.player().pos;
                self.play(Command::Cast { rite, target: here });
            }
            RiteTarget::Creature => {
                self.mode = Mode::Target {
                    aim: Aim::Rite(rite),
                    cursor: self.first_target(),
                };
            }
            RiteTarget::Tile | RiteTarget::Door => {
                self.mode = Mode::Target {
                    aim: Aim::Rite(rite),
                    cursor: self.world.player().pos,
                };
            }
        }
    }

    fn handle_pack_key(&mut self, key: KeyEvent, purpose: PackPurpose, selected: Option<ItemId>) {
        let KeyCode::Char(c) = key.code else {
            if key.code == KeyCode::Esc {
                self.mode = match selected {
                    Some(_) => Mode::Pack {
                        purpose,
                        selected: None,
                    },
                    None => Mode::Play,
                };
            }
            return;
        };
        if selected.is_none()
            && purpose == PackPurpose::Browse
            && let Some(digit) = c.to_digit(10)
            && let Some(&id) = self
                .world
                .carried_leavings()
                .get((digit as usize).wrapping_sub(1))
        {
            self.mode = Mode::Leaving(id);
            return;
        }
        let Some(item) = selected else {
            // Choosing an item by its letter.
            let index = (c as u32).wrapping_sub('a' as u32) as usize;
            let Some(&item) = self.world.player().inventory.get(index) else {
                return;
            };
            let throwable = self.world.content().item(item.kind).thrown.is_some();
            if purpose == PackPurpose::Throw && !throwable {
                return;
            }
            self.mode = match purpose {
                PackPurpose::Browse => Mode::Pack {
                    purpose,
                    selected: Some(item.id),
                },
                PackPurpose::Throw => Mode::Target {
                    aim: Aim::Throw(item.id),
                    cursor: self.first_target(),
                },
            };
            return;
        };
        let command = match c {
            'e' => Command::Equip(item),
            'a' => Command::Use(item),
            'd' => Command::Drop(item),
            't' => {
                self.mode = Mode::Target {
                    aim: Aim::Throw(item),
                    cursor: self.first_target(),
                };
                return;
            }
            _ => return,
        };
        self.mode = Mode::Play;
        self.play(command);
    }

    /// Applies a command and writes what happened to the log.
    fn play(&mut self, command: Command) {
        self.commands.push(command);
        for event in self.world.apply(command) {
            if let Some((text, tone)) = narrate(&event, &self.world) {
                self.log.push(text, tone);
            }
        }
        if self.world.death().is_some() {
            self.mode = Mode::Dead;
        } else if self.world.victory().is_some() {
            self.mode = Mode::Won;
        } else if self.world.pending_draft().is_some() {
            self.mode = Mode::Draft;
        } else if self.mode == Mode::Draft {
            self.mode = Mode::Play;
        }
    }

    /// Moves a cursor for a movement action, or `None` if it isn't one.
    fn move_cursor(&self, action: Action, cursor: Point) -> Option<Point> {
        let map = self.world.map();
        let clamp = |p: Point| {
            Point::new(
                p.x.clamp(0, map.width() - 1),
                p.y.clamp(0, map.height() - 1),
            )
        };
        match action {
            Action::Move(dir) => Some(clamp(cursor + dir)),
            Action::Run(dir) => {
                let (dx, dy) = dir.delta();
                Some(clamp(Point::new(
                    cursor.x + dx * CURSOR_JUMP,
                    cursor.y + dy * CURSOR_JUMP,
                )))
            }
            Action::NextTarget => {
                let spots: Vec<Point> = self
                    .visible_monsters()
                    .iter()
                    .filter_map(|&id| self.world.floor().monster(id))
                    .map(|m| m.pos)
                    .collect();
                let next = spots.iter().position(|&p| p == cursor).map_or(0, |i| i + 1);
                Some(
                    spots
                        .get(next % spots.len().max(1))
                        .copied()
                        .unwrap_or(cursor),
                )
            }
            _ => None,
        }
    }

    /// Where a cursor starts: on the nearest creature in view, else on you.
    fn first_target(&self) -> Point {
        self.visible_monsters()
            .first()
            .and_then(|&id| self.world.floor().monster(id))
            .map_or(self.world.player().pos, |m| m.pos)
    }

    fn range(&self, aim: Aim) -> i32 {
        match aim {
            Aim::Throw(_) => THROW_RANGE,
            Aim::Fire => self.world.fire_range().unwrap_or(0),
            Aim::Rite(rite) => self.world.content().rite(rite).range,
        }
    }

    fn visible_monsters(&self) -> Vec<MonsterId> {
        self.world.floor().visible_monsters(self.world.player().pos)
    }

    #[cfg(test)]
    pub fn clear_floor_for_test(&mut self) {
        self.world.despawn_all();
    }

    /// Direct access for tests and the dev flags.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Grants Insight through the real level-up path, then refreshes the mode.
    #[cfg(test)]
    pub fn grant_insight_for_test(&mut self, amount: u32) {
        self.world.grant_insight(amount);
        if self.world.pending_draft().is_some() {
            self.mode = Mode::Draft;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;
    use tallow_core::Direction;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn look_moves_a_cursor_and_returns_to_play() {
        let mut app = App::new(3);
        let start = app.world().player().pos;
        app.handle(Action::Look);
        let Mode::Look { cursor } = app.mode() else {
            panic!("not looking")
        };
        app.handle(Action::Move(Direction::E));
        assert_eq!(
            app.mode(),
            Mode::Look {
                cursor: cursor + Direction::E
            }
        );
        assert_eq!(app.world().player().pos, start, "looking doesn't move you");
        assert_eq!(app.world().turn(), 0, "looking takes no time");
        app.handle(Action::Cancel);
        assert_eq!(app.mode(), Mode::Play);
    }

    #[test]
    fn the_pack_opens_items_by_letter_and_acts_on_them() {
        let mut app = App::new(3);
        app.handle(Action::Pack);
        app.handle_key(key('c')); // the starting tincture
        let Mode::Pack {
            selected: Some(item),
            ..
        } = app.mode()
        else {
            panic!("item not opened: {:?}", app.mode())
        };
        app.handle_key(key('a'));
        assert_eq!(app.mode(), Mode::Play);
        assert!(app.world().inventory_item(item).is_none(), "drunk");
        assert!(
            app.log()
                .entries()
                .last()
                .unwrap()
                .text
                .contains("Your wounds close")
        );
    }

    #[test]
    fn throwing_goes_through_the_pack_then_a_target() {
        let mut app = App::new(3);
        app.clear_floor_for_test();
        let world = app.world_mut();
        let here = world.player().pos;
        let stone = world.content().item_by_id("stone").unwrap();
        world.place_item(here, stone, 3);
        app.handle(Action::PickUp);
        app.handle(Action::Throw);
        let letter = (b'a' + app.world().player().inventory.len() as u8 - 1) as char;
        app.handle_key(key(letter));
        assert!(matches!(app.mode(), Mode::Target { .. }));
        app.handle(Action::Cancel);
        assert_eq!(app.mode(), Mode::Play);
    }

    #[test]
    fn a_level_up_opens_the_draft_and_a_number_chooses() {
        let mut app = App::new(3);
        let needed = tallow_core::progress::insight_for_next(1);
        app.grant_insight_for_test(needed);
        assert_eq!(app.mode(), Mode::Draft);
        app.handle(Action::Move(Direction::E)); // movement is ignored while choosing
        assert_eq!(app.mode(), Mode::Draft);
        app.handle_key(key('2'));
        assert_eq!(app.mode(), Mode::Play);
        assert_eq!(app.world().player().boons.len(), 1);
    }

    #[test]
    fn the_sheet_opens_and_closes() {
        let mut app = App::new(3);
        app.handle(Action::Sheet);
        assert_eq!(app.mode(), Mode::Sheet);
        app.handle(Action::Cancel);
        assert_eq!(app.mode(), Mode::Play);
    }

    #[test]
    fn rites_need_learning_then_cast_from_the_list() {
        let mut app = App::new(3);
        app.clear_floor_for_test();
        app.handle(Action::Rites);
        assert_eq!(app.mode(), Mode::Play);
        assert!(
            app.log()
                .entries()
                .last()
                .unwrap()
                .text
                .contains("no rites")
        );
        let shroud = app.world().content().rite_by_id("shroud").unwrap();
        let compel = app.world().content().rite_by_id("compel").unwrap();
        app.world_mut().teach_rite(shroud);
        app.world_mut().teach_rite(compel);
        app.handle(Action::Rites);
        assert_eq!(app.mode(), Mode::Rites);
        app.handle_key(key('b'));
        assert!(
            matches!(
                app.mode(),
                Mode::Target {
                    aim: Aim::Rite(_),
                    ..
                }
            ),
            "Compel needs a target"
        );
        app.handle(Action::Cancel);
        app.handle(Action::Rites);
        app.handle_key(key('a'));
        assert_eq!(app.mode(), Mode::Play);
        assert!(app.world().shrouded(), "Shroud is cast at once");
    }

    #[test]
    fn standing_on_a_body_offers_study_or_render() {
        let mut app = App::new(3);
        app.clear_floor_for_test();
        app.handle(Action::Study);
        assert_eq!(app.mode(), Mode::Play);
        assert!(app.log().entries().last().unwrap().text.contains("no body"));
    }

    #[test]
    fn a_saved_run_resumes_exactly() {
        let mut app = App::new(9);
        for _ in 0..30 {
            app.handle(Action::Explore);
            app.handle(Action::Move(Direction::E));
            if app.mode() == Mode::Draft {
                app.handle_key(key('1'));
            }
        }
        let save = app.save();
        assert!(!save.commands.is_empty());
        let resumed = App::resume(&save);
        assert_eq!(resumed.world().turn(), app.world().turn());
        assert_eq!(resumed.world().player().pos, app.world().player().pos);
        assert_eq!(resumed.world().player().health, app.world().player().health);
        assert_eq!(resumed.save(), save);
    }

    #[test]
    fn fire_without_a_sling_says_so_instead_of_aiming() {
        let mut app = App::new(3);
        app.handle(Action::Fire);
        assert_eq!(app.mode(), Mode::Play);
        assert!(
            app.log()
                .entries()
                .last()
                .unwrap()
                .text
                .contains("nothing ready to shoot")
        );
    }
}
