//! Tiles and the floor map.

pub mod prefab;

use crate::geom::Point;

/// What a single map cell is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Wall,
    Floor,
    /// An open doorway. Walkable for now; opening, closing and barring come later.
    Door,
}

impl Tile {
    /// Can a creature stand on this tile?
    pub const fn is_walkable(self) -> bool {
        matches!(self, Tile::Floor | Tile::Door)
    }
}

/// A rectangular grid of tiles. Anything outside the bounds reads as wall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    width: i32,
    height: i32,
    tiles: Vec<Tile>,
}

impl Map {
    /// A map of the given size filled with walls.
    pub fn filled(width: i32, height: i32) -> Self {
        assert!(width > 0 && height > 0, "map must have a positive size");
        Self {
            width,
            height,
            tiles: vec![Tile::Wall; (width * height) as usize],
        }
    }

    pub const fn width(&self) -> i32 {
        self.width
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    pub const fn in_bounds(&self, p: Point) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    pub fn tile(&self, p: Point) -> Tile {
        self.index(p).map_or(Tile::Wall, |i| self.tiles[i])
    }

    /// Sets a tile. Writes outside the bounds are ignored.
    pub fn set(&mut self, p: Point, tile: Tile) {
        if let Some(i) = self.index(p) {
            self.tiles[i] = tile;
        }
    }

    pub fn is_walkable(&self, p: Point) -> bool {
        self.tile(p).is_walkable()
    }

    fn index(&self, p: Point) -> Option<usize> {
        self.in_bounds(p).then(|| (p.y * self.width + p.x) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_bounds_reads_as_wall() {
        let map = Map::filled(3, 3);
        assert_eq!(map.tile(Point::new(-1, 0)), Tile::Wall);
        assert_eq!(map.tile(Point::new(3, 1)), Tile::Wall);
    }

    #[test]
    fn set_and_read_back() {
        let mut map = Map::filled(4, 4);
        map.set(Point::new(2, 1), Tile::Floor);
        assert_eq!(map.tile(Point::new(2, 1)), Tile::Floor);
        assert!(map.is_walkable(Point::new(2, 1)));
        assert!(!map.is_walkable(Point::new(1, 2)));
    }

    #[test]
    fn writes_out_of_bounds_are_ignored() {
        let mut map = Map::filled(2, 2);
        map.set(Point::new(5, 5), Tile::Floor);
        assert_eq!(map, Map::filled(2, 2));
    }
}
