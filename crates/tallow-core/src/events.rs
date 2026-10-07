//! Events: everything that happened as a result of a command.
//!
//! The frontend turns events into log messages and animations.
//! Logic code never builds display strings.

use crate::geom::Point;
use crate::map::Tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The player moved to a new tile.
    PlayerMoved { to: Point },
    /// The player tried to move but something solid was in the way. Costs no time.
    PlayerBlocked { at: Point, tile: Tile },
    /// The player waited a turn.
    PlayerWaited,
    /// A landmark came into view for the first time on this floor.
    Spotted { tile: Tile, at: Point },
    /// The player went down to a new floor.
    Descended { depth: u8 },
    /// The player tried to use stairs that aren't there. Costs no time.
    NoStairsHere,
    /// The way back up is closed during the descent. Costs no time.
    StairsSealed { depth: u8 },
}
