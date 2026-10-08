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
    /// A hallucination: looks real, can't hurt anyone, and Look sees through it.
    pub phantom: bool,
    /// Actions left before it can move again (it can still strike).
    pub pinned: u32,
    /// Actions left under Compel: it fights for you and follows you.
    pub compelled: u32,
    /// Actions left before it can notice you again (Unsee).
    pub unseeing: u32,
    /// Actions left fleeing in terror (Transference).
    pub terrified: u32,
    /// Already holds some of your dread; it can't take more.
    pub carries_dread: bool,
    /// Another creature it is fighting instead of you.
    pub foe: Option<MonsterId>,
    /// Stepped onto holy ground: loses its next action.
    pub(crate) flinching: bool,
    /// Turned against its own kind by Turncoat.
    pub turned: bool,
    /// Actions left walking to you under Beckon, heedless of fire.
    pub beckoned: u32,
    /// Slipped on oil: loses its next action.
    pub(crate) slipping: bool,
    /// Mid-chant: the rite lands next action if it can still see you.
    pub chanting: bool,
    /// Actions until it can use its special ability again.
    pub ability_cooldown: u32,
    /// Actions spent chewing through the door ahead.
    pub(crate) gnawed: u32,
    /// Actions it loses outright, dazed.
    pub stunned: u32,
    /// Stronger than its kind: +6 health, +5 accuracy and +1 damage a step.
    pub grown: u32,
    /// Actions until it can put out your candle again.
    pub(crate) snuff_cooldown: u32,
    /// Actions left bleeding, 1 health each.
    pub bleeding: u32,
    /// Where it was put: the Remnant keep to it.
    pub post: Point,
    /// Crouched to leap at this tile next action.
    pub lunging: Option<Point>,
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
            phantom: false,
            pinned: 0,
            compelled: 0,
            unseeing: 0,
            terrified: 0,
            carries_dread: false,
            foe: None,
            flinching: false,
            turned: false,
            beckoned: 0,
            slipping: false,
            chanting: false,
            ability_cooldown: 0,
            gnawed: 0,
            stunned: 0,
            grown: 0,
            snuff_cooldown: 0,
            bleeding: 0,
            post: pos,
            lunging: None,
        }
    }
}
