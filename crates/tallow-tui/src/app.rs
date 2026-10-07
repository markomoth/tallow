//! Top-level app state: the world, the log, and which screen is up.

use tallow_core::{Command, MonsterId, Point, World};

use crate::input::Action;
use crate::log::{MessageLog, narrate};

/// How far a Shift+direction moves the Look cursor.
const LOOK_JUMP: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Play,
    /// Examining the map. The world is paused.
    Look {
        cursor: Point,
    },
    /// The run is over; waiting for "again" or "quit".
    Dead,
}

pub struct App {
    world: World,
    log: MessageLog,
    mode: Mode,
    quit: bool,
    restart: bool,
}

impl App {
    pub fn new(seed: u64) -> Self {
        let mut log = MessageLog::default();
        log.push("You pry up the floorboards behind the altar and climb down.");
        log.push("The air tastes of tallow and old water. Somewhere below, a bell.");
        Self {
            world: World::new(seed),
            log,
            mode: Mode::Play,
            quit: false,
            restart: false,
        }
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

    pub fn handle(&mut self, action: Action) {
        match self.mode {
            Mode::Play => self.handle_play(action),
            Mode::Look { cursor } => self.handle_look(action, cursor),
            Mode::Dead => match action {
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
            Action::Look => {
                let cursor = self
                    .visible_monsters()
                    .first()
                    .and_then(|&id| self.world.floor().monster(id))
                    .map_or(self.world.player().pos, |m| m.pos);
                self.mode = Mode::Look { cursor };
                return;
            }
            Action::Quit => {
                self.quit = true;
                return;
            }
            Action::NextTarget | Action::Cancel | Action::Confirm => return,
        };
        let content = self.world.content();
        for event in self.world.apply(command) {
            if let Some(text) = narrate(&event, content) {
                self.log.push(text);
            }
        }
        if self.world.death().is_some() {
            self.mode = Mode::Dead;
        }
    }

    fn handle_look(&mut self, action: Action, cursor: Point) {
        let map = self.world.map();
        let clamp = |p: Point| {
            Point::new(
                p.x.clamp(0, map.width() - 1),
                p.y.clamp(0, map.height() - 1),
            )
        };
        let cursor = match action {
            Action::Move(dir) => clamp(cursor + dir),
            Action::Run(dir) => {
                let (dx, dy) = dir.delta();
                clamp(Point::new(
                    cursor.x + dx * LOOK_JUMP,
                    cursor.y + dy * LOOK_JUMP,
                ))
            }
            Action::NextTarget => {
                let spots: Vec<Point> = self
                    .visible_monsters()
                    .iter()
                    .filter_map(|&id| self.world.floor().monster(id))
                    .map(|m| m.pos)
                    .collect();
                let next = spots.iter().position(|&p| p == cursor).map_or(0, |i| i + 1);
                spots
                    .get(next % spots.len().max(1))
                    .copied()
                    .unwrap_or(cursor)
            }
            Action::Look | Action::Cancel | Action::Confirm | Action::Quit => {
                self.mode = Mode::Play;
                return;
            }
            Action::Wait | Action::Descend | Action::Ascend => cursor,
        };
        self.mode = Mode::Look { cursor };
    }

    #[cfg(test)]
    pub fn clear_floor_for_test(&mut self) {
        self.world.despawn_all();
    }

    #[cfg(test)]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    fn visible_monsters(&self) -> Vec<MonsterId> {
        self.world.floor().visible_monsters(self.world.player().pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tallow_core::Direction;

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
}
