//! Skills that grow from use, and the techniques their ranks unlock.

use std::collections::HashMap;

use serde::Deserialize;

use crate::item::Family;

pub const MAX_RANK: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Skill {
    Blades,
    Bludgeons,
    Reach,
    Missiles,
    Endurance,
}

impl Skill {
    pub const ALL: [Skill; 5] = [
        Skill::Blades,
        Skill::Bludgeons,
        Skill::Reach,
        Skill::Missiles,
        Skill::Endurance,
    ];

    pub const fn of_family(family: Family) -> Skill {
        match family {
            Family::Blade => Skill::Blades,
            Family::Bludgeon => Skill::Bludgeons,
            Family::Reach => Skill::Reach,
        }
    }
}

/// Something a rank unlocks. Numbers are percent unless named otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Technique {
    /// When a creature misses you in melee, strike back at once.
    Riposte { chance: u32 },
    /// A bludgeon hit may cost the creature its next action.
    Stagger { chance: u32 },
    /// Strike a creature two tiles away in a straight line.
    LongReach { accuracy: i32 },
    /// A missile hit may pin the creature in place.
    Pin { chance: u32, actions: u32 },
    /// Heavy blows that land on you do less.
    Brace { percent: u32 },
}

impl Technique {
    fn same_kind(self, other: Technique) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Unlock {
    pub rank: u32,
    pub technique: Technique,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillDef {
    pub skill: Skill,
    pub description: String,
    pub xp_factor: u32,
    pub accuracy_per_rank: i32,
    pub defense_per_rank: i32,
    pub techniques: Vec<Unlock>,
}

impl SkillDef {
    /// Total experience needed to reach `rank`.
    pub fn xp_for(&self, rank: u32) -> u32 {
        self.xp_factor * rank * rank
    }

    pub fn rank_for(&self, xp: u32) -> u32 {
        (0..=MAX_RANK)
            .rev()
            .find(|&r| xp >= self.xp_for(r))
            .unwrap_or(0)
    }

    /// The strongest version of each technique unlocked at `rank`.
    pub fn techniques_at(&self, rank: u32) -> Vec<Technique> {
        let mut known: Vec<Technique> = Vec::new();
        for unlock in self.techniques.iter().filter(|u| u.rank <= rank) {
            known.retain(|t| !t.same_kind(unlock.technique));
            known.push(unlock.technique);
        }
        known
    }
}

/// Experience per skill.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Skills {
    xp: HashMap<Skill, u32>,
}

impl Skills {
    pub fn xp(&self, skill: Skill) -> u32 {
        self.xp.get(&skill).copied().unwrap_or(0)
    }

    pub(crate) fn add(&mut self, skill: Skill, amount: u32) {
        *self.xp.entry(skill).or_default() += amount;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;

    #[test]
    fn every_skill_is_defined_once() {
        let content = Content::bundled();
        for skill in Skill::ALL {
            assert_eq!(
                content.skills.iter().filter(|d| d.skill == skill).count(),
                1,
                "{skill:?}"
            );
        }
    }

    #[test]
    fn ranks_follow_the_square_curve() {
        let blades = Content::bundled().skill(Skill::Blades);
        assert_eq!(blades.rank_for(0), 0);
        assert_eq!(blades.rank_for(7), 0);
        assert_eq!(blades.rank_for(8), 1);
        assert_eq!(blades.rank_for(72), 3);
        assert_eq!(blades.rank_for(1_000_000), MAX_RANK);
    }

    #[test]
    fn later_techniques_replace_earlier_ones() {
        let blades = Content::bundled().skill(Skill::Blades);
        assert!(blades.techniques_at(2).is_empty());
        assert_eq!(
            blades.techniques_at(3),
            vec![Technique::Riposte { chance: 50 }]
        );
        assert_eq!(
            blades.techniques_at(9),
            vec![Technique::Riposte { chance: 100 }]
        );
    }
}
