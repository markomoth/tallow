//! How items are named on screen. Tinctures show what you've learned of them.

use tallow_core::{ItemClass, ItemKindId, Potency, SideEffect, World};

use crate::log::with_article;

/// "iron candlestick", "3 stones", "strong mending tincture (bitter)",
/// "mending tincture (untried)".
pub fn item_name(world: &World, kind: ItemKindId, count: u32) -> String {
    let def = world.content().item(kind);
    let base = if count == 1 {
        def.name.clone()
    } else {
        format!("{count} {}", def.plural)
    };
    let Some(lore) = world.tincture_lore(kind) else {
        return base;
    };
    if !lore.known {
        return format!("{base} (untried)");
    }
    let strength = match lore.potency {
        Potency::Weak => "weak ",
        Potency::Common => "",
        Potency::Strong => "strong ",
    };
    let named = if count == 1 {
        format!("{strength}{base}")
    } else {
        format!("{count} {strength}{}", def.plural)
    };
    match lore.side {
        SideEffect::None => named,
        SideEffect::Bitter => format!("{named} (bitter)"),
    }
}

/// With "a"/"an" for single items: "a sickle", "3 stones".
pub fn item_phrase(world: &World, kind: ItemKindId, count: u32) -> String {
    let name = item_name(world, kind, count);
    if count == 1 {
        with_article(&name)
    } else {
        name
    }
}

/// Short stat lines for the pack screen and Look.
pub fn item_stats(world: &World, kind: ItemKindId) -> Vec<String> {
    let def = world.content().item(kind);
    let mut lines = Vec::new();
    match &def.class {
        ItemClass::Melee {
            family,
            damage: (lo, hi),
            accuracy,
        } => {
            lines.push(format!(
                "Damage {lo}–{hi} · accuracy {accuracy:+} · {family:?}"
            ));
        }
        ItemClass::Ranged {
            damage: (lo, hi),
            accuracy,
            range,
            ammo,
        } => {
            let ammo = world
                .content()
                .item_by_id(ammo)
                .map_or_else(|| ammo.clone(), |k| world.content().item(k).plural.clone());
            lines.push(format!(
                "Damage {lo}–{hi} · accuracy {accuracy:+} · range {range}"
            ));
            lines.push(format!("Shoots {ammo}."));
        }
        ItemClass::Vestment { defense } => lines.push(format!("Defense {defense:+}")),
        ItemClass::Tincture { effect } => {
            let lore = world.tincture_lore(kind);
            match lore.filter(|l| l.known) {
                Some(lore) => {
                    let amount = tallow_core::item::tincture_amount(*effect, lore.potency);
                    lines.push(match effect {
                        tallow_core::TinctureEffect::Mending => format!("Mends {amount} health."),
                        tallow_core::TinctureEffect::Steadying => {
                            format!("Lowers dread by {amount}.")
                        }
                        tallow_core::TinctureEffect::Seeing if amount >= 99 => {
                            "Shows the whole floor.".into()
                        }
                        tallow_core::TinctureEffect::Seeing => {
                            format!("Shows the floor within {amount} paces.")
                        }
                    });
                    if lore.side == SideEffect::Bitter {
                        lines.push("Bitter: leaves a little dread.".into());
                    }
                }
                None => {
                    lines.push("How strong, and at what cost, you'll learn by drinking one.".into())
                }
            }
        }
        ItemClass::Text { school } => lines.push(format!(
            "Read it (a few quiet turns) to learn a {} rite you don't know.",
            school_name(*school)
        )),
        ItemClass::Bell { noise } => lines.push(format!(
            "Ring it (a) or throw it: heard {noise} steps away. The Taken close by cower."
        )),
        ItemClass::Ammo | ItemClass::Throwable => {}
    }
    if let Some(thrown) = def.thrown {
        match thrown.holy {
            Some(holy) => lines.push(format!(
                "Thrown: burns the Dreaming {}–{}, the Taken {}–{}. Leaves the ground holy for a while.",
                holy.dreaming.0, holy.dreaming.1, holy.taken.0, holy.taken.1
            )),
            None if thrown.fire => lines.push(
                "Thrown: bursts into flame where it breaks, catching books, pews, doors and oil."
                    .into(),
            ),
            None if thrown.oil => lines
                .push("Thrown: spills oil around where it breaks. Slippery; burns fast.".into()),
            None if thrown.noise.is_some() => {
                lines.push("Thrown: rings where it lands, and can be picked up again.".into())
            }
            None => lines.push(format!(
                "Thrown: {}–{} · accuracy {:+}",
                thrown.damage.0, thrown.damage.1, thrown.accuracy
            )),
        }
    }
    lines.push(format!("Weight {}", tenths(def.weight)));
    lines
}

