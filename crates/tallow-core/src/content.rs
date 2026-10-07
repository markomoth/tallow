//! Game content loaded from the RON files in `assets/`, embedded at compile time.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::item::{ItemClass, ItemDef, ItemKindId};
use crate::map::light::Rgb;
use crate::rites::{RiteDef, RiteId, RiteTarget};
use crate::skills::{Skill, SkillDef};

const MONSTERS: &str = include_str!("../../../assets/monsters.ron");
const ITEMS: &str = include_str!("../../../assets/items.ron");
const SKILLS: &str = include_str!("../../../assets/skills.ron");
const RITES: &str = include_str!("../../../assets/rites.ron");
const LEAVINGS: &str = include_str!("../../../assets/leavings.ron");

/// Index of a monster definition in [`Content::monsters`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KindId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Faction {
    Dreaming,
    Taken,
    Swarm,
    Remnant,
}

impl Faction {
    pub const ALL: [Faction; 4] = [
        Faction::Dreaming,
        Faction::Taken,
        Faction::Swarm,
        Faction::Remnant,
    ];

    /// Who attacks whom on sight (BUILD_GUIDE.md §2). The Dreaming hate the
    /// living, the Swarm anything with a pulse, the Remnant every intruder.
    /// The Taken only want you.
    pub const fn hates(self, other: Faction) -> bool {
        use Faction::{Dreaming, Remnant, Swarm, Taken};
        matches!(
            (self, other),
            (Dreaming, Swarm | Taken) | (Swarm, Taken) | (Remnant, Dreaming | Swarm | Taken)
        )
    }

    /// Hands enough to open a door.
    pub const fn opens_doors(self) -> bool {
        matches!(self, Faction::Taken | Faction::Remnant)
    }
}

/// A creature's one clear trick (BUILD_GUIDE.md §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Trait {
    /// Flees when no packmate is close.
    PackCourage,
    /// Speaks when it notices you.
    Pleads,
    /// Will not step onto ground lit by braziers.
    ShunsLight,
    /// Its bite snuffs your candle and eats some tallow.
    EatsLight,
    /// Always knows where you are.
    Relentless,
    /// Winds up for one action, then strikes a marked tile hard.
    HeavyBlow { damage: (u32, u32), cooldown: u32 },
    /// Its hit gets in your eyes: your light shrinks for a while.
    Blinds { turns: u32 },
    /// Chants a rite at you for one action, then it lands if it can still see you.
    Chants { damage: (u32, u32), cooldown: u32 },
    /// Its hit drags you a step toward deep water.
    Drags,
    /// Its singing raises your dread while it can see you (hundredths per action).
    Sings { dread: u32 },
    /// Bursts into its `spawns` creature when it dies.
    Bursts,
    /// In darkness, calls up its `spawns` creature, up to `max` at a time.
    Summons { cooldown: u32, max: u32 },
    /// Raises a nearby body as its `spawns` creature.
    Raises { range: i32, cooldown: u32 },
    /// Locks the doors around it and sets books alight.
    Rewrites { cooldown: u32 },
    /// Several bodies, one life: a wound to one is a wound to all.
    Chorus,
    /// At home in deep water.
    Swims,
    /// Its heavy blows strike a cross: the marked tile and the four beside it.
    Sweeps,
    /// Beelzebub's borrowed body: Binding rites and Exorcise pin him in it.
    Vessel,
    /// Can't set foot on holy ground.
    Unholy,
    /// Eats its way through shut doors (3 actions; sealed ones 8).
    Gnaws,
    /// Can't be killed. Only slowed.
    Undying,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonsterDef {
    pub id: String,
    pub name: String,
    pub plural: String,
    pub glyph: char,
    pub color: Rgb,
    pub faction: Faction,
    pub description: String,
    pub health: u32,
    pub speed: u32,
    pub accuracy: i32,
    pub defense: i32,
    pub damage: (u32, u32),
    pub sight: i32,
    pub depth: (u8, u8),
    pub group: (u32, u32),
    pub threat: u32,
    pub weight: u32,
    /// Spawns with floors. Manifestations and other summoned things don't.
    #[serde(default = "natural_default")]
    pub natural: bool,
    /// Leaves a body when it dies. The Dreaming come apart instead.
    #[serde(default = "natural_default")]
    pub corpse: bool,
    /// A mini-boss or worse: resists Binding, can't be freed by Exorcise.
    #[serde(default)]
    pub boss: bool,
    /// Rites that studying its body teaches, in order (rite ids).
    #[serde(default)]
    pub teaches: Vec<String>,
    /// What it bursts into, summons or raises (a monster id).
    #[serde(default)]
    pub spawns: Option<String>,
    /// Resolved `spawns`, filled in when content loads.
    #[serde(skip)]
    pub spawn: Option<KindId>,
    /// The floor a boss waits on, by the stair down.
    #[serde(default)]
    pub boss_floor: Option<u8>,
    pub traits: Vec<Trait>,
    pub barks: Vec<String>,
}

