//! Seeded randomness. Every random stream is derived from the run seed, so a
//! seed always produces the same dungeon.

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;

pub type GameRng = Pcg64Mcg;

/// The map-generation stream for one floor. Independent of every other floor,
/// so what happens on floor 3 never changes the layout of floor 4.
pub fn floor_rng(seed: u64, depth: u8) -> GameRng {
    GameRng::seed_from_u64(splitmix64(seed ^ splitmix64(u64::from(depth))))
}

/// Run-wide streams for things that aren't map generation. Separate streams mean
/// a change in how often monsters roll for wandering never changes combat rolls.
#[derive(Debug, Clone, Copy)]
pub enum Stream {
    Combat = 1,
    Ai = 2,
    /// Per-run item secrets, like how strong each tincture is.
    Loot = 3,
}

pub fn stream(seed: u64, stream: Stream) -> GameRng {
    GameRng::seed_from_u64(splitmix64(seed ^ splitmix64(0x1000 + stream as u64)))
}

/// A fast, well-distributed 64-bit mixer.
pub const fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;

    #[test]
    fn floors_get_distinct_reproducible_streams() {
        let a: u64 = floor_rng(7, 1).random();
        let again: u64 = floor_rng(7, 1).random();
        let b: u64 = floor_rng(7, 2).random();
        assert_eq!(a, again);
        assert_ne!(a, b);
    }
}
