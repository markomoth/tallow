//! Pure game logic for Tallow.
//!
//! This crate never prints, reads the clock, or touches the terminal.
//! Frontends send [`Command`]s to a [`World`] and receive [`Event`]s back.

pub mod actions;
pub mod biome;
pub mod events;
pub mod floor;
pub mod geom;
pub mod grid;
pub mod map;
pub mod rng;
pub mod world;

pub use actions::Command;
pub use biome::{Biome, MAX_DEPTH};
pub use events::Event;
pub use floor::Floor;
pub use geom::{Direction, Point};
pub use map::light::{Light, Rgb};
pub use map::{Map, Tile};
pub use world::World;
