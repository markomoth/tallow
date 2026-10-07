//! The world: the current floor, the player, and the turn counter.

use crate::actions::Command;
use crate::events::Event;
use crate::geom::Point;
use crate::map::{Map, prefab};

#[derive(Debug, Clone)]
pub struct World {
    map: Map,
    player: Point,
    turn: u64,
}

impl World {
    /// A world on the given map with the player at `start`.
    pub fn new(map: Map, start: Point) -> Self {
        assert!(
            map.is_walkable(start),
            "player must start on a walkable tile"
        );
        Self {
            map,
            player: start,
            turn: 0,
        }
    }

    /// A fresh run on the hand-drawn Undercroft floor.
    pub fn undercroft() -> Self {
        let (map, start) = prefab::parse(prefab::UNDERCROFT)
            .expect("bundled undercroft prefab is valid; covered by tests");
        Self::new(map, start)
    }

    pub fn map(&self) -> &Map {
        &self.map
    }

    pub fn player(&self) -> Point {
        self.player
    }

    /// How many turns have passed. Blocked moves don't count.
    pub fn turn(&self) -> u64 {
        self.turn
    }

    /// Resolves one player command and returns what happened.
    pub fn apply(&mut self, command: Command) -> Vec<Event> {
        match command {
            Command::Move(dir) => {
                let target = self.player + dir;
                let tile = self.map.tile(target);
                if tile.is_walkable() {
                    self.player = target;
                    self.turn += 1;
                    vec![Event::PlayerMoved { to: target }]
                } else {
                    vec![Event::PlayerBlocked { at: target, tile }]
                }
            }
            Command::Wait => {
                self.turn += 1;
                vec![Event::PlayerWaited]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Direction;
    use crate::map::Tile;

    fn small_world() -> World {
        let (map, start) = prefab::parse("#####\n#@..#\n#..+#\n#####").unwrap();
        World::new(map, start)
    }

    #[test]
    fn moving_onto_floor_advances_the_turn() {
        let mut world = small_world();
        let events = world.apply(Command::Move(Direction::E));
        assert_eq!(world.player(), Point::new(2, 1));
        assert_eq!(world.turn(), 1);
        assert_eq!(
            events,
            vec![Event::PlayerMoved {
                to: Point::new(2, 1)
            }]
        );
    }

    #[test]
    fn walls_block_and_cost_no_time() {
        let mut world = small_world();
        let events = world.apply(Command::Move(Direction::N));
        assert_eq!(world.player(), Point::new(1, 1));
        assert_eq!(world.turn(), 0);
        assert_eq!(
            events,
            vec![Event::PlayerBlocked {
                at: Point::new(1, 0),
                tile: Tile::Wall
            }]
        );
    }

    #[test]
    fn diagonal_moves_and_doors_are_walkable() {
        let mut world = small_world();
        world.apply(Command::Move(Direction::SE));
        world.apply(Command::Move(Direction::E));
        assert_eq!(world.player(), Point::new(3, 2));
        assert_eq!(world.map().tile(world.player()), Tile::Door);
    }

    #[test]
    fn waiting_advances_the_turn() {
        let mut world = small_world();
        assert_eq!(world.apply(Command::Wait), vec![Event::PlayerWaited]);
        assert_eq!(world.turn(), 1);
    }

    #[test]
    fn undercroft_world_starts_on_floor() {
        let world = World::undercroft();
        assert!(world.map().is_walkable(world.player()));
    }
}
