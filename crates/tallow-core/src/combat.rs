//! Combat math. Kept small and visible: Look shows these exact numbers.

use rand::{Rng, RngExt};

pub const PLAYER_HEALTH: u32 = 24;
/// The acolyte's own accuracy and defense, before weapon and vestment.
pub const BASE_ACCURACY: i32 = 80;
pub const BASE_DEFENSE: i32 = 7;
/// Damage with nothing in hand.
pub const FISTS: (u32, u32) = (1, 2);

/// Striking at something that stands in the dark: your accuracy drops by this
/// much (half with the Night Eyes boon).
pub const DARK_ACCURACY: i32 = 20;
/// A creature standing in the dark strikes this much more surely, and
/// one harder at both ends of its damage.
pub const DARK_FURY_ACCURACY: i32 = 10;
pub const DARK_FURY_DAMAGE: u32 = 1;

/// Percent chance to land a blow.
pub fn hit_chance(accuracy: i32, defense: i32) -> u32 {
    (accuracy - defense).clamp(5, 95) as u32
}

/// Rolls an attack: `Some(damage)` on a hit, `None` on a miss.
pub fn roll_attack<R: Rng + ?Sized>(rng: &mut R, chance: u32, damage: (u32, u32)) -> Option<u32> {
    (rng.random_range(0..100) < chance).then(|| roll_damage(rng, damage))
}

pub fn roll_damage<R: Rng + ?Sized>(rng: &mut R, (lo, hi): (u32, u32)) -> u32 {
    rng.random_range(lo..=hi)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_pcg::Pcg64Mcg;

    #[test]
    fn hit_chance_is_clamped() {
        assert_eq!(hit_chance(85, 10), 75);
        assert_eq!(hit_chance(10, 90), 5);
        assert_eq!(hit_chance(200, 0), 95);
    }

    #[test]
    fn rolls_respect_their_ranges() {
        let mut rng = Pcg64Mcg::seed_from_u64(3);
        let mut hits = 0;
        for _ in 0..1000 {
            if let Some(d) = roll_attack(&mut rng, 50, (2, 5)) {
                assert!((2..=5).contains(&d));
                hits += 1;
            }
        }
        assert!(
            (400..600).contains(&hits),
            "about half should hit, got {hits}"
        );
    }
}
