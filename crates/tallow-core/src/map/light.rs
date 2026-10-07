//! Light: sources cast light along their field of view, falling off with distance.

use std::collections::HashSet;

use super::{Map, fov};
use crate::geom::Point;
use crate::grid::Grid;

/// An RGB color in 0–255 per channel.
pub type Rgb = [u8; 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightSource {
    pub at: Point,
    pub radius: i32,
    pub color: Rgb,
}

/// How much light reaches a tile, and its blended color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Light {
    /// 0 is pitch dark, 255 is as bright as it gets.
    pub level: u8,
    pub color: Rgb,
}

impl Light {
    pub const fn is_lit(self) -> bool {
        self.level > 0
    }
}

/// Lights the map from every source. Light adds up; color is the brightness-weighted blend.
pub fn compute(map: &Map, sources: &[LightSource]) -> Grid<Light> {
    // Accumulate premultiplied color plus total intensity per tile.
    let mut acc: Grid<[f32; 4]> = Grid::new(map.width(), map.height(), [0.0; 4]);
    for source in sources {
        let reach = source.radius as f32 + 1.0;
        let mut lit = HashSet::new();
        fov::compute(
            source.at,
            source.radius,
            |p| map.blocks_sight(p),
            |p| {
                if !lit.insert(p) {
                    return;
                }
                let distance = (p.distance_squared(source.at) as f32).sqrt();
                let strength = (1.0 - distance / reach).max(0.0);
                if let Some(cell) = acc.get_mut(p) {
                    for (channel, &c) in cell.iter_mut().zip(&source.color) {
                        *channel += f32::from(c) * strength;
                    }
                    cell[3] += strength;
                }
            },
        );
    }

    let mut light = Grid::new(map.width(), map.height(), Light::default());
    for p in acc.points() {
        let [r, g, b, total] = acc.at(p);
        if total > 0.0 {
            light.set(
                p,
                Light {
                    level: (total * 255.0).round().clamp(1.0, 255.0) as u8,
                    color: [r, g, b].map(|c| (c / total).round().clamp(0.0, 255.0) as u8),
                },
            );
        }
    }
    light
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::prefab;

    const WARM: Rgb = [255, 180, 100];

    #[test]
    fn light_falls_off_and_walls_stop_it() {
        let (map, start) = prefab::parse(
            "#########\n\
             #@......#\n\
             #####.###\n\
             #.......#\n\
             #########",
        )
        .unwrap();
        let light = compute(
            &map,
            &[LightSource {
                at: start,
                radius: 6,
                color: WARM,
            }],
        );
        let near = light.at(Point::new(2, 1)).level;
        let far = light.at(Point::new(6, 1)).level;
        assert!(near > far && far > 0);
        assert!(!light.at(Point::new(1, 3)).is_lit(), "behind a wall");
        assert_eq!(light.at(start).color, WARM);
    }

    #[test]
    fn overlapping_lights_add_up_and_blend() {
        let (map, start) = prefab::parse("#######\n#@...##\n#######").unwrap();
        let other = Point::new(4, 1);
        let blue = [100, 100, 255];
        let one = compute(
            &map,
            &[LightSource {
                at: start,
                radius: 4,
                color: WARM,
            }],
        );
        let both = compute(
            &map,
            &[
                LightSource {
                    at: start,
                    radius: 4,
                    color: WARM,
                },
                LightSource {
                    at: other,
                    radius: 4,
                    color: blue,
                },
            ],
        );
        let mid = Point::new(3, 1);
        assert!(both.at(mid).level > one.at(mid).level);
        assert!(both.at(mid).color[2] > WARM[2]);
    }
}
