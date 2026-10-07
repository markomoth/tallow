//! Dread: what the dark does to you over time (BUILD_GUIDE.md §4).
//!
//! Stored in hundredths so slow rates stay exact integers.

/// Change per turn, in hundredths of a point.
pub const PER_TURN_IN_CANDLELIGHT: i32 = 5; // +1 every 20 turns
pub const PER_TURN_IN_DARKNESS: i32 = 25; // +1 every 4 turns
pub const PER_TURN_BY_BRAZIER: i32 = -34; // −1 about every 3 turns

/// The first sight of a new kind of creature.
pub const FIRST_SIGHT: i32 = 500;
pub const FIRST_SIGHT_OF_DREAMING: i32 = 800;

/// Where dread settles after a Manifestation is killed or escaped.
pub const AFTER_MANIFESTATION: u32 = 50;

pub const MAX: u32 = 100;
const SCALE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DreadBand {
    /// 0–39.
    Calm,
    /// 40–69: whispers.
    Uneasy,
    /// 70–99: phantoms.
    Frayed,
    /// 100: a Manifestation hunts you.
    Manifest,
}

impl DreadBand {
    pub const fn of(value: u32) -> DreadBand {
        match value {
            0..=39 => DreadBand::Calm,
            40..=69 => DreadBand::Uneasy,
            70..=99 => DreadBand::Frayed,
            _ => DreadBand::Manifest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dread {
    hundredths: u32,
}

impl Dread {
    /// 0–100.
    pub fn value(&self) -> u32 {
        self.hundredths / SCALE
    }

    pub fn band(&self) -> DreadBand {
        DreadBand::of(self.value())
    }

    /// Changes dread by `hundredths`. Returns the new band if it changed.
    pub(crate) fn shift(&mut self, hundredths: i32) -> Option<DreadBand> {
        let before = self.band();
        let next = i64::from(self.hundredths) + i64::from(hundredths);
        self.hundredths = next.clamp(0, i64::from(MAX * SCALE)) as u32;
        let after = self.band();
        (after != before).then_some(after)
    }

    pub(crate) fn set(&mut self, value: u32) -> Option<DreadBand> {
        let before = self.band();
        self.hundredths = value.min(MAX) * SCALE;
        let after = self.band();
        (after != before).then_some(after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_match_the_guide() {
        assert_eq!(DreadBand::of(0), DreadBand::Calm);
        assert_eq!(DreadBand::of(40), DreadBand::Uneasy);
        assert_eq!(DreadBand::of(70), DreadBand::Frayed);
        assert_eq!(DreadBand::of(100), DreadBand::Manifest);
    }

    #[test]
    fn slow_rates_add_up_exactly_and_clamp() {
        let mut dread = Dread::default();
        for _ in 0..20 {
            dread.shift(PER_TURN_IN_CANDLELIGHT);
        }
        assert_eq!(dread.value(), 1);
        dread.shift(-10_000);
        assert_eq!(dread.value(), 0);
        dread.shift(50_000);
        assert_eq!(dread.value(), MAX);
    }

    #[test]
    fn reports_band_changes_only() {
        let mut dread = Dread::default();
        assert_eq!(dread.set(39), None);
        assert_eq!(dread.shift(100), Some(DreadBand::Uneasy));
        assert_eq!(dread.shift(1), None);
    }
}
