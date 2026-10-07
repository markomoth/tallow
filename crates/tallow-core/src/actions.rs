//! Commands: everything the player can ask the world to do.

use crate::geom::{Direction, Point};
use crate::item::ItemId;

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
    /// Snuff the candle, or light it again.
    ToggleCandle,
    /// Wait until healed or until something happens.
    Rest,
    /// Pick up everything here.
    PickUp,
    Drop(ItemId),
    /// Equip, or take off if already equipped.
    Equip(ItemId),
    /// Drink a tincture, or equip gear.
    Use(ItemId),
    Throw {
        item: ItemId,
        target: Point,
    },
    /// Shoot the equipped ranged weapon.
    Fire {
        target: Point,
    },
    /// Take one boon from the waiting level-up draft. Costs no time.
    ChooseBoon(usize),
}
