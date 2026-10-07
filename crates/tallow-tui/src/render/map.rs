//! The map viewport: a camera that follows the player, with lighting and memory.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;
use tallow_core::{Direction, Floor, Light, Point, Rgb, Tile, World};

use super::palette::{self, rgb};

/// Brightness of a tile you can only feel (adjacent, unlit).
const FELT: f32 = 0.22;
/// How strongly a light's color tints what it falls on.
const TINT: f32 = 0.65;

pub struct MapView<'a> {
    world: &'a World,
    /// Seconds since start; drives flicker. Visual only, never affects rules.
    time: f32,
    /// The Look or target cursor, if any.
    cursor: Option<Point>,
    /// Tiles a projectile would cross, while aiming.
    path: Vec<Point>,
}

impl<'a> MapView<'a> {
    pub fn new(world: &'a World, time: f32, cursor: Option<Point>, path: Vec<Point>) -> Self {
        Self {
            world,
            time,
            cursor,
            path,
        }
    }
}

impl Widget for MapView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let floor = self.world.floor();
        let map = floor.map();
        let player = self.world.player().pos;
        let content = self.world.content();
        let telegraphs: Vec<Point> = self.world.telegraphs().collect();
        let pulse = 0.75 + 0.25 * (self.time * 9.0).sin();
        let origin = Point::new(
            camera_origin(map.width(), area.width.into(), player.x),
            camera_origin(map.height(), area.height.into(), player.y),
        );
        let frame = (self.time * 12.0) as u32;
        let global_flicker =
            1.0 + 0.06 * (self.time * 6.1).sin() + 0.04 * (self.time * 14.3 + 1.3).sin();

        for sy in 0..area.height {
            for sx in 0..area.width {
                let p = Point::new(origin.x + i32::from(sx), origin.y + i32::from(sy));
                let Some(cell) = buf.cell_mut((area.x + sx, area.y + sy)) else {
                    continue;
                };
                let Some(look) = tile_look(floor, p) else {
                    cell.set_char(' ')
                        .set_fg(palette::VOID)
                        .set_bg(palette::VOID);
                    continue;
                };

                let (fg, bg) = if floor.is_visible(p) {
                    let flicker = global_flicker + jitter(p, frame);
                    let light = floor.light(p);
                    let fg = if look.emissive {
                        look.fg
                    } else {
                        shade(look.fg, light, flicker)
                    };
                    (fg, shade(look.bg, light, flicker))
                } else {
                    (remember(look.fg), remember_bg(look.bg))
                };

                let bg = if telegraphs.contains(&p) {
                    palette::TELEGRAPH.map(|c| (f32::from(c) * pulse) as u8)
                } else if self.path.contains(&p) {
                    palette::AIM
                } else if floor.is_sanctified(p) && floor.is_explored(p) {
                    if floor.is_visible(p) {
                        palette::HOLY_BG
                    } else {
                        remember_bg(palette::HOLY_BG)
                    }
                } else {
                    bg
                };
                let bg = if floor.has_oil(p) && floor.is_explored(p) && !floor.is_burning(p) {
                    if floor.is_visible(p) {
                        palette::OIL_BG
                    } else {
                        remember_bg(palette::OIL_BG)
                    }
                } else {
                    bg
                };
                let fire = floor.is_burning(p) && floor.is_visible(p);
                let bg = if fire {
                    palette::FIRE_BG.map(|c| (f32::from(c) * pulse) as u8)
                } else {
                    bg
                };
                let decoy = floor.decoy().filter(|d| d.at == p && floor.is_visible(p));
                let corpse = floor.corpse_at(p).filter(|_| floor.is_explored(p));
                let monster = floor
                    .monster_at(p)
                    .filter(|_| floor.is_visible(p))
                    .and_then(|id| floor.monster(id));
                let tallow = floor.tallow_at(p).filter(|_| floor.is_explored(p));
                let item = floor.items_at(p).last().filter(|_| floor.is_explored(p));
                if p == player {
                    cell.set_char('@').set_fg(palette::PLAYER).set_bg(rgb(bg));
                    cell.modifier.insert(Modifier::BOLD);
                } else if let Some(monster) = monster {
                    let def = content.monster(monster.kind);
                    // Phantoms shimmer, very slightly. Look tells for sure.
                    let color = if monster.phantom {
                        let shimmer = 0.8 + 0.2 * (self.time * 5.0 + p.x as f32).sin();
                        def.color.map(|c| (f32::from(c) * shimmer) as u8)
                    } else {
                        def.color
                    };
                    cell.set_char(def.glyph).set_fg(rgb(color)).set_bg(rgb(bg));
                    cell.modifier.insert(Modifier::BOLD);
                    if monster.compelled > 0 {
                        cell.modifier.insert(Modifier::UNDERLINED);
                    }
                } else if fire {
                    // Flames flicker through three colors and two shapes.
                    let i = (frame as usize + (p.x * 7 + p.y * 3) as usize) % 3;
                    let glyph = if i == 1 { '"' } else { '^' };
                    cell.set_char(glyph)
                        .set_fg(rgb(palette::FIRE_FG[i]))
                        .set_bg(rgb(bg));
                    cell.modifier.insert(Modifier::BOLD);
                } else if decoy.is_some() {
                    let flicker = 0.8 + 0.2 * (self.time * 11.0).sin();
                    let fg =
                        tallow_core::rites::DECOY_COLOR.map(|c| (f32::from(c) * flicker) as u8);
                    cell.set_char('*').set_fg(rgb(fg)).set_bg(rgb(bg));
                    cell.modifier.insert(Modifier::BOLD);
                } else if let Some(item) = item {
                    let def = content.item(item.item.kind);
                    let fg = if floor.is_visible(p) {
                        shade(def.color, floor.light(p), global_flicker)
                    } else {
                        remember(def.color)
                    };
                    cell.set_char(def.glyph).set_fg(rgb(fg)).set_bg(rgb(bg));
                } else if tallow.is_some() {
                    let fg = if floor.is_visible(p) {
                        shade(palette::TALLOW_FG, floor.light(p), global_flicker)
                    } else {
                        remember(palette::TALLOW_FG)
                    };
                    cell.set_char(',').set_fg(rgb(fg)).set_bg(rgb(bg));
                } else if let Some(corpse) = corpse {
                    let base = content.monster(corpse.kind).color.map(|c| c / 2 + 30);
                    let fg = if floor.is_visible(p) {
                        shade(base, floor.light(p), global_flicker)
                    } else {
                        remember(base)
                    };
                    cell.set_char('%').set_fg(rgb(fg)).set_bg(rgb(bg));
                } else {
                    cell.set_char(look.glyph).set_fg(rgb(fg)).set_bg(rgb(bg));
                }
                if self.cursor == Some(p) {
                    cell.modifier.insert(Modifier::REVERSED);
                }
            }
        }
    }
}

