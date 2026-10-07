//! Events: everything that happened as a result of a command.
//!
//! The frontend turns events into log messages and animations.
//! Logic code never builds display strings.

use crate::content::KindId;
use crate::geom::Point;
use crate::map::Tile;

/// Someone in a fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    Player,
    Monster(KindId),
}

/// What killed the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Attack(KindId),
    HeavyBlow(KindId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The player moved to a new tile.
    PlayerMoved {
        to: Point,
    },
    /// The player tried to move but something solid was in the way. Costs no time.
    PlayerBlocked {
        at: Point,
        tile: Tile,
    },
    /// The player waited a turn.
    PlayerWaited,
    /// A landmark came into view for the first time on this floor.
    Spotted {
        tile: Tile,
        at: Point,
    },
    /// The player went down to a new floor.
    Descended {
        depth: u8,
    },
    /// The player tried to use stairs that aren't there. Costs no time.
    NoStairsHere,
    /// The way back up is closed during the descent. Costs no time.
    StairsSealed {
        depth: u8,
    },
    /// A run was refused because something hostile is in view. Costs no time.
    RunRefused,

    /// The first time this run the player sees a kind of creature.
    FirstSighting {
        kind: KindId,
    },
    /// A monster noticed the player. `seen` is whether the player can see it.
    Noticed {
        kind: KindId,
        seen: bool,
    },
    /// A monster spoke. `line` indexes its barks.
    Bark {
        kind: KindId,
        line: usize,
    },
    /// A blow was struck. `damage` is `None` on a miss.
    Attack {
        attacker: Who,
        defender: Who,
        damage: Option<u32>,
    },
    /// A monster raised a heavy blow aimed at `target`. It lands next action.
    WindUp {
        kind: KindId,
        target: Point,
    },
    /// A heavy blow came down on `target`. `damage` is `None` if nobody was there.
    HeavyBlow {
        kind: KindId,
        target: Point,
        damage: Option<u32>,
    },
    /// A monster lost its nerve.
    Fled {
        kind: KindId,
    },
    MonsterDied {
        kind: KindId,
        at: Point,
    },
    PlayerDied {
        cause: Cause,
    },
}
