//! Pure game logic for Tallow.
//!
//! This crate never prints, reads the clock, or touches the terminal.
//! Frontends send [`Command`]s to a [`World`] and receive [`Event`]s back.

pub mod actions;
mod ai;
pub mod biome;
pub mod candle;
pub mod combat;
pub mod content;
pub mod dread;
pub mod events;
pub mod floor;
pub mod geom;
pub mod grid;
pub mod inventory;
pub mod item;
pub mod map;
pub mod monster;
mod nightmare;
pub mod rng;
pub mod spawn;
pub mod time;
pub mod world;

pub use actions::Command;
pub use biome::{Biome, MAX_DEPTH};
pub use candle::{Candle, CandleState};
pub use content::{Content, Faction, KindId, MonsterDef, Trait};
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
pub use world::{Death, MonsterInfo, Player, World};