struct Look {
    glyph: char,
    fg: Rgb,
    bg: Rgb,
    /// Gives off its own light, so it never looks dim while in view.
    emissive: bool,
}

/// How a tile looks, or `None` if it should not be drawn: never seen, or solid
/// rock with no open space beside it.
fn tile_look(floor: &Floor, p: Point) -> Option<Look> {
    if !floor.is_explored(p) {
        return None;
    }
    let map = floor.map();
    let plain = |glyph, fg| Look {
        glyph,
        fg,
        bg: palette::FLOOR_BG,
        emissive: false,
    };
    let look = match map.tile(p) {
        Tile::Floor => plain('.', palette::FLOOR_FG),
        Tile::Door => plain('\'', palette::DOOR_FG),
        Tile::DoorClosed => plain('+', palette::DOOR_FG),
        Tile::DoorSealed => plain('+', palette::SEAL_FG),
        Tile::ColdBrazier => plain('&', palette::COLD_BRAZIER_FG),
        Tile::Pew => plain('=', palette::PEW_FG),
        Tile::BellRope => Look {
            glyph: '|',
            fg: palette::BELL_FG,
            bg: palette::WALL_BG,
            emissive: false,
        },
        Tile::Bookshelf => Look {
            glyph: '#',
            fg: palette::SHELF_FG,
            bg: palette::SHELF_BG,
            emissive: false,
        },
        Tile::StairsDown => plain('>', palette::STAIRS_DOWN_FG),
        Tile::StairsUp => plain('<', palette::STAIRS_UP_FG),
        Tile::Brazier => Look {
            emissive: true,
            ..plain('&', palette::BRAZIER_FG)
        },
        Tile::Wall => {
            let faces_open = Direction::ALL.iter().any(|&d| map.is_walkable(p + d));
            if !faces_open {
                return None;
            }
            Look {
                glyph: '#',
                fg: palette::WALL_FG,
                bg: palette::WALL_BG,
                emissive: false,
            }
        }
    };
    Some(look)
}

