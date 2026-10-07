//! Pure game logic for Tallow.
//!
//! This crate never prints, reads the clock, or touches the terminal.
//! Frontends send [`Command`]s to a [`World`] and receive [`Event`]s back.

pub mod actions;
pub mod events;
pub mod geom;
pub mod map;
pub mod world;

pub use actions::Command;
pub use events::Event;
pub use geom::{Direction, Point};
pub use map::{Map, Tile};
pub use world::World;
