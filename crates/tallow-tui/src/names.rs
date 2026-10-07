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
        ItemClass::Ammo | ItemClass::Throwable => {}
    }
    if let Some(thrown) = def.thrown {
        match thrown.holy {
            Some(holy) => lines.push(format!(
                "Thrown: burns the Dreaming {}–{}, the Taken {}–{}.",
                holy.dreaming.0, holy.dreaming.1, holy.taken.0, holy.taken.1
            )),
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
