//! The candle: your light and your clock (BUILD_GUIDE.md §4).

use crate::map::light::Rgb;

/// Tallow at the start of a run, in turns of burning.
pub const START_TALLOW: u32 = 900;
pub const RADIUS: i32 = 6;
/// Light radius once the candle is nearly spent.
pub const GUTTER_RADIUS: i32 = 3;
pub const COLOR: Rgb = [255, 196, 128];

/// Below this the HUD and log warn you.
pub const LOW_AT: u32 = 200;
/// Below this the flame shrinks to `GUTTER_RADIUS`.
pub const GUTTER_AT: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleState {
    Lit,
    /// Lit, but nearly spent: smaller light.
    Guttering,
    /// Put out on purpose. Tallow is saved.
    Snuffed,
    /// No tallow left.
    Out,
}

/// A threshold the candle crossed while burning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BurnWarning {
    Low,
    Guttering,
    BurnedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candle {
    tallow: u32,
    lit: bool,
}

impl Default for Candle {
    fn default() -> Self {
        Self {
            tallow: START_TALLOW,
            lit: true,
        }
    }
}

impl Candle {
    /// Turns of burning left.
    pub fn tallow(&self) -> u32 {
        self.tallow
    }

    pub fn is_lit(&self) -> bool {
        self.lit && self.tallow > 0
    }

    pub fn state(&self) -> CandleState {
        match (self.tallow, self.lit) {
            (0, _) => CandleState::Out,
            (_, false) => CandleState::Snuffed,
            (t, true) if t < GUTTER_AT => CandleState::Guttering,
            _ => CandleState::Lit,
        }
    }

    /// How far it lights, or `None` when not burning.
    pub fn radius(&self) -> Option<i32> {
        match self.state() {
            CandleState::Lit => Some(RADIUS),
            CandleState::Guttering => Some(GUTTER_RADIUS),
            CandleState::Snuffed | CandleState::Out => None,
        }
    }

    /// Burns one turn of tallow if lit, reporting any threshold crossed.
    pub(crate) fn burn(&mut self) -> Option<BurnWarning> {
        if !self.is_lit() {
            return None;
        }
        self.tallow -= 1;
        match self.tallow {
            0 => Some(BurnWarning::BurnedOut),
            t if t == GUTTER_AT - 1 => Some(BurnWarning::Guttering),
            t if t == LOW_AT - 1 => Some(BurnWarning::Low),
            _ => None,
        }
    }

    pub(crate) fn add(&mut self, tallow: u32) {
        self.tallow += tallow;
    }

    /// Takes tallow off the candle without putting it out.
    pub(crate) fn shed(&mut self, tallow: u32) {
        self.tallow = self.tallow.saturating_sub(tallow);
    }

    /// Takes tallow away (something ate it) and puts the flame out.
    pub(crate) fn eat(&mut self, tallow: u32) {
        self.tallow = self.tallow.saturating_sub(tallow);
        self.lit = false;
    }

    pub(crate) fn snuff(&mut self) {
        self.lit = false;
    }

    /// Lights the candle. Fails when there is no tallow.
    pub(crate) fn light(&mut self) -> bool {
        self.lit = self.tallow > 0;
        self.lit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burns_down_through_its_warnings() {
        let mut candle = Candle {
            tallow: LOW_AT,
            lit: true,
        };
        assert_eq!(candle.burn(), Some(BurnWarning::Low));
        let mut warnings = Vec::new();
        while let Some(t) = (candle.tallow() > 0).then(|| candle.burn()) {
            warnings.extend(t);
        }
        assert_eq!(
            warnings,
            vec![BurnWarning::Guttering, BurnWarning::BurnedOut]
        );
        assert_eq!(candle.state(), CandleState::Out);
        assert_eq!(candle.radius(), None);
    }

    #[test]
    fn guttering_shrinks_the_light() {
        let candle = Candle {
            tallow: GUTTER_AT - 1,
            lit: true,
        };
        assert_eq!(candle.radius(), Some(GUTTER_RADIUS));
        assert_eq!(Candle::default().radius(), Some(RADIUS));
    }

    #[test]
    fn snuffed_candles_keep_their_tallow() {
        let mut candle = Candle::default();
        candle.snuff();
        assert_eq!(candle.burn(), None);
        assert_eq!(candle.tallow(), START_TALLOW);
        assert!(candle.light());
        assert!(candle.is_lit());
    }

    #[test]
    fn a_spent_candle_will_not_light() {
        let mut candle = Candle {
            tallow: 0,
            lit: false,
        };
        assert!(!candle.light());
        candle.add(10);
        assert!(candle.light());
    }
}
