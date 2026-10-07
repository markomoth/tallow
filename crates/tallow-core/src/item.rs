//! Items: what they are (from `assets/items.ron`) and what the player carries.

use serde::Deserialize;

/// Index of an item definition in [`crate::Content::items`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemKindId(pub u16);

/// One particular item or stack, unique for the run.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, Deserialize,
)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKindId,
    pub count: u32,
}

/// Weapon families. Each will have its own skill (M5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Family {
    Blade,
    Bludgeon,
    Reach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum TinctureEffect {
    Mending,
    Steadying,
    Seeing,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum ItemClass {
    Melee {
        family: Family,
        damage: (u32, u32),
        accuracy: i32,
    },
    Ranged {
        damage: (u32, u32),
        accuracy: i32,
        range: i32,
        ammo: String,
    },
    Ammo,
    Throwable,
    Vestment {
        defense: i32,
    },
    Tincture {
        effect: TinctureEffect,
    },
    /// Teaches a rite of its school when read.
    Text {
        school: crate::rites::School,
    },
    /// Rung in the hand (`a`): heard this many steps away.
    Bell {
        noise: u32,
    },
    /// The Vigil Candle.
    Relic,
}

/// Damage holy water does on a splash, by what it lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Holy {
    pub dreaming: (u32, u32),
    pub taken: (u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct ThrowStats {
    pub accuracy: i32,
    pub damage: (u32, u32),
    /// Shatters where it lands instead of lying there to be picked up.
    #[serde(default)]
    pub breaks: bool,
    #[serde(default)]
    pub holy: Option<Holy>,
    /// Bursts into flame where it breaks.
    #[serde(default)]
    pub fire: bool,
    /// Spills lamp oil where it breaks.
    #[serde(default)]
    pub oil: bool,
    /// Rings where it lands, heard this many steps away.
    #[serde(default)]
    pub noise: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    pub plural: String,
    pub glyph: char,
    pub color: crate::map::light::Rgb,
    pub description: String,
    pub class: ItemClass,
    /// In tenths.
    pub weight: u32,
    pub depth: (u8, u8),
    pub frequency: u32,
    #[serde(default = "single")]
    pub stack: (u32, u32),
    #[serde(default)]
    pub thrown: Option<ThrowStats>,
}

fn single() -> (u32, u32) {
    (1, 1)
}

impl ItemDef {
    /// Consumables and ammunition share one pack slot per kind.
    pub fn stacks(&self) -> bool {
        matches!(
            self.class,
            ItemClass::Ammo
                | ItemClass::Throwable
                | ItemClass::Tincture { .. }
                | ItemClass::Text { .. }
        )
    }

    pub fn slot(&self) -> Option<Slot> {
        match self.class {
            ItemClass::Melee { .. } => Some(Slot::Melee),
            ItemClass::Ranged { .. } => Some(Slot::Ranged),
            ItemClass::Vestment { .. } => Some(Slot::Body),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Melee,
    Ranged,
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Equipment {
    pub melee: Option<ItemId>,
    pub ranged: Option<ItemId>,
    pub body: Option<ItemId>,
}

impl Equipment {
    pub fn get(&self, slot: Slot) -> Option<ItemId> {
        match slot {
            Slot::Melee => self.melee,
            Slot::Ranged => self.ranged,
            Slot::Body => self.body,
        }
    }

    pub(crate) fn set(&mut self, slot: Slot, item: Option<ItemId>) {
        match slot {
            Slot::Melee => self.melee = item,
            Slot::Ranged => self.ranged = item,
            Slot::Body => self.body = item,
        }
    }

    pub fn contains(&self, item: ItemId) -> bool {
        [self.melee, self.ranged, self.body].contains(&Some(item))
    }
}

/// How strong a tincture kind is this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Potency {
    Weak,
    Common,
    Strong,
}

/// What a tincture kind costs you this run. Never lethal, never permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideEffect {
    None,
    /// Leaves a taste of the dark: a little dread.
    Bitter,
}

pub const BITTER_DREAD: i32 = 600;

/// This run's truth about one tincture kind, and whether you've learned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TinctureLore {
    pub potency: Potency,
    pub side: SideEffect,
    pub known: bool,
}

/// How much a tincture does: health mended, dread steadied, or tiles seen.
pub const fn tincture_amount(effect: TinctureEffect, potency: Potency) -> u32 {
    match (effect, potency) {
        (TinctureEffect::Mending, Potency::Weak) => 5,
        (TinctureEffect::Mending, Potency::Common) => 9,
        (TinctureEffect::Mending, Potency::Strong) => 15,
        (TinctureEffect::Steadying, Potency::Weak) => 12,
        (TinctureEffect::Steadying, Potency::Common) => 20,
        (TinctureEffect::Steadying, Potency::Strong) => 32,
        (TinctureEffect::Seeing, Potency::Weak) => 12,
        (TinctureEffect::Seeing, Potency::Common) => 24,
        (TinctureEffect::Seeing, Potency::Strong) => 99,
    }
}

/// Carrying limits, in tenths.
pub const LIGHT_LOAD: u32 = 250;
pub const MAX_LOAD: u32 = 380;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Burden {
    Light,
    /// Moving costs half again as long.
    Burdened,
    /// Too heavy to move at all.
    Overloaded,
}

impl Burden {
    /// Burden at the standard limits.
    pub const fn of(load: u32) -> Burden {
        Burden::within(load, LIGHT_LOAD, MAX_LOAD)
    }

    pub const fn within(load: u32, light: u32, max: u32) -> Burden {
        if load <= light {
            Burden::Light
        } else if load <= max {
            Burden::Burdened
        } else {
            Burden::Overloaded
        }
    }
}
