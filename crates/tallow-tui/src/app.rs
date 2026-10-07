//! Top-level app state: the world, the log, and whether we're quitting.

use tallow_core::World;

use crate::input::Action;
use crate::log::{MessageLog, narrate};

pub struct App {
    world: World,
    log: MessageLog,
    quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut log = MessageLog::default();
        log.push("You pry up the floorboards behind the altar and climb down.");
        log.push("The air tastes of tallow and old water. Somewhere below, a bell.");
        Self {
            world: World::undercroft(),
            log,
            quit: false,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn log(&self) -> &MessageLog {
        &self.log
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn handle(&mut self, action: Action) {
        match action {
            Action::Game(command) => {
                for event in self.world.apply(command) {
                    if let Some(text) = narrate(&event) {
                        self.log.push(text);
                    }
                }
            }
            Action::Quit => self.quit = true,
        }
    }
}
