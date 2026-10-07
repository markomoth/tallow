//! The world: the run seed, the current floor, the player, and the turn counter.

use crate::actions::Command;
use crate::biome::MAX_DEPTH;
use crate::events::Event;
use crate::floor::Floor;
use crate::geom::{Direction, Point};
use crate::map::light::{LightSource, Rgb};
use crate::map::{Map, Tile, generate};
use crate::rng;

pub const CANDLE_RADIUS: i32 = 6;
pub const CANDLE_COLOR: Rgb = [255, 196, 128];

/// Safety stop for runs across very long halls.
const MAX_RUN_STEPS: u32 = 120;

#[derive(Debug, Clone)]
pub struct World {
    seed: u64,
    depth: u8,
    floor: Floor,
    player: Point,
    turn: u64,
}

impl World {
    /// A new run. The seed decides every floor.
    pub fn new(seed: u64) -> Self {
        Self::on_floor(seed, 1, Self::generate_floor(seed, 1))
    }

    /// A run starting on a hand-made map, for tests and vaults.
    pub fn from_map(map: Map, start: Point) -> Self {
        assert!(
            map.is_walkable(start),
            "player must start on a walkable tile"
        );
        Self::on_floor(0, 1, Floor::new(map, start))
    }

    fn on_floor(seed: u64, depth: u8, floor: Floor) -> Self {
        let mut world = Self {
            seed,
            depth,
            player: floor.arrival(),
            floor,
            turn: 0,
        };
        // What you see on arrival is the scene, not news.
        world.update_view();
        world
    }

    fn generate_floor(seed: u64, depth: u8) -> Floor {
        let mut rng = rng::floor_rng(seed, depth);
        let layout = generate::crypt(&mut rng, depth < MAX_DEPTH);
        Floor::new(layout.map, layout.start)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }

    pub fn floor(&self) -> &Floor {
        &self.floor
    }

    pub fn map(&self) -> &Map {
        self.floor.map()
    }

    pub fn player(&self) -> Point {
        self.player
    }

    /// How many turns have passed. Actions that fail cost nothing.
    pub fn turn(&self) -> u64 {
        self.turn
    }

    /// Resolves one player command and returns what happened.
    pub fn apply(&mut self, command: Command) -> Vec<Event> {
        match command {
            Command::Move(dir) => self.step(dir),
            Command::Run(dir) => self.run(dir),
            Command::Wait => {
                self.turn += 1;
                let mut events = vec![Event::PlayerWaited];
                events.extend(self.update_view());
                events
            }
            Command::Descend => self.descend(),
            Command::Ascend => match self.map().tile(self.player) {
                Tile::StairsUp => vec![Event::StairsSealed { depth: self.depth }],
                _ => vec![Event::NoStairsHere],
            },
        }
    }

    fn step(&mut self, dir: Direction) -> Vec<Event> {
        let target = self.player + dir;
        let tile = self.map().tile(target);
        if !tile.is_walkable() {
            return vec![Event::PlayerBlocked { at: target, tile }];
        }
        self.player = target;
        self.turn += 1;
        let mut events = vec![Event::PlayerMoved { to: target }];
        events.extend(self.update_view());
        events
    }

    /// Steps in `dir` until something worth a decision happens: a wall ahead,
    /// a door or stair underfoot or alongside, a new landmark in view, or (in a
    /// corridor) a side passage opening up.
    fn run(&mut self, dir: Direction) -> Vec<Event> {
        let sides = |world: &World| {
            [2, -2].map(|turn| world.map().is_walkable(world.player + dir.rotate(turn)))
        };
        let in_corridor = sides(self) == [false, false];
        let mut known_landmarks: Vec<Point> = self.floor.visible_landmarks().collect();
        let mut known_doors = self.doors_beside(dir);

        let mut events = Vec::new();
        for steps in 0..MAX_RUN_STEPS {
            if !self.map().is_walkable(self.player + dir) {
                if steps == 0 {
                    events.extend(self.step(dir));
                }
                break;
            }
            events.extend(self.step(dir));

            let underfoot = self.map().tile(self.player);
            let new_landmark = self
                .floor
                .visible_landmarks()
                .any(|p| !known_landmarks.contains(&p));
            let doors = self.doors_beside(dir);
            let new_door = doors.iter().any(|d| !known_doors.contains(d));
            let side_opened = in_corridor && sides(self) != [false, false];
            if matches!(underfoot, Tile::Door | Tile::StairsDown | Tile::StairsUp)
                || new_landmark
                || new_door
                || side_opened
            {
                break;
            }
            known_landmarks.extend(self.floor.visible_landmarks());
            known_doors = doors;
        }
        events
    }

    /// Doors next to the player, except the one straight ahead (a run steps onto that one).
    fn doors_beside(&self, heading: Direction) -> Vec<Point> {
        Direction::ALL
            .iter()
            .filter(|&&d| d != heading)
            .map(|&d| self.player + d)
            .filter(|&p| self.map().tile(p) == Tile::Door)
            .collect()
    }

    fn descend(&mut self) -> Vec<Event> {
        if self.map().tile(self.player) != Tile::StairsDown {
            return vec![Event::NoStairsHere];
        }
        self.depth += 1;
        self.floor = Self::generate_floor(self.seed, self.depth);
        self.player = self.floor.arrival();
        self.turn += 1;
        let mut events = vec![Event::Descended { depth: self.depth }];
        events.extend(self.update_view());
        events
    }

