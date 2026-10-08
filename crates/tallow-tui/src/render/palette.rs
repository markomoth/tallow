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
pub const SEEP_BG: Rgb = [30, 22, 34];
pub const ANOMALY_FG: Rgb = [220, 120, 255];
/// Letters on a wall that glow only in the dark.
pub const WRITING_FG: Rgb = [150, 220, 230];
pub fn tier_color(tier: tallow_core::leavings::Tier) -> Rgb {
    use tallow_core::leavings::Tier;
    match tier {
        Tier::Mild => [150, 210, 150],
        Tier::Strange => [240, 190, 90],
        Tier::Deadly => [240, 70, 60],
    }
}

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

/// The ascent: everything a little wrong, bruised violet.
pub const UNRAVELLING_TINT: [f32; 3] = [1.05, 0.78, 1.15];
pub const ALTAR_FG: Rgb = [255, 232, 170];

pub fn tinted(base: Rgb, tint: [f32; 3]) -> Rgb {
    [0, 1, 2].map(|i| (f32::from(base[i]) * tint[i]).clamp(0.0, 255.0) as u8)
}

/// Remembered tiles, out of sight: a cold blue-grey. Faces (backgrounds)
/// stay dark; glyphs keep enough light to read the shape of a room by.
pub const MEMORY_TINT: [f32; 3] = [0.30, 0.34, 0.46];
pub const MEMORY_FG_TINT: [f32; 3] = [0.66, 0.74, 0.98];
pub const MEMORY_BG: Rgb = [11, 12, 16];

pub const PLAYER: Color = Color::Rgb(255, 210, 128);

/// Background of a tile about to be struck. Pulses.
pub const TELEGRAPH: Rgb = [150, 34, 26];
pub const HEALTH: Color = Color::Rgb(178, 64, 52);
pub const HEALTH_EMPTY: Color = Color::Rgb(58, 30, 28);
pub const DANGER: Color = Color::Rgb(232, 96, 72);
pub const DREAD: Color = Color::Rgb(176, 140, 214);
pub const DREAD_EMPTY: Color = Color::Rgb(70, 58, 88);
/// The dread bar's cells deepen through its bands: calm, uneasy, frayed.
pub const DREAD_BANDS: [Color; 3] = [
    Color::Rgb(112, 92, 140),
    Color::Rgb(150, 118, 190),
    Color::Rgb(196, 150, 240),
];
/// Where a band starts, on the empty part of the dread bar.
pub const DREAD_MARK: Color = Color::Rgb(116, 94, 146);
/// Rules and edges once you are frayed.
pub const DREAD_RULE: Color = Color::Rgb(110, 84, 140);
/// What the map's edges bruise toward when you are frayed.
pub const DREAD_EDGE: Rgb = [70, 34, 96];
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
pub const TEXT_DIM: Color = Color::Rgb(146, 136, 120);
pub const ACCENT: Color = Color::Rgb(214, 160, 86);
pub const BORDER: Color = Color::Rgb(70, 60, 52);
/// Popups and the Look card: a frame a little warmer than the map's rules,
/// on a ground a little lighter than the void.
pub const FRAME: Color = Color::Rgb(110, 92, 70);
pub const PANEL: Color = Color::Rgb(16, 14, 13);
/// The selected row in a list.
pub const SELECTED: Color = Color::Rgb(40, 32, 24);

pub const fn rgb([r, g, b]: Rgb) -> Color {
    Color::Rgb(r, g, b)
}
