//! The Journal (BUILD_GUIDE.md §11): lore that survives between runs.
//! Creatures met and studied, rites learned, Leavings held, and pages of the
//! town's history. Nothing in it adds power.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tallow_core::World;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub runs: u32,
    pub deaths: u32,
    pub wins: u32,
    pub deepest: u8,
    /// Creature ids met, and whether ever studied.
    pub creatures: BTreeMap<String, bool>,
    pub rites: BTreeSet<String>,
    /// Names of Leavings held.
    pub leavings: BTreeSet<String>,
    /// Bosses put down (ids).
    pub bosses: BTreeSet<String>,
    /// History pages found (ids from [`PAGES`]).
    pub pages: BTreeSet<String>,
}

/// A page of the town's history: id, title, text.
pub struct Page {
    pub id: &'static str,
    pub title: &'static str,
    pub text: &'static str,
}

pub const PAGES: [Page; 9] = [
    Page {
        id: "bell",
        title: "The Low Bell",
        text: "The church was built over a well that the town will not name. Its bell was cast low and wide, to be heard underground. For six hundred years a candle burned on the altar, and for six hundred years the town slept without dreaming.",
    },
    Page {
        id: "collegium",
        title: "The Collegium",
        text: "Under the crypts, the bishops founded a school to study what the bell kept out. Its scholars learned the old words: how to hold a body, how to give a fear away, how to make ground remember it was holy. Then the lectures stopped, and nobody came up.",
    },
    Page {
        id: "stacks",
        title: "The Drowned Stacks",
        text: "When the lower libraries flooded, the deacons went down to save the books. They are still down there, the water to their chests, holding the books above their heads.",
    },
    Page {
        id: "court",
        title: "The Rot Court",
        text: "Below the water the stone is warm, and it hums. Someone has been holding court there for a long time, among the flies, wearing whatever dead thing was nearest.",
    },
    Page {
        id: "throne",
        title: "The Throne",
        text: "The candle was not stolen. It was carried down, by someone who wanted to be the one who brought it back. The flies were waiting for him.",
    },
    Page {
        id: "sexton",
        title: "The Sexton's Ledger",
        text: "Forty years of burials, in a neat hand. In the last pages the entries run backwards: dug up, dug up, dug up. Beside each name, a small mark like a fly.",
    },
    Page {
        id: "provost",
        title: "The Provost's Order",
        text: "\"No student will leave the building until the matter is understood.\" Signed, and sealed, and dated the year the Collegium went quiet.",
    },
    Page {
        id: "drowned_choir",
        title: "The Choir",
        text: "Three choristers sang the evening office the night the water came, and did not stop. Some nights, in the town above, you can still hear them through the floor.",
    },
    Page {
        id: "candle",
        title: "Vigil",
        text: "The candle is back on the altar. It is a small flame. It has to be enough, and it is.",
    },
];

impl Journal {
    /// The journal on disk, or a fresh one if there is none (or it can't be read).
    pub fn load(path: &Path) -> Journal {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }

    /// Writes down what this run has shown you so far.
    pub fn record(&mut self, world: &World) {
        let content = world.content();
        for kind in world.sighted_kinds() {
            let def = content.monster(kind);
            if def.id != "manifestation" {
                self.creatures.entry(def.id.clone()).or_insert(false);
            }
        }
        for kind in world.studied_kinds() {
            self.creatures
                .insert(content.monster(kind).id.clone(), true);
        }
        for &rite in world.known_rites() {
            self.rites.insert(content.rite(rite).name.clone());
        }
        for &id in world.leavings_taken() {
            self.leavings.insert(world.leaving(id).name.clone());
        }
        for kind in world.defeated() {
            self.bosses.insert(content.monster(kind).id.clone());
        }
        let deepest = world.depth();
        self.deepest = self.deepest.max(deepest);
        self.pages.insert("bell".into());
        for (depth, page) in [
            (4, "collegium"),
            (7, "stacks"),
            (10, "court"),
            (12, "throne"),
        ] {
            if self.deepest >= depth {
                self.pages.insert(page.into());
            }
        }
        for boss in ["sexton", "provost", "drowned_choir"] {
            if self.bosses.contains(boss) {
                self.pages.insert(boss.into());
            }
        }
        if world.victory().is_some() {
            self.pages.insert("candle".into());
        }
    }

    /// A run is over: count it, and keep what it showed you.
    pub fn end_run(&mut self, world: &World) {
        self.record(world);
        self.runs += 1;
        if world.victory().is_some() {
            self.wins += 1;
        } else if world.death().is_some() {
            self.deaths += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_is_recorded_and_survives_a_round_trip() {
        let mut world = World::new(3);
        world.dev_skip_to(4);
        let compel = world.content().rite_by_id("compel").unwrap();
        world.teach_rite(compel);
        let mut journal = Journal::default();
        journal.record(&world);
        assert!(journal.rites.contains("Compel"));
        assert!(journal.pages.contains("collegium"));
        assert_eq!(journal.deepest, 4);

        let dir = std::env::temp_dir().join(format!("tallow-journal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal.ron");
        journal.save(&path).unwrap();
        assert_eq!(Journal::load(&path), journal);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_or_broken_journal_starts_fresh() {
        assert_eq!(
            Journal::load(Path::new("/nonexistent/tallow/journal.ron")),
            Journal::default()
        );
    }
}
