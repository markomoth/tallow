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
    /// An open door.
    Door,
    /// A shut door. Blocks sight. You (and the Taken and the Remnant) open it by walking into it.
    DoorClosed,
    /// A door barred by the rite of Seal. Nothing opens it but you.
    DoorSealed,
    /// The way further down.
    StairsDown,
    /// The way you came. Sealed during the descent.
    StairsUp,
    /// A burning iron bowl. Blocks movement, not sight. Gives light.
    Brazier,
    /// A brazier gone cold. Touch your candle to it to light it.
    ColdBrazier,
    /// Shelves of old books. Blocks movement and sight. Burns.
    Bookshelf,
    /// A wooden pew. Walkable. Burns.
    Pew,
    /// A bell rope on the wall. Pull it (walk into it) and the bell rings out.
    BellRope,
    /// Ankle-deep water. Slow going. Fire can't burn here.
    ShallowWater,
    /// Deep water. Wading in puts your candle out.
    DeepWater,
    /// Rotten boards. They give way under you and drop you a floor.
    RottenFloor,
    /// Where rotten boards gave way.
    Pit,
}

impl Tile {
    /// Can a creature stand on this tile?
    pub const fn is_walkable(self) -> bool {
        matches!(
            self,
            Tile::Floor
                | Tile::Door
                | Tile::DoorClosed
                | Tile::DoorSealed
                | Tile::StairsDown
                | Tile::StairsUp
                | Tile::Pew
                | Tile::ShallowWater
                | Tile::DeepWater
                | Tile::RottenFloor
        )
    }

    pub const fn is_water(self) -> bool {
        matches!(self, Tile::ShallowWater | Tile::DeepWater)
    }

    /// Does this tile stop line of sight (and light)?
    pub const fn blocks_sight(self) -> bool {
        matches!(
            self,
            Tile::Wall | Tile::DoorClosed | Tile::DoorSealed | Tile::Bookshelf | Tile::BellRope
        )
    }

    /// Worth stopping a run for when it first comes into view.
    pub const fn is_landmark(self) -> bool {
        matches!(
            self,
            Tile::StairsDown | Tile::Brazier | Tile::ColdBrazier | Tile::BellRope
        )
    }

    pub const fn is_door(self) -> bool {
        matches!(self, Tile::Door | Tile::DoorClosed | Tile::DoorSealed)
    }

    /// Turns a fire burns on this tile, if it can burn at all on its own.
    pub const fn fuel(self) -> Option<u8> {
        match self {
            Tile::Bookshelf => Some(10),
            Tile::Pew => Some(6),
            Tile::Door | Tile::DoorClosed | Tile::DoorSealed => Some(6),
            _ => None,
        }
    }

    /// Percent chance per turn that fire next door catches here.
    pub const fn catch_chance(self) -> u32 {
        match self {
            Tile::Bookshelf => 30,
            Tile::Pew => 30,
            Tile::Door | Tile::DoorClosed | Tile::DoorSealed => 15,
            _ => 0,
        }
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
