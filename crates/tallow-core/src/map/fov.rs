//! Field of view: symmetric shadowcasting.
//!
//! After Albert Ford's algorithm: if A can see B, B can see A. Slopes are
//! exact fractions, so results never depend on float rounding.

use crate::geom::Point;

/// Calls `reveal` for every tile visible from `origin` within `radius`,
/// including the origin and the blocking tiles that bound the view.
/// `reveal` may be called more than once for the same tile.
pub fn compute(
    origin: Point,
    radius: i32,
    blocks: impl Fn(Point) -> bool,
    mut reveal: impl FnMut(Point),
) {
    reveal(origin);
    let reach = radius * radius + radius;
    for quadrant in Quadrant::ALL {
        let mut rows = vec![Row {
            depth: 1,
            start: Slope::new(-1, 1),
            end: Slope::new(1, 1),
        }];
        while let Some(mut row) = rows.pop() {
            if row.depth > radius {
                continue;
            }
            let mut prev_wall: Option<bool> = None;
            for col in row.min_col()..=row.max_col() {
                let p = quadrant.transform(origin, row.depth, col);
                let wall = blocks(p);
                if (wall || row.is_symmetric(col)) && col * col + row.depth * row.depth <= reach {
                    reveal(p);
                }
                match (prev_wall, wall) {
                    (Some(true), false) => row.start = Slope::of_tile(row.depth, col),
                    (Some(false), true) => {
                        let mut next = row.next();
                        next.end = Slope::of_tile(row.depth, col);
                        rows.push(next);
                    }
                    _ => {}
                }
                prev_wall = Some(wall);
            }
            if prev_wall == Some(false) {
                rows.push(row.next());
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Quadrant {
    North,
    East,
    South,
    West,
}

impl Quadrant {
    const ALL: [Quadrant; 4] = [
        Quadrant::North,
        Quadrant::East,
        Quadrant::South,
        Quadrant::West,
    ];

    fn transform(self, origin: Point, depth: i32, col: i32) -> Point {
        match self {
            Quadrant::North => Point::new(origin.x + col, origin.y - depth),
            Quadrant::South => Point::new(origin.x + col, origin.y + depth),
            Quadrant::East => Point::new(origin.x + depth, origin.y + col),
            Quadrant::West => Point::new(origin.x - depth, origin.y + col),
        }
    }
}

/// A slope `num / den` with `den > 0`.
#[derive(Clone, Copy)]
struct Slope {
    num: i32,
    den: i32,
}

impl Slope {
    const fn new(num: i32, den: i32) -> Self {
        Self { num, den }
    }

    /// The slope to the left edge of a tile.
    const fn of_tile(depth: i32, col: i32) -> Self {
        Self::new(2 * col - 1, 2 * depth)
    }
}

#[derive(Clone, Copy)]
struct Row {
    depth: i32,
    start: Slope,
    end: Slope,
}

impl Row {
    /// `round_ties_up(depth * start)`
    fn min_col(&self) -> i32 {
        (2 * self.depth * self.start.num + self.start.den).div_euclid(2 * self.start.den)
    }

    /// `round_ties_down(depth * end)`
    fn max_col(&self) -> i32 {
        -((self.end.den - 2 * self.depth * self.end.num).div_euclid(2 * self.end.den))
    }

    fn is_symmetric(&self, col: i32) -> bool {
        col * self.start.den >= self.depth * self.start.num
            && col * self.end.den <= self.depth * self.end.num
    }

    fn next(&self) -> Row {
        Row {
            depth: self.depth + 1,
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Map, Tile, prefab};
    use std::collections::HashSet;

    fn visible_from(map: &Map, origin: Point, radius: i32) -> HashSet<Point> {
        let mut seen = HashSet::new();
        compute(
            origin,
            radius,
            |p| map.blocks_sight(p),
            |p| {
                seen.insert(p);
            },
        );
        seen
    }

    #[test]
    fn open_room_is_fully_visible_with_its_walls() {
        let (map, start) = prefab::parse("#####\n#...#\n#.@.#\n#...#\n#####").unwrap();
        let seen = visible_from(&map, start, 10);
        assert_eq!(seen.len(), 25);
    }

    #[test]
    fn walls_cast_shadows() {
        let (map, start) = prefab::parse(
            "#######\n\
             #@....#\n\
             ###.###\n\
             #.....#\n\
             #######",
        )
        .unwrap();
        let seen = visible_from(&map, start, 10);
        assert!(seen.contains(&Point::new(5, 1)));
        assert!(!seen.contains(&Point::new(1, 3)), "hidden behind the wall");
    }

    #[test]
    fn radius_limits_sight() {
        let mut map = Map::filled(30, 3);
        for x in 1..29 {
            map.set(Point::new(x, 1), Tile::Floor);
        }
        let seen = visible_from(&map, Point::new(1, 1), 5);
        assert!(seen.contains(&Point::new(6, 1)));
        assert!(!seen.contains(&Point::new(7, 1)));
    }

    #[test]
    fn sight_is_symmetric() {
        let (map, _) = prefab::parse(
            "##############\n\
             #@...#.......#\n\
             #..#...##....#\n\
             #......#..#..#\n\
             ###.####.....#\n\
             #.....#...#..#\n\
             ##############",
        )
        .unwrap();
        let floors: Vec<Point> = map.points().filter(|&p| map.is_walkable(p)).collect();
        for &a in &floors {
            let from_a = visible_from(&map, a, 20);
            for &b in &floors {
                if from_a.contains(&b) {
                    assert!(
                        visible_from(&map, b, 20).contains(&a),
                        "{a:?} sees {b:?} but not the reverse"
                    );
                }
            }
        }
    }
}
