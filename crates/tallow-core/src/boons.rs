//! Boons: what a level up offers (BUILD_GUIDE.md §5).
//!
//! A boon is a passive gift, or a trigger joined to a reward ("when you kill
//! with a blade, mend 2"). Each level up drafts three, weighted toward the
//! skills you actually use, and never offers one that can't do anything yet.

use rand::Rng;
use rand::RngExt;

use crate::item::Family;
use crate::skills::{Skill, Skills};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Passive {
    /// +accuracy with everything.
    Accuracy(i32),
    Defense(i32),
    MaxHealth(u32),
    /// +1 to both ends of a weapon family's damage.
    FamilyDamage(Family),
    /// +accuracy with thrown and fired things.
    MissileAccuracy(i32),
    /// The candle burns a quarter slower.
    CandleThrift,
    /// In the dark you feel two tiles around you, not one.
    DarkSight,
    /// The dark breeds dread half again as fast.
    DarkFed,
    /// Braziers mend twice as much for what you offer them.
    BrazierKin,
    /// Mending tinctures mend half again as much.
    StrongMedicine,
    /// Tallow you find is worth a quarter more.
    TallowThief,
    /// Carry 5.0 more before you're burdened or stuck.
    PackMule,
    /// Regain health every 8 turns instead of 12.
    QuickMending,
    /// Rites cost a quarter less dread.
    RiteThrift,
}

