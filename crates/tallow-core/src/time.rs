//! Energy-based turns. Every tick each creature gains its speed in energy and
//! acts once it has enough. Speed 10 is one action per turn.

pub const ACTION_COST: i32 = 100;
pub const TICKS_PER_TURN: u64 = 10;
pub const PLAYER_SPEED: i32 = 10;

/// Turns between regaining one point of health.
pub const REGEN_TURNS: u64 = 12;
