//! Grid geometry: points and the eight movement directions.

use std::ops::Add;

/// A tile coordinate. `x` grows right, `y` grows down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}