impl Passive {
    /// Unique passives can be taken once; the rest stack.
    pub const fn unique(self) -> bool {
        !matches!(
            self,
            Passive::Accuracy(_)
                | Passive::Defense(_)
                | Passive::MaxHealth(_)
                | Passive::FamilyDamage(_)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    /// You kill a creature.
    Kill,
    /// You kill a creature with a weapon of this family.
    KillWith(Family),
    /// A thrown or fired thing hits.
    MissileHit,
    /// You snuff your candle.
    Snuff,
    /// You go down a stair.
    Descend,
    /// You drink a tincture.
    Drink,
    /// A heavy blow comes down where you no longer are.
    DodgeHeavyBlow,
    /// You cast a rite.
    Cast,
    /// You finish studying a body.
    Study,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reward {
    Heal(u32),
    /// Dread to spend on rites.
    Dread(u32),
    Tallow(u32),
    /// The nearest creature hunting you loses your trail.
    LoseTrail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boon {
    Passive(Passive),
    Triggered { when: Trigger, then: Reward },
}

pub const DRAFT_SIZE: usize = 3;

/// What the draft needs to know about the acolyte.
pub struct DraftContext<'a> {
    pub skills: &'a Skills,
    pub ranks: &'a dyn Fn(Skill) -> u32,
    pub owned: &'a [Boon],
    pub snuffs: u32,
    pub drinks: u32,
    pub heavy_blows_seen: u32,
    pub casts: u32,
    pub studies: u32,
    pub knows_rites: bool,
}

const FAMILIES: [Family; 3] = [Family::Blade, Family::Bludgeon, Family::Reach];

/// How big a reward is for a trigger: rare triggers pay more.
fn rewards(when: Trigger) -> Vec<Reward> {
    use Reward::{Dread, Heal, LoseTrail, Tallow};
    match when {
        Trigger::Kill => vec![Heal(1), Dread(2), Tallow(8)],
        Trigger::KillWith(_) => vec![Heal(2), Dread(3), Tallow(12)],
        Trigger::MissileHit => vec![Heal(1), Dread(1), Tallow(6)],
        Trigger::Snuff => vec![Dread(3), LoseTrail],
        Trigger::Descend => vec![Heal(6), Dread(10), Tallow(60)],
        Trigger::Drink => vec![Heal(4), Dread(6)],
        Trigger::DodgeHeavyBlow => vec![Heal(3), Dread(5), Tallow(20)],
        Trigger::Cast => vec![Heal(2), Dread(4), Tallow(10)],
        Trigger::Study => vec![Heal(4), Dread(6), Tallow(25)],
    }
}

/// Every boon the acolyte could usefully be offered now, with its weight.
fn candidates(ctx: &DraftContext) -> Vec<(Boon, u32)> {
    let used = |skill: Skill| ctx.skills.xp(skill) > 0;
    let weight_for = |skill: Skill| 5 + 4 * (ctx.ranks)(skill);

    let mut passives: Vec<(Passive, u32)> = vec![
        (Passive::Accuracy(5), 6),
        (Passive::Defense(2), 6),
        (Passive::MaxHealth(4), 6),
        (Passive::CandleThrift, 5),
        (Passive::DarkSight, 4),
        (Passive::DarkFed, 5),
        (Passive::BrazierKin, 4),
        (Passive::StrongMedicine, 4),
        (Passive::TallowThief, 4),
        (Passive::PackMule, 4),
        (Passive::QuickMending, 5),
    ];
    for family in FAMILIES {
        if used(Skill::of_family(family)) {
            passives.push((
                Passive::FamilyDamage(family),
                weight_for(Skill::of_family(family)),
            ));
        }
    }
    if used(Skill::Missiles) {
        passives.push((Passive::MissileAccuracy(8), weight_for(Skill::Missiles)));
    }
    if ctx.knows_rites {
        passives.push((Passive::RiteThrift, 5));
    }

    let mut triggers: Vec<(Trigger, u32)> = vec![(Trigger::Kill, 5), (Trigger::Descend, 5)];
    for family in FAMILIES {
        if used(Skill::of_family(family)) {
            triggers.push((
                Trigger::KillWith(family),
                weight_for(Skill::of_family(family)),
            ));
        }
    }
    if used(Skill::Missiles) {
        triggers.push((Trigger::MissileHit, weight_for(Skill::Missiles)));
    }
    if ctx.snuffs > 0 {
        triggers.push((Trigger::Snuff, 5));
    }
    if ctx.drinks > 0 {
        triggers.push((Trigger::Drink, 4));
    }
    if ctx.heavy_blows_seen > 0 {
        triggers.push((Trigger::DodgeHeavyBlow, 4));
    }
    if ctx.casts > 0 {
        triggers.push((Trigger::Cast, 6));
    }
    if ctx.studies > 0 {
        triggers.push((Trigger::Study, 4));
    }

    let mut all: Vec<(Boon, u32)> = passives
        .into_iter()
        .filter(|(p, _)| !(p.unique() && ctx.owned.contains(&Boon::Passive(*p))))
        .map(|(p, w)| (Boon::Passive(p), w))
        .collect();
    for (when, weight) in triggers {
        let options = rewards(when);
        let share = (weight / options.len() as u32).max(1);
        for then in options {
            let boon = Boon::Triggered { when, then };
            if !ctx.owned.contains(&boon) {
                all.push((boon, share));
            }
        }
    }
    all
}

/// Three different boons, drawn by weight without repeats.
pub fn draft<R: Rng + ?Sized>(rng: &mut R, ctx: &DraftContext) -> Vec<Boon> {
    let mut pool = candidates(ctx);
    let mut picks = Vec::new();
    while picks.len() < DRAFT_SIZE && !pool.is_empty() {
        let total: u32 = pool.iter().map(|(_, w)| w).sum();
        let mut roll = rng.random_range(0..total);
        let index = pool
            .iter()
            .position(|(_, w)| {
                if roll < *w {
                    true
                } else {
                    roll -= w;
                    false
                }
            })
            .expect("roll is below the total");
        picks.push(pool.swap_remove(index).0);
    }
    picks
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_pcg::Pcg64Mcg;

    fn ctx<'a>(skills: &'a Skills, owned: &'a [Boon]) -> DraftContext<'a> {
        DraftContext {
            skills,
            ranks: &|_| 0,
            owned,
            snuffs: 0,
            drinks: 0,
            heavy_blows_seen: 0,
            casts: 0,
            studies: 0,
            knows_rites: false,
        }
    }

    #[test]
    fn drafts_three_different_boons() {
        let skills = Skills::default();
        for seed in 0..200 {
            let picks = draft(&mut Pcg64Mcg::seed_from_u64(seed), &ctx(&skills, &[]));
            assert_eq!(picks.len(), DRAFT_SIZE);
            assert!(picks[0] != picks[1] && picks[1] != picks[2] && picks[0] != picks[2]);
        }
    }

    #[test]
    fn never_offers_dead_picks() {
        let skills = Skills::default();
        for seed in 0..500 {
            for boon in draft(&mut Pcg64Mcg::seed_from_u64(seed), &ctx(&skills, &[])) {
                assert!(
                    !matches!(
                        boon,
                        Boon::Passive(
                            Passive::FamilyDamage(_)
                                | Passive::MissileAccuracy(_)
                                | Passive::RiteThrift
                        ) | Boon::Triggered {
                            when: Trigger::KillWith(_)
                                | Trigger::MissileHit
                                | Trigger::Snuff
                                | Trigger::Drink
                                | Trigger::DodgeHeavyBlow
                                | Trigger::Cast
                                | Trigger::Study,
                            ..
                        }
                    ),
                    "seed {seed}: offered {boon:?} with nothing to power it"
                );
            }
        }
    }

    #[test]
    fn unique_boons_are_not_offered_twice() {
        let skills = Skills::default();
        let owned = [Boon::Passive(Passive::DarkSight)];
        for seed in 0..300 {
            let picks = draft(&mut Pcg64Mcg::seed_from_u64(seed), &ctx(&skills, &owned));
            assert!(!picks.contains(&Boon::Passive(Passive::DarkSight)));
        }
    }

    #[test]
    fn used_skills_show_up_more() {
        let mut skills = Skills::default();
        skills.add(Skill::Blades, 200);
        let ranks = |s: Skill| if s == Skill::Blades { 5 } else { 0 };
        let context = DraftContext {
            ranks: &ranks,
            ..ctx(&skills, &[])
        };
        let blade_boons = (0..300)
            .flat_map(|seed| draft(&mut Pcg64Mcg::seed_from_u64(seed), &context))
            .filter(|b| {
                matches!(
                    b,
                    Boon::Passive(Passive::FamilyDamage(Family::Blade))
                        | Boon::Triggered {
                            when: Trigger::KillWith(Family::Blade),
                            ..
                        }
                )
            })
            .count();
        assert!(
            blade_boons > 150,
            "blade boons should be common for a swordsman, got {blade_boons}"
        );
    }
}