fn natural_default() -> bool {
    true
}

impl MonsterDef {
    pub fn has(&self, wanted: impl Fn(&Trait) -> bool) -> bool {
        self.traits.iter().any(wanted)
    }

    pub fn heavy_blow(&self) -> Option<((u32, u32), u32)> {
        self.traits.iter().find_map(|t| match *t {
            Trait::HeavyBlow { damage, cooldown } => Some((damage, cooldown)),
            _ => None,
        })
    }
}

#[derive(Debug)]
pub struct Content {
    pub monsters: Vec<MonsterDef>,
    pub items: Vec<ItemDef>,
    pub skills: Vec<SkillDef>,
    pub rites: Vec<RiteDef>,
    pub leavings: crate::leavings::LeavingParts,
}

#[derive(Debug)]
pub enum ContentError {
    Parse(ron::error::SpannedError),
    /// A reference to an id that doesn't exist, like a sling's ammunition.
    Missing(String),
}

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentError::Parse(e) => write!(f, "{e}"),
            ContentError::Missing(id) => write!(f, "unknown id {id:?}"),
        }
    }
}

impl std::error::Error for ContentError {}

impl From<ron::error::SpannedError> for ContentError {
    fn from(e: ron::error::SpannedError) -> Self {
        ContentError::Parse(e)
    }
}

