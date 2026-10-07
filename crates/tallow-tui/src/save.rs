//! Saving a run (BUILD_GUIDE.md §11). The game is deterministic, so a save is
//! just the seed and every command given; loading replays them. A save is
//! deleted as soon as it is loaded, so there is no going back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tallow_core::Command;

/// Bumped whenever a change would make old saves replay differently.
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub seed: u64,
    pub commands: Vec<Command>,
}

/// Where saves and the journal live: `$TALLOW_HOME` if set, else the
/// platform's data directory for Tallow.
pub fn home() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("TALLOW_HOME") {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("", "", "tallow").map(|d| d.data_dir().to_path_buf())
}

impl Save {
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = ron::to_string(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }

    /// Reads and deletes a save. `None` if there is none, or it is from
    /// another version of the game.
    pub fn take(path: &Path) -> Option<Save> {
        let text = std::fs::read_to_string(path).ok()?;
        std::fs::remove_file(path).ok();
        let save: Save = ron::from_str(&text).ok()?;
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
}
