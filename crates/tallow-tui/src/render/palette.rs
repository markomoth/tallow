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
pub const COLD_BRAZIER_FG: Rgb = [120, 110, 100];
pub const SHELF_FG: Rgb = [150, 96, 60];
pub const SHELF_BG: Rgb = [48, 30, 22];
pub const PEW_FG: Rgb = [140, 98, 64];
pub const BELL_FG: Rgb = [226, 196, 110];
pub const SEAL_FG: Rgb = [236, 214, 130];
pub const FIRE_FG: [Rgb; 3] = [[255, 200, 80], [255, 130, 40], [230, 70, 30]];
pub const FIRE_BG: Rgb = [110, 34, 10];
pub const OIL_BG: Rgb = [52, 46, 18];
pub const SHALLOW_FG: Rgb = [110, 150, 180];
pub const SHALLOW_BG: Rgb = [22, 34, 44];
pub const DEEP_FG: Rgb = [70, 110, 170];
pub const DEEP_BG: Rgb = [10, 18, 40];
pub const ROTTEN_FG: Rgb = [130, 100, 60];

/// Each biome tints its stone a little: cold crypt grey, cool Collegium
/// stone, green-blue damp, yellowed rot, the Throne's red.
pub fn biome_tint(biome: tallow_core::Biome) -> [f32; 3] {
    use tallow_core::Biome;
    match biome {
        Biome::Crypts => [1.0, 1.0, 1.0],
        Biome::Collegium => [0.92, 0.96, 1.12],
        Biome::DrownedStacks => [0.78, 0.98, 1.08],
        Biome::RotCourt => [1.1, 1.0, 0.7],
        Biome::Throne => [1.2, 0.82, 0.78],
    }
}

pub fn tinted(base: Rgb, tint: [f32; 3]) -> Rgb {
    [0, 1, 2].map(|i| (f32::from(base[i]) * tint[i]).clamp(0.0, 255.0) as u8)
}

/// Remembered tiles, out of sight: a cold blue-grey.
pub const MEMORY_TINT: [f32; 3] = [0.30, 0.34, 0.46];
pub const MEMORY_BG: Rgb = [11, 12, 16];

pub const PLAYER: Color = Color::Rgb(255, 210, 128);

/// Background of a tile about to be struck. Pulses.
pub const TELEGRAPH: Rgb = [150, 34, 26];
pub const HEALTH: Color = Color::Rgb(178, 64, 52);
pub const HEALTH_EMPTY: Color = Color::Rgb(58, 30, 28);
pub const DANGER: Color = Color::Rgb(232, 96, 72);
pub const DREAD: Color = Color::Rgb(176, 140, 214);
pub const DREAD_EMPTY: Color = Color::Rgb(44, 36, 56);
pub const GOOD: Color = Color::Rgb(168, 196, 138);
pub const CANDLE: Color = Color::Rgb(232, 176, 92);
pub const CANDLE_EMPTY: Color = Color::Rgb(56, 44, 30);
pub const TALLOW_FG: Rgb = [240, 220, 168];
pub const LOAD: Color = Color::Rgb(150, 140, 120);
pub const LOAD_EMPTY: Color = Color::Rgb(46, 42, 36);
/// Background of holy ground.
pub const HOLY_BG: Rgb = [58, 52, 26];
/// Background of tiles a projectile would cross.
pub const AIM: Rgb = [70, 58, 34];

pub const TEXT: Color = Color::Rgb(196, 186, 170);
pub const TEXT_DIM: Color = Color::Rgb(110, 102, 92);
pub const ACCENT: Color = Color::Rgb(214, 160, 86);
pub const BORDER: Color = Color::Rgb(70, 60, 52);

pub const fn rgb([r, g, b]: Rgb) -> Color {
    Color::Rgb(r, g, b)
}