    fn update_view(&mut self) -> Vec<Event> {
        let candle = LightSource {
            at: self.player,
            radius: CANDLE_RADIUS,
            color: CANDLE_COLOR,
        };
        self.floor.update_view(self.player, Some(candle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::prefab;

    fn world_from(text: &str) -> World {
        let (map, start) = prefab::parse(text).unwrap();
        World::from_map(map, start)
    }

    fn small_world() -> World {
        world_from("#####\n#@..#\n#..+#\n#####")
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
    fn running_stops_at_the_wall() {
        let mut world = world_from("##########\n#@.......#\n##########");
        world.apply(Command::Run(Direction::E));
        assert_eq!(world.player(), Point::new(8, 1));
        assert_eq!(world.turn(), 7);
    }

    #[test]
    fn running_into_a_wall_is_just_a_bump() {
        let mut world = world_from("####\n#@.#\n####");
        let events = world.apply(Command::Run(Direction::N));
        assert!(matches!(events[..], [Event::PlayerBlocked { .. }]));
        assert_eq!(world.turn(), 0);
    }

    #[test]
    fn running_stops_on_a_door() {
        let mut world = world_from("##########\n#@..+....#\n##########");
        world.apply(Command::Run(Direction::E));
        assert_eq!(world.player(), Point::new(4, 1));
    }

    #[test]
    fn running_stops_at_a_side_passage() {
        let mut world = world_from(
            "##########\n\
             #@.......#\n\
             ####.#####\n\
             ####.#####",
        );
        world.apply(Command::Run(Direction::E));
        assert_eq!(world.player(), Point::new(4, 1));
    }

    #[test]
    fn running_through_a_room_ignores_its_shape_but_stops_beside_doors() {
        let mut world = world_from(
            "###########\n\
             #@........#\n\
             #.........#\n\
             ######+####",
        );
        world.apply(Command::Run(Direction::E));
        assert_eq!(
            world.player(),
            Point::new(9, 1),
            "open rooms don't stop a run"
        );

        let mut world = world_from(
            "###########\n\
             #.........#\n\
             #@........#\n\
             ######+####",
        );
        world.apply(Command::Run(Direction::E));
        assert_eq!(
            world.player(),
            Point::new(5, 2),
            "a door alongside stops it"
        );
    }

    #[test]
    fn running_stops_when_a_new_landmark_comes_into_view() {
        // The brazier sits in a side room that only becomes visible past the gap.
        let mut world = world_from(
            "#################\n\
             #@..............#\n\
             #######.#########\n\
             ####.......######\n\
             ####...&...######\n\
             #################",
        );
        let events = world.apply(Command::Run(Direction::E));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Spotted {
                tile: Tile::Brazier,
                ..
            }
        )));
        assert!(world.player().x < 15, "stopped before the far wall");
    }

    #[test]
    fn stairs_up_are_sealed_and_cost_nothing() {
        let mut world = world_from("#####\n#<@>#\n#####");
        world.apply(Command::Move(Direction::W));
        let turn = world.turn();
        assert_eq!(
            world.apply(Command::Ascend),
            vec![Event::StairsSealed { depth: 1 }]
        );
        assert_eq!(world.turn(), turn);
    }

    #[test]
    fn descending_needs_stairs() {
        let mut world = small_world();
        assert_eq!(world.apply(Command::Descend), vec![Event::NoStairsHere]);
        assert_eq!(world.depth(), 1);
    }

    /// Walks to the down-stair along the distance map and takes it.
    fn descend_once(world: &mut World) {
        let stairs = world
            .map()
            .find(Tile::StairsDown)
            .next()
            .expect("stairs exist");
        let dist = crate::map::path::distances(world.map(), stairs);
        while world.player() != stairs {
            let here = dist.at(world.player()).expect("stairs reachable");
            let dir = Direction::ALL
                .into_iter()
                .find(|&d| dist.at(world.player() + d).is_some_and(|n| n < here))
                .expect("a step closer exists");
            world.apply(Command::Move(dir));
        }
        let events = world.apply(Command::Descend);
        assert!(matches!(events[0], Event::Descended { .. }));
    }

    #[test]
    fn a_seed_reaches_the_bottom_and_the_bottom_has_no_stairs() {
        let mut world = World::new(1234);
        while world.depth() < MAX_DEPTH {
            descend_once(&mut world);
            assert_eq!(world.player(), world.floor().arrival());
            assert_eq!(world.map().tile(world.player()), Tile::StairsUp);
        }
        assert_eq!(world.map().find(Tile::StairsDown).count(), 0);
    }

    #[test]
    fn same_seed_same_dungeon() {
        let a = World::new(99);
        let b = World::new(99);
        assert_eq!(a.map(), b.map());
        assert_eq!(a.player(), b.player());
        assert_ne!(World::new(100).map(), a.map());
    }

    #[test]
    fn the_candle_lights_the_way() {
        let world = World::new(5);
        assert!(world.floor().light(world.player()).is_lit());
        assert!(world.floor().is_visible(world.player()));
    }
}
