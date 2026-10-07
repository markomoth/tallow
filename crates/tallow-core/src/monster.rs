//! Monsters: the creatures sharing a floor with the player.

use crate::content::{KindId, MonsterDef};
use crate::geom::Point;

slotmap::new_key_type! {
    /// A monster on the current floor. Stale ids simply stop resolving.
    pub struct MonsterId;
}

/// What a monster is doing about the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mind {
    /// Hasn't noticed the player. Wanders.
    Unaware,
    /// Chasing the player, heading for where it last saw them.
    Hunting { last_seen: Point },
    /// Running away.
    Fleeing,
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub kind: KindId,
    pub pos: Point,
    pub health: u32,
    pub max_health: u32,
    pub energy: i32,
    pub mind: Mind,
    /// Mid wind-up: the tile its heavy blow will land on next action.
    pub winding_up: Option<Point>,
    /// The player has acted since the wind-up, so the blow may land. This is
    /// what guarantees every telegraph gives you a turn to respond.
    pub(crate) blow_ready: bool,
    /// Actions until it can wind up again.
    pub cooldown: u32,
}

impl Monster {
    pub fn new(kind: KindId, def: &MonsterDef, pos: Point) -> Self {
        Self {
            kind,
            pos,
            health: def.health,
            max_health: def.health,
            energy: 0,
            mind: Mind::Unaware,
            winding_up: None,
            blow_ready: false,
            cooldown: 0,
        }
    }
}
