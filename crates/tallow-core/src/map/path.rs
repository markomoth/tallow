//! Distance maps over walkable tiles.

use std::collections::VecDeque;

use super::Map;
use crate::geom::{Direction, Point};
use crate::grid::Grid;

/// Steps from `from` to every reachable walkable tile (8-way). Unreachable is `None`.
pub fn distances(map: &Map, from: Point) -> Grid<Option<u32>> {
    let mut dist = Grid::new(map.width(), map.height(), None);
    if !map.is_walkable(from) {
        return dist;
    }
    dist.set(from, Some(0));
    let mut queue = VecDeque::from([from]);
    while let Some(p) = queue.pop_front() {
        let d = dist.at(p).unwrap_or(0);
        for dir in Direction::ALL {
            let next = p + dir;
            if map.is_walkable(next) && dist.at(next).is_none() {
                dist.set(next, Some(d + 1));
                queue.push_back(next);
            }
        }
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::prefab;

    #[test]
    fn counts_steps_and_skips_unreachable() {
        let (map, start) = prefab::parse("#######\n#@..#.#\n#######").unwrap();
        let dist = distances(&map, start);
        assert_eq!(dist.at(Point::new(3, 1)), Some(2));
        assert_eq!(dist.at(Point::new(5, 1)), None);
    }
}
