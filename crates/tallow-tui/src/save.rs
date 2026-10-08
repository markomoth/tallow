//! Saving a run (BUILD_GUIDE.md §11). The game is deterministic, so a save is
//! just the seed and every command given; loading replays them. A save is
//! deleted as soon as it is loaded, so there is no going back.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tallow_core::Command;

/// Bumped whenever a change would make old saves replay differently.
pub const VERSION: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub seed: u64,
    pub commands: Vec<Command>,
}

impl Save {
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_ron())
    }

    /// Reads and deletes a save. `None` if there is none, or it is from
    /// another version of the game.
    pub fn take(path: &Path) -> Option<Save> {
        let text = std::fs::read_to_string(path).ok()?;
        std::fs::remove_file(path).ok();
        Save::from_ron(&text)
    }

    pub fn to_ron(&self) -> String {
        ron::to_string(self).expect("a save is plain data")
    }

    /// `None` if the text isn't a save, or is from another version of the game.
    pub fn from_ron(text: &str) -> Option<Save> {
        let save: Save = ron::from_str(text).ok()?;
        (save.version == VERSION).then_some(save)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tallow_core::Direction;

    #[test]
    fn a_save_is_read_once() {
        let dir = std::env::temp_dir().join(format!("tallow-save-{}", std::process::id()));
        let path = dir.join("save.ron");
        let save = Save {
            version: VERSION,
            seed: 42,
            commands: vec![Command::Move(Direction::E), Command::Wait],
        };
        save.write(&path).unwrap();
        assert_eq!(Save::take(&path), Some(save));
        assert_eq!(Save::take(&path), None, "deleted when loaded");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The browser keeps saves as text; another version's save is not loaded.
    #[test]
    fn a_save_as_text() {
        let save = Save {
            version: VERSION,
            seed: 7,
            commands: vec![Command::Wait],
        };
        assert_eq!(Save::from_ron(&save.to_ron()), Some(save.clone()));
        let old = Save {
            version: VERSION - 1,
            ..save
        };
        assert_eq!(Save::from_ron(&old.to_ron()), None);
        assert_eq!(Save::from_ron("not a save"), None);
    }
}
