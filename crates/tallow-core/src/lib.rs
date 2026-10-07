//! Pure game logic for Tallow.
//!
//! This crate never prints, reads the clock, or touches the terminal.
//! Frontends send [`Command`]s to a [`World`] and receive [`Event`]s back.

pub mod actions;
mod ai;
pub mod biome;
pub mod combat;
pub mod content;
pub mod events;
pub mod floor;
pub mod geom;
pub mod grid;
pub mod map;
pub mod monster;
pub mod rng;
pub mod spawn;
pub mod time;
pub mod world;

pub use actions::Command;
pub use biome::{Biome, MAX_DEPTH};
pub use content::{Content, Faction, KindId, MonsterDef, Trait};
pub use events::{Cause, Event, Who};
pub use floor::Floor;
pub use geom::{Direction, Point};
pub use map::light::{Light, Rgb};
pub use map::{Map, Tile};
pub use monster::{Mind, Monster, MonsterId};
pub use world::{Death, MonsterInfo, Player, World};
