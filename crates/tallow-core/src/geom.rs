//! Grid geometry: points and the eight movement directions.

use std::ops::Add;

/// A tile coordinate. `x` grows right, `y` grows down.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Steps needed with diagonal moves allowed.
    pub const fn chebyshev(self, other: Point) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        if dx > dy { dx } else { dy }
    }

    pub const fn distance_squared(self, other: Point) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

impl Add<Direction> for Point {
    type Output = Point;

    fn add(self, dir: Direction) -> Point {
        let (dx, dy) = dir.delta();
        Point::new(self.x + dx, self.y + dy)
    }
}

/// One of the eight compass directions a creature can step in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl Direction {
    /// Clockwise from north.
    pub const ALL: [Direction; 8] = [
        Direction::N,
        Direction::NE,
        Direction::E,
        Direction::SE,
        Direction::S,
        Direction::SW,
        Direction::W,
        Direction::NW,
    ];

    pub const CARDINAL: [Direction; 4] = [Direction::N, Direction::E, Direction::S, Direction::W];

    pub const fn delta(self) -> (i32, i32) {
        match self {
            Direction::N => (0, -1),
            Direction::NE => (1, -1),
            Direction::E => (1, 0),
            Direction::SE => (1, 1),
            Direction::S => (0, 1),
            Direction::SW => (-1, 1),
            Direction::W => (-1, 0),
            Direction::NW => (-1, -1),
        }
    }

    /// Rotates by `steps` eighths of a turn; positive is clockwise.
    pub fn rotate(self, steps: i32) -> Direction {
        let index = Self::ALL.iter().position(|&d| d == self).unwrap_or(0) as i32;
        Self::ALL[(index + steps).rem_euclid(8) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_a_direction_steps_one_tile() {
        let p = Point::new(5, 5);
        assert_eq!(p + Direction::N, Point::new(5, 4));
        assert_eq!(p + Direction::SE, Point::new(6, 6));
        assert_eq!(p + Direction::W, Point::new(4, 5));
    }

    #[test]
    fn every_direction_is_a_unit_step() {
        for dir in Direction::ALL {
            let (dx, dy) = dir.delta();
            assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0));
        }
    }

    #[test]
    fn rotation_wraps_both_ways() {
        assert_eq!(Direction::N.rotate(2), Direction::E);
        assert_eq!(Direction::N.rotate(-2), Direction::W);
        assert_eq!(Direction::NW.rotate(1), Direction::N);
    }

    #[test]
    fn distances() {
        let a = Point::new(0, 0);
        assert_eq!(a.chebyshev(Point::new(3, -5)), 5);
        assert_eq!(a.distance_squared(Point::new(3, 4)), 25);
    }
}
