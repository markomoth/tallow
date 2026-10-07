//! Game content loaded from the RON files in `assets/`, embedded at compile time.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::map::light::Rgb;

const MONSTERS: &str = include_str!("../../../assets/monsters.ron");

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
}

impl Content {
    /// The content bundled into the binary. Parsed once.
    pub fn bundled() -> &'static Content {
        static CONTENT: OnceLock<Content> = OnceLock::new();
        CONTENT.get_or_init(|| {
            Content::parse(MONSTERS).expect("bundled monsters.ron is valid; covered by tests")
        })
    }

    pub fn parse(monsters: &str) -> Result<Content, ron::error::SpannedError> {
        let monsters: Vec<MonsterDef> = ron::from_str(monsters)?;
        Ok(Content { monsters })
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