/// 143 → "14.3".
pub fn tenths(value: u32) -> String {
    format!("{}.{}", value / 10, value % 10)
}

use tallow_core::{
    Boon, Family, Passive, Reward, RiteEffect, RiteId, RiteTarget, School, Skill, Technique,
    Trigger,
};

pub fn skill_name(skill: Skill) -> &'static str {
    match skill {
        Skill::Blades => "Blades",
        Skill::Bludgeons => "Bludgeons",
        Skill::Reach => "Reach",
        Skill::Missiles => "Missiles",
        Skill::Endurance => "Endurance",
        Skill::Binding => "Binding",
        Skill::Communion => "Communion",
        Skill::Veil => "Veil",
        Skill::Warding => "Warding",
    }
}

pub fn school_name(school: School) -> &'static str {
    skill_name(school.skill())
}

/// A rite's numbers right now, at the current potency: "dread +15 · range 6 · 12 actions".
pub fn rite_numbers(world: &World, rite: RiteId) -> String {
    let def = world.content().rite(rite);
    let p = world.rite_potency(rite);
    let scale = |n: u32| (n * p / 100).max(1);
    let what = match def.effect {
        RiteEffect::Compel { actions } => format!("{} actions", scale(actions)),
        RiteEffect::Kneel { actions } => format!("{} actions", scale(actions)),
        RiteEffect::Leech { amount } => format!("drains {}", scale(amount)),
        RiteEffect::Transference { dread, actions } => {
            format!(
                "dread −{}, it flees {} actions",
                scale(dread),
                scale(actions)
            )
        }
        RiteEffect::BorrowedEyes { turns, .. } => format!("{} turns", scale(turns)),
        RiteEffect::Unsee { actions } => format!("{} actions", scale(actions)),
        RiteEffect::FalseFlame { turns } => format!("{} turns", scale(turns)),
        RiteEffect::Shroud { turns } => format!("{} turns", scale(turns)),
        RiteEffect::Sanctify { turns, radius } => {
            format!("radius {radius}, {} turns", scale(turns))
        }
        RiteEffect::Exorcise { damage } => {
            format!("frees one of the Taken (bosses: {})", scale(damage))
        }
        RiteEffect::Turncoat => "it turns on its own kind".into(),
        RiteEffect::Beckon { actions } => format!("up to {} steps", scale(actions)),
        RiteEffect::Exchange => "trade places".into(),
        RiteEffect::Hush { turns } => format!("{} turns", scale(turns)),
        RiteEffect::Seal { turns } => format!("{} turns", scale(turns)),
        RiteEffect::Banish => "the Dreaming only".into(),
    };
    let reach = match def.effect.target() {
        RiteTarget::Myself => "on yourself".to_string(),
        RiteTarget::Creature | RiteTarget::Tile | RiteTarget::Door => {
            format!("range {}", def.range)
        }
    };
    format!("dread +{} · {reach} · {what}", world.rite_cost(rite))
}

fn family_noun(family: Family) -> &'static str {
    match family {
        Family::Blade => "blades",
        Family::Bludgeon => "bludgeons",
        Family::Reach => "reach weapons",
    }
}

