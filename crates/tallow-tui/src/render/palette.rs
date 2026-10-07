//! Colors. Truecolor; terminals without it get the nearest match from crossterm.
//!
//! Warm amber is reserved for the acolyte and candlelight. Everything else is
//! stone, soot and old bone.

use ratatui::style::Color;

pub const VOID: Color = Color::Rgb(9, 8, 8);

pub const WALL_FG: Color = Color::Rgb(122, 106, 92);
pub const WALL_BG: Color = Color::Rgb(36, 31, 28);
pub const FLOOR_FG: Color = Color::Rgb(74, 64, 56);
pub const FLOOR_BG: Color = Color::Rgb(18, 15, 14);
pub const DOOR_FG: Color = Color::Rgb(176, 124, 72);

pub const PLAYER: Color = Color::Rgb(255, 204, 120);

pub const TEXT: Color = Color::Rgb(196, 186, 170);
pub const TEXT_DIM: Color = Color::Rgb(110, 102, 92);
pub const ACCENT: Color = Color::Rgb(214, 160, 86);
pub const BORDER: Color = Color::Rgb(70, 60, 52);
