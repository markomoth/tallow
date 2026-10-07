//! Auto-explore (`o`): walk toward the nearest unknown ground until
//! something new comes into view or anything happens (BUILD_GUIDE.md §1).

use std::collections::VecDeque;

use crate::events::Event;
use crate::geom::{Direction, Point};
use crate::grid::Grid;
use crate::map::Tile;
use crate::world::{World, is_routine};

/// Safety stop for one auto-explore.
const MAX_EXPLORE_STEPS: u32 = 300;

impl World {
    pub(crate) fn explore(&mut self) -> Vec<Event> {
        if !self.hostiles_in_view().is_empty() {
            return vec![Event::RunRefused];
        }
        let mut events = Vec::new();
        for steps in 0..MAX_EXPLORE_STEPS {
            let Some(dir) = self.explore_step() else {
                if steps == 0 {
                    events.push(Event::Explored);
                }
                break;
            };
            let step = self.player_step(dir);
            let stop = step
                .iter()
                .any(|e| !is_routine(e) && !matches!(e, Event::DoorOpened { .. }))
                || step.iter().any(|e| {
                    matches!(
                        e,
                        Event::Spotted { .. }
                            | Event::SpottedTallow { .. }
                            | Event::SpottedItem { .. }
                            | Event::SpottedLeaving { .. }
                            | Event::SeepSpotted
                    )
                });
            events.extend(step);
            if stop || self.death.is_some() || !self.hostiles_in_view().is_empty() {
                break;
            }
        }
        events
    }

    /// The first step toward the nearest known, safe tile beside unknown ground.
    fn explore_step(&self) -> Option<Direction> {
        let here = self.player.pos;
        let map = self.map();
        let safe = |p: Point| {
            map.is_walkable(p)
                && self.floor.is_explored(p)
                && !self.floor.is_burning(p)
                && !matches!(map.tile(p), Tile::DeepWater | Tile::RottenFloor)
                && self.anomaly_at(p).is_none_or(|a| !a.revealed)
                && !self.floor.is_seep(p)
                && self.floor.monster_at(p).is_none()
        };
        let frontier = |p: Point| {
            Direction::ALL
                .iter()
                .any(|&d| map.in_bounds(p + d) && !self.floor.is_explored(p + d))
        };
        let mut first: Grid<Option<Direction>> = Grid::new(map.width(), map.height(), None);
        let mut seen: Grid<bool> = Grid::new(map.width(), map.height(), false);
        seen.set(here, true);
        let mut queue = VecDeque::new();
        for d in Direction::ALL {
            let p = here + d;
            if safe(p) {
                seen.set(p, true);
                first.set(p, Some(d));
                queue.push_back(p);
            }
        }
        while let Some(p) = queue.pop_front() {
            if frontier(p) {
                return first.at(p);
            }
            for d in Direction::ALL {
                let q = p + d;
                if !seen.at(q) && safe(q) {
                    seen.set(q, true);
                    first.set(q, first.at(p));
                    queue.push_back(q);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Command;
    use crate::map::prefab;

    #[test]
    fn explores_until_something_new_shows_up() {
        let (map, start) = prefab::parse(
            "##########################\n\
             #@.......................#\n\
             ###########.##############\n\
             #.........................#\n\
             ##########################",
        )
        .unwrap();
        let mut world = World::from_map(map, start);
        world.apply(Command::ToggleCandle);
        let before = world.turn();
        world.apply(Command::Explore);
        assert!(world.turn() > before, "it walked");
    }

    #[test]
    fn a_known_floor_has_nothing_left() {
        let (map, start) = prefab::parse("#####\n#@..#\n#####").unwrap();
        let mut world = World::from_map(map, start);
        assert_eq!(world.apply(Command::Explore), vec![Event::Explored]);
    }

    #[test]
    fn explore_walks_a_whole_floor_eventually() {
        let mut world = World::new(11);
        world.despawn_all();
        for _ in 0..200 {
            let events = world.apply(Command::Explore);
            if events.contains(&Event::Explored) {
                break;
            }
            world.despawn_all();
        }
        let open = world
            .map()
            .points()
            .filter(|&p| world.map().is_walkable(p))
            .count();
        let known = world
            .map()
            .points()
            .filter(|&p| world.map().is_walkable(p) && world.floor().is_explored(p))
            .count();
        assert!(known * 10 >= open * 9, "explored {known} of {open}");
    }
}