/// A base color under the given light: brighter near the source, tinted by its color.
fn shade(base: Rgb, light: Light, flicker: f32) -> Rgb {
    let level = f32::from(light.level) / 255.0;
    if level == 0.0 {
        return base.map(|c| (f32::from(c) * FELT) as u8);
    }
    let brightness = FELT + 1.25 * level * flicker;
    let mut out = [0u8; 3];
    for i in 0..3 {
        let tint = 1.0 - TINT + TINT * f32::from(light.color[i]) / 255.0;
        out[i] = (f32::from(base[i]) * brightness * tint).clamp(0.0, 255.0) as u8;
    }
    out
}

/// Remembered but out of sight: desaturated toward cold blue-grey.
fn remember(base: Rgb) -> Rgb {
    let [r, g, b] = base.map(f32::from);
    let luma = 0.30 * r + 0.59 * g + 0.11 * b;
    palette::MEMORY_TINT.map(|t| (luma * t).clamp(0.0, 255.0) as u8)
}

fn remember_bg(base: Rgb) -> Rgb {
    if base == palette::WALL_BG {
        remember(base).map(|c| c.max(palette::MEMORY_BG[0]))
    } else {
        palette::MEMORY_BG
    }
}

/// A small per-tile wobble so the flicker isn't perfectly uniform.
fn jitter(p: Point, frame: u32) -> f32 {
    let mut h = (p.x as u32).wrapping_mul(73_856_093)
        ^ (p.y as u32).wrapping_mul(19_349_663)
        ^ frame.wrapping_mul(83_492_791);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h % 1000) as f32 / 1000.0 * 0.08 - 0.04
}

/// Where the camera's top-left sits on one axis. Centers small maps; otherwise
/// keeps the focus centered but never scrolls past the map edge.
fn camera_origin(map_len: i32, view_len: i32, focus: i32) -> i32 {
    if map_len <= view_len {
        -(view_len - map_len) / 2
    } else {
        (focus - view_len / 2).clamp(0, map_len - view_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_maps_are_centered() {
        assert_eq!(camera_origin(10, 20, 3), -5);
        assert_eq!(camera_origin(20, 20, 3), 0);
    }

    #[test]
    fn large_maps_follow_the_focus_within_bounds() {
        assert_eq!(camera_origin(100, 20, 50), 40);
        assert_eq!(camera_origin(100, 20, 2), 0);
        assert_eq!(camera_origin(100, 20, 99), 80);
    }

    #[test]
    fn brighter_light_means_brighter_tiles() {
        let warm = [255, 200, 140];
        let dim = shade(
            palette::FLOOR_FG,
            Light {
                level: 40,
                color: warm,
            },
            1.0,
        );
        let bright = shade(
            palette::FLOOR_FG,
            Light {
                level: 220,
                color: warm,
            },
            1.0,
        );
        let dark = shade(palette::FLOOR_FG, Light::default(), 1.0);
        assert!(bright[0] > dim[0] && dim[0] > dark[0]);
    }

    #[test]
    fn memory_is_cold() {
        let [r, _, b] = remember(palette::DOOR_FG);
        assert!(b > r, "remembered tiles lean blue");
    }

    #[test]
    fn jitter_stays_small() {
        for x in 0..50 {
            assert!(jitter(Point::new(x, x * 3), x as u32).abs() <= 0.04);
        }
    }
}
