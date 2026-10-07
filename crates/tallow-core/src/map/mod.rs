//! Tiles, the floor map, and the algorithms that run over it.

pub mod fov;
pub mod generate;
pub mod light;
pub mod path;
pub mod prefab;

use crate::geom::Point;
use crate::grid::Grid;

/// What a single map cell is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tile {
    #[default]
    Wall,
    Floor,
    /// An open doorway. Opening, closing and barring come later.
    Door,
    /// The way further down.
    StairsDown,
    /// The way you came. Sealed during the descent.
    StairsUp,
    /// A burning iron bowl. Blocks movement, not sight. Gives light.
    Brazier,
}

impl Tile {
    /// Can a creature stand on this tile?
    pub const fn is_walkable(self) -> bool {
        matches!(
            self,
            Tile::Floor | Tile::Door | Tile::StairsDown | Tile::StairsUp
        )
    }

    /// Does this tile stop line of sight (and light)?
    pub const fn blocks_sight(self) -> bool {
        matches!(self, Tile::Wall)
    }

    /// Worth stopping a run for when it first comes into view.
    pub const fn is_landmark(self) -> bool {
        matches!(self, Tile::StairsDown | Tile::Brazier)
    }
}

/// A rectangular grid of tiles. Anything outside the bounds reads as wall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    tiles: Grid<Tile>,
}

impl Map {
    /// A map of the given size filled with walls.
    pub fn filled(width: i32, height: i32) -> Self {
        Self {
            tiles: Grid::new(width, height, Tile::Wall),
        }
    }

    pub const fn width(&self) -> i32 {
        self.tiles.width()
    }

    pub const fn height(&self) -> i32 {
        self.tiles.height()
    }

    pub const fn in_bounds(&self, p: Point) -> bool {
        self.tiles.in_bounds(p)
    }

    pub fn tile(&self, p: Point) -> Tile {
        self.tiles.at(p)
    }

    /// Sets a tile. Writes outside the bounds are ignored.
    pub fn set(&mut self, p: Point, tile: Tile) {
        self.tiles.set(p, tile);
    }

    pub fn is_walkable(&self, p: Point) -> bool {
        self.tile(p).is_walkable()
    }

    pub fn blocks_sight(&self, p: Point) -> bool {
        self.tile(p).blocks_sight()
    }

    /// Every point on the map, row by row.
    pub fn points(&self) -> impl Iterator<Item = Point> + use<> {
        self.tiles.points()
    }

    /// Every point holding the given tile.
    pub fn find(&self, tile: Tile) -> impl Iterator<Item = Point> + '_ {
        self.points().filter(move |&p| self.tile(p) == tile)
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
    fn braziers_block_walking_but_not_sight() {
        assert!(!Tile::Brazier.is_walkable());
        assert!(!Tile::Brazier.blocks_sight());
        assert!(Tile::Wall.blocks_sight());
    }
}
