//! Colors. Truecolor; terminals without it get the nearest match from crossterm.
//!
//! Map colors are base colors at full light. The map view shades them by the
//! light on each tile. Warm amber is reserved for the acolyte and for fire.

use ratatui::style::Color;
use tallow_core::Rgb;

pub const VOID: Color = Color::Rgb(9, 8, 8);

pub const WALL_FG: Rgb = [150, 132, 114];
pub const WALL_BG: Rgb = [54, 46, 40];
pub const FLOOR_FG: Rgb = [118, 104, 90];
pub const FLOOR_BG: Rgb = [28, 23, 20];
pub const DOOR_FG: Rgb = [196, 140, 82];
pub const STAIRS_DOWN_FG: Rgb = [236, 224, 198];
pub const STAIRS_UP_FG: Rgb = [140, 130, 118];
pub const BRAZIER_FG: Rgb = [255, 156, 64];

/// Remembered tiles, out of sight: a cold blue-grey.
pub const MEMORY_TINT: [f32; 3] = [0.30, 0.34, 0.46];
pub const MEMORY_BG: Rgb = [11, 12, 16];

pub const PLAYER: Color = Color::Rgb(255, 210, 128);

/// Background of a tile about to be struck. Pulses.
pub const TELEGRAPH: Rgb = [150, 34, 26];
pub const HEALTH: Color = Color::Rgb(178, 64, 52);
pub const HEALTH_EMPTY: Color = Color::Rgb(58, 30, 28);
pub const DANGER: Color = Color::Rgb(232, 96, 72);

pub const TEXT: Color = Color::Rgb(196, 186, 170);
pub const TEXT_DIM: Color = Color::Rgb(110, 102, 92);
pub const ACCENT: Color = Color::Rgb(214, 160, 86);
pub const BORDER: Color = Color::Rgb(70, 60, 52);

pub const fn rgb([r, g, b]: Rgb) -> Color {
    Color::Rgb(r, g, b)
}
