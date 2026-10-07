//! Which part of the labyrinth a depth belongs to (see BUILD_GUIDE.md §3).

pub const MAX_DEPTH: u8 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    Crypts,
    Collegium,
    DrownedStacks,
    RotCourt,
    Throne,
}

impl Biome {
    pub const fn for_depth(depth: u8) -> Biome {
        match depth {
            0..=3 => Biome::Crypts,
            4..=6 => Biome::Collegium,
            7..=9 => Biome::DrownedStacks,
            10..=11 => Biome::RotCourt,
            _ => Biome::Throne,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_bands_match_the_guide() {
        assert_eq!(Biome::for_depth(1), Biome::Crypts);
        assert_eq!(Biome::for_depth(4), Biome::Collegium);
        assert_eq!(Biome::for_depth(9), Biome::DrownedStacks);
        assert_eq!(Biome::for_depth(11), Biome::RotCourt);
        assert_eq!(Biome::for_depth(MAX_DEPTH), Biome::Throne);
    }
}