/// A boon's name and what it does.
pub fn boon_text(boon: Boon) -> (String, String) {
    match boon {
        Boon::Passive(p) => {
            let (title, text) = match p {
                Passive::Accuracy(n) => ("Steady Hand", format!("+{n} accuracy with everything.")),
                Passive::Defense(n) => ("Hard to Hit", format!("+{n} defense.")),
                Passive::MaxHealth(n) => ("Deep Breath", format!("+{n} health.")),
                Passive::FamilyDamage(f) => (
                    match f {
                        Family::Blade => "Keen Edge",
                        Family::Bludgeon => "Heavy Hand",
                        Family::Reach => "Long Leverage",
                    },
                    format!("+1 damage with {}.", family_noun(f)),
                ),
                Passive::MissileAccuracy(n) => (
                    "Sure Throw",
                    format!("+{n} accuracy with thrown and fired things."),
                ),
                Passive::CandleThrift => {
                    ("Slow Wick", "Your candle burns a quarter slower.".into())
                }
                Passive::DarkSight => (
                    "Night Eyes",
                    "In the dark you feel two tiles around you, not one.".into(),
                ),
                Passive::CalmInLight => (
                    "Candle Calm",
                    "Candlelight breeds less than half the dread.".into(),
                ),
                Passive::BrazierKin => (
                    "Hearth-Kin",
                    "Brazier light eases dread twice as fast.".into(),
                ),
                Passive::StrongMedicine => (
                    "Strong Medicine",
                    "Mending tinctures mend half again as much.".into(),
                ),
                Passive::TallowThief => (
                    "Tallow Thief",
                    "Tallow you find is worth a quarter more.".into(),
                ),
                Passive::PackMule => (
                    "Broad Back",
                    "Carry 5.0 more before you're burdened or stuck.".into(),
                ),
                Passive::QuickMending => (
                    "Quick Mending",
                    "Regain health every 8 turns instead of 12.".into(),
                ),
                Passive::RiteThrift => {
                    ("Familiar Words", "Rites cost a quarter less dread.".into())
                }
            };
            (title.into(), text)
        }
        Boon::Triggered { when, then } => {
            let (title, condition) = match when {
                Trigger::Kill => ("Grim Harvest", "When you kill something".to_string()),
                Trigger::KillWith(f) => (
                    match f {
                        Family::Blade => "Blade's Due",
                        Family::Bludgeon => "Bone Toll",
                        Family::Reach => "Long Due",
                    },
                    format!("When you kill with {}", family_noun(f)),
                ),
                Trigger::MissileHit => (
                    "Marksman's Ease",
                    "When something you throw or shoot hits".into(),
                ),
                Trigger::Snuff => ("Dark Comfort", "When you snuff your candle".into()),
                Trigger::Descend => ("Down and Deeper", "When you go down a stair".into()),
                Trigger::Drink => ("Second Draught", "When you drink a tincture".into()),
                Trigger::DodgeHeavyBlow => ("Light Feet", "When a heavy blow misses you".into()),
                Trigger::Cast => ("Answered Prayer", "When you cast a rite".into()),
                Trigger::Study => ("Scholar's Reward", "When you finish studying a body".into()),
            };
            let result = match then {
                Reward::Heal(n) => format!("mend {n} health"),
                Reward::EaseDread(n) => format!("dread eases by {n}"),
                Reward::Tallow(n) => format!("gain {n} tallow"),
                Reward::LoseTrail => "the nearest hunter loses your trail".into(),
            };
            (title.into(), format!("{condition}, {result}."))
        }
    }
}

/// A technique's name and what it does.
pub fn technique_text(skill: Skill, technique: Technique) -> (String, String) {
    let school = skill_name(skill);
    match technique {
        Technique::Deepen { percent } => (
            format!("Deep Rites (+{percent}%)"),
            format!("{school} rites last longer and do more."),
        ),
        Technique::Thrift { percent } => (
            format!("Quiet Rites (−{percent}%)"),
            format!("{school} rites cost less dread."),
        ),
        Technique::Riposte { chance } => (
            format!("Riposte ({chance}%)"),
            "When a creature misses you in melee, your blade strikes back at once.".into(),
        ),
        Technique::Stagger { chance } => (
            format!("Stagger ({chance}%)"),
            "A bludgeon hit can cost a creature its next action.".into(),
        ),
        Technique::LongReach { accuracy } => (
            "Long Reach".into(),
            if accuracy > 0 {
                format!(
                    "Strike a creature two tiles away in a straight line, at {accuracy:+} accuracy."
                )
            } else {
                "Strike a creature two tiles away in a straight line.".into()
            },
        ),
        Technique::Pin { chance, actions } => (
            format!("Pin ({chance}%)"),
            format!("A missile hit can hold a creature in place for {actions} of its actions."),
        ),
        Technique::Brace { percent } => (
            "Brace".into(),
            format!("Heavy blows that land on you do {percent}% less."),
        ),
    }
}
