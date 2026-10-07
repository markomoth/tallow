//! Pure game logic for Tallow.
//!
//! This crate never prints, reads the clock, or touches the terminal.
//! Frontends send [`Command`]s to a [`World`] and receive [`Event`]s back.

pub mod abilities;
pub mod actions;
mod ai;
pub mod biome;
pub mod boons;
pub mod candle;
pub mod combat;
pub mod content;
pub mod corpse;
pub mod dread;
pub mod environment;
pub mod events;
pub mod floor;
pub mod geom;
pub mod grid;
pub mod inventory;
pub mod item;
pub mod map;
pub mod monster;
mod nightmare;
pub mod progress;
pub mod rites;
pub mod rng;
pub mod skills;
pub mod spawn;
pub mod time;
pub mod world;

pub use actions::Command;
pub use biome::{Biome, MAX_DEPTH};
pub use boons::{Boon, Passive, Reward, Trigger};
pub use candle::{Candle, CandleState};
pub use content::{Content, Faction, KindId, MonsterDef, Trait};
pub use corpse::{Corpse, Decay};
pub use dread::{Dread, DreadBand};
pub use events::{Cause, Event, Who};
pub use floor::{Floor, FloorItem, Tallow};
pub use geom::{Direction, Point};
pub use item::{
    Burden, Equipment, Family, Item, ItemClass, ItemDef, ItemId, ItemKindId, Potency, SideEffect,
    Slot, TinctureEffect, TinctureLore,
};
pub use map::light::{Light, Rgb};
pub use map::{Map, Tile};
pub use monster::{Mind, Monster, MonsterId};
pub use rites::{RiteDef, RiteEffect, RiteFailure, RiteId, RiteTarget, School};
pub use skills::{Skill, Technique};
pub use world::{Death, MonsterInfo, Player, RunStats, World};
