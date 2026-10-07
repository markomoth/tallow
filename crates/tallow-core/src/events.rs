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
}
