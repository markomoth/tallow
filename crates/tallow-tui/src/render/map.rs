//! The map viewport: a camera that follows the player over the tile grid.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;
use tallow_core::{Direction, Map, Point, Tile, World};

use super::palette;

pub struct MapView<'a> {
    world: &'a World,
}

impl<'a> MapView<'a> {
    pub fn new(world: &'a World) -> Self {
        Self { world }
    }
}

impl Widget for MapView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let map = self.world.map();
        let player = self.world.player();
        let origin = Point::new(
            camera_origin(map.width(), area.width.into(), player.x),
            camera_origin(map.height(), area.height.into(), player.y),
        );

        for sy in 0..area.height {
            for sx in 0..area.width {
                let p = Point::new(origin.x + i32::from(sx), origin.y + i32::from(sy));
                let (glyph, fg, bg) = if p == player {
                    ('@', palette::PLAYER, palette::FLOOR_BG)
                } else {
                    tile_look(map, p)
                };
                if let Some(cell) = buf.cell_mut((area.x + sx, area.y + sy)) {
                    cell.set_char(glyph).set_fg(fg).set_bg(bg);
                    if p == player {
                        cell.modifier.insert(Modifier::BOLD);
                    }
                }
            }
        }
    }
}

/// Glyph and colors for a map tile. Solid rock deep inside walls draws as empty
/// void, so only the faces of rooms and corridors show.
fn tile_look(map: &Map, p: Point) -> (char, Color, Color) {
    match map.tile(p) {
        Tile::Floor => ('.', palette::FLOOR_FG, palette::FLOOR_BG),
        Tile::Door => ('+', palette::DOOR_FG, palette::FLOOR_BG),
        Tile::Wall if touches_open_space(map, p) => ('#', palette::WALL_FG, palette::WALL_BG),
        Tile::Wall => (' ', palette::VOID, palette::VOID),
    }
}

fn touches_open_space(map: &Map, p: Point) -> bool {
    Direction::ALL.iter().any(|&dir| map.is_walkable(p + dir))
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
}
