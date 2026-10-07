//! Commands: everything the player can ask the world to do.

use crate::geom::Direction;

/// A request from the player. The world decides what actually happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Step one tile. Later this also bumps to attack and opens doors.
    Move(Direction),
    /// Keep stepping in one direction until something interesting happens.
    Run(Direction),
    /// Let one turn pass.
    Wait,
    /// Take the stairs down, if standing on them.
    Descend,
    /// Try the stairs up, if standing on them.
    Ascend,
}