impl Content {
    /// The content bundled into the binary. Parsed once.
    pub fn bundled() -> &'static Content {
        static CONTENT: OnceLock<Content> = OnceLock::new();
        CONTENT.get_or_init(|| {
            Content::parse(MONSTERS, ITEMS, SKILLS, RITES, LEAVINGS)
                .expect("bundled content is valid; covered by tests")
        })
    }

    pub fn parse(
        monsters: &str,
        items: &str,
        skills: &str,
        rites: &str,
        leavings: &str,
    ) -> Result<Content, ContentError> {
        let mut monsters: Vec<MonsterDef> = ron::from_str(monsters)?;
        let ids: Vec<String> = monsters.iter().map(|d| d.id.clone()).collect();
        for def in &mut monsters {
            if let Some(id) = &def.spawns {
                let index = ids
                    .iter()
                    .position(|i| i == id)
                    .ok_or_else(|| ContentError::Missing(id.clone()))?;
                def.spawn = Some(KindId(index as u16));
            }
        }
        let items: Vec<ItemDef> = ron::from_str(items)?;
        let skills: Vec<SkillDef> = ron::from_str(skills)?;
        let rites: Vec<RiteDef> = ron::from_str(rites)?;
        let leavings: crate::leavings::LeavingParts = ron::from_str(leavings)?;
        let content = Content {
            monsters,
            items,
            skills,
            rites,
            leavings,
        };
        for def in &content.leavings.named {
            if let Some(from) = &def.from {
                content
                    .kind_by_id(from)
                    .ok_or_else(|| ContentError::Missing(from.clone()))?;
            }
        }
        for def in &content.monsters {
            for id in &def.teaches {
                content
                    .rite_by_id(id)
                    .ok_or_else(|| ContentError::Missing(id.clone()))?;
            }
        }
        for def in &content.rites {
            let aimed = def.effect.target() != RiteTarget::Myself;
            if aimed != (def.range > 0) {
                return Err(ContentError::Missing(format!("{}: range", def.id)));
            }
        }
        for skill in Skill::ALL {
            if !content.skills.iter().any(|d| d.skill == skill) {
                return Err(ContentError::Missing(format!("{skill:?}")));
            }
        }
        for def in &content.items {
            if let ItemClass::Ranged { ammo, .. } = &def.class {
                content
                    .item_by_id(ammo)
                    .ok_or_else(|| ContentError::Missing(ammo.clone()))?;
            }
        }
        Ok(content)
    }

    pub fn skill(&self, skill: Skill) -> &SkillDef {
        self.skills
            .iter()
            .find(|d| d.skill == skill)
            .expect("every skill is defined; checked when content loads")
    }

    pub fn item(&self, kind: ItemKindId) -> &ItemDef {
        &self.items[usize::from(kind.0)]
    }

    pub fn item_kinds(&self) -> impl Iterator<Item = (ItemKindId, &ItemDef)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, def)| (ItemKindId(i as u16), def))
    }

    pub fn item_by_id(&self, id: &str) -> Option<ItemKindId> {
        self.item_kinds()
            .find(|(_, def)| def.id == id)
            .map(|(kind, _)| kind)
    }

    pub fn rite(&self, rite: RiteId) -> &RiteDef {
        &self.rites[usize::from(rite.0)]
    }

    pub fn rite_ids(&self) -> impl Iterator<Item = RiteId> + use<> {
        (0..self.rites.len() as u16).map(RiteId)
    }

    pub fn rite_by_id(&self, id: &str) -> Option<RiteId> {
        self.rites
            .iter()
            .position(|def| def.id == id)
            .map(|i| RiteId(i as u16))
    }

    pub fn monster(&self, kind: KindId) -> &MonsterDef {
        &self.monsters[usize::from(kind.0)]
    }

    pub fn kinds(&self) -> impl Iterator<Item = (KindId, &MonsterDef)> {
        self.monsters
            .iter()
            .enumerate()
            .map(|(i, def)| (KindId(i as u16), def))
    }

    pub fn kind_by_id(&self, id: &str) -> Option<KindId> {
        self.kinds()
            .find(|(_, def)| def.id == id)
            .map(|(kind, _)| kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn bundled_content_parses() {
        let content = Content::bundled();
        assert!(content.monsters.len() >= 4);
    }

    #[test]
    fn item_definitions_are_sane() {
        let content = Content::bundled();
        let mut ids = HashSet::new();
        for def in &content.items {
            assert!(ids.insert(&def.id), "duplicate id {}", def.id);
            assert!(def.glyph.is_ascii_graphic(), "{}", def.id);
            assert!(def.depth.0 <= def.depth.1, "{}", def.id);
            assert!(def.stack.0 >= 1 && def.stack.0 <= def.stack.1, "{}", def.id);
            assert!(
                def.stacks() || def.stack == (1, 1),
                "{} can't stack",
                def.id
            );
            if matches!(def.class, ItemClass::Throwable) {
                assert!(
                    def.thrown.is_some(),
                    "{} is a throwable that can't be thrown",
                    def.id
                );
            }
        }
        for id in ["iron_candlestick", "cassock", "mending_tincture"] {
            assert!(content.item_by_id(id).is_some(), "starting kit needs {id}");
        }
    }

    #[test]
    fn missing_ammo_is_caught() {
        let items = r#"[(id: "sling", name: "s", plural: "s", glyph: ')', color: (1, 1, 1),
            description: "", class: Ranged(damage: (1, 2), accuracy: 0, range: 5, ammo: "nope"),
            weight: 1, depth: (1, 1), frequency: 1)]"#;
        assert!(matches!(
            Content::parse("[]", items, SKILLS, RITES, LEAVINGS),
            Err(ContentError::Missing(_))
        ));
    }

    #[test]
    fn monster_definitions_are_sane() {
        let content = Content::bundled();
        let mut ids = HashSet::new();
        for def in &content.monsters {
            assert!(ids.insert(&def.id), "duplicate id {}", def.id);
            assert!(def.health > 0 && def.speed > 0, "{}", def.id);
            assert!(def.damage.0 <= def.damage.1, "{}", def.id);
            assert!(def.group.0 >= 1 && def.group.0 <= def.group.1, "{}", def.id);
            assert!(def.depth.0 <= def.depth.1, "{}", def.id);
            assert!(def.threat > 0 && def.weight > 0, "{}", def.id);
            assert!(def.glyph.is_ascii_graphic(), "{}", def.id);
            if def.has(|t| *t == Trait::Pleads) {
                assert!(!def.barks.is_empty(), "{} pleads but has no barks", def.id);
            }
        }
    }
}
