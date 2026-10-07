//! One floor of the labyrinth: its map plus what the player sees and remembers.

use crate::events::Event;
use crate::geom::Point;
use crate::grid::Grid;
use crate::map::light::{self, Light, LightSource, Rgb};
use crate::map::{Map, Tile, fov};

pub const BRAZIER_RADIUS: i32 = 5;
pub const BRAZIER_COLOR: Rgb = [255, 138, 64];

/// How far the eye reaches along an unbroken line. You still only see what is lit.
const SIGHT_RADIUS: i32 = 40;

#[derive(Debug, Clone)]
pub struct Floor {
    map: Map,
    arrival: Point,
    lights: Vec<LightSource>,
    light: Grid<Light>,
    visible: Grid<bool>,
    explored: Grid<bool>,
}

impl Floor {
    /// A floor whose player arrives at `arrival`. Braziers on the map become light sources.
    pub fn new(map: Map, arrival: Point) -> Self {
        let lights = map
            .find(Tile::Brazier)
            .map(|at| LightSource {
                at,
                radius: BRAZIER_RADIUS,
                color: BRAZIER_COLOR,
            })
            .collect();
        let (w, h) = (map.width(), map.height());
        Self {
            map,
            arrival,
            lights,
            light: Grid::new(w, h, Light::default()),
            visible: Grid::new(w, h, false),
            explored: Grid::new(w, h, false),
        }
    }

    pub fn map(&self) -> &Map {
        &self.map
    }

    pub fn arrival(&self) -> Point {
        self.arrival
    }

    /// Light reaching a tile this turn.
    pub fn light(&self, p: Point) -> Light {
        self.light.at(p)
    }

    /// In view right now.
    pub fn is_visible(&self, p: Point) -> bool {
        self.visible.at(p)
    }

    /// Seen at some point on this floor.
    pub fn is_explored(&self, p: Point) -> bool {
        self.explored.at(p)
    }

    /// Landmarks in view right now.
    pub fn visible_landmarks(&self) -> impl Iterator<Item = Point> + '_ {
        self.map
            .points()
            .filter(|&p| self.is_visible(p) && self.map.tile(p).is_landmark())
    }

    /// Recomputes light and sight from `eye`, carrying an optional light of its own.
    /// You see what is in line of sight and lit, plus whatever is right beside you.
    /// Returns a `Spotted` event for each landmark seen for the first time.
    pub(crate) fn update_view(&mut self, eye: Point, carried: Option<LightSource>) -> Vec<Event> {
        let sources: Vec<LightSource> = self.lights.iter().copied().chain(carried).collect();
        self.light = light::compute(&self.map, &sources);

        self.visible.fill(false);
        let (map, light, visible) = (&self.map, &self.light, &mut self.visible);
        fov::compute(
            eye,
            SIGHT_RADIUS,
            |p| map.blocks_sight(p),
            |p| {
                if light.at(p).is_lit() || p.chebyshev(eye) <= 1 {
                    visible.set(p, true);
                }
            },
        );

        let mut events = Vec::new();
        for p in self.map.points() {
            if self.visible.at(p) && !self.explored.at(p) {
                self.explored.set(p, true);
                let tile = self.map.tile(p);
                if tile.is_landmark() {
                    events.push(Event::Spotted { tile, at: p });
                }
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::prefab;

    #[test]
    fn darkness_hides_what_is_in_line_of_sight() {
        let (map, start) = prefab::parse("##########\n#@.......#\n##########").unwrap();
        let mut floor = Floor::new(map, start);
        floor.update_view(start, None);
        assert!(
            floor.is_visible(Point::new(2, 1)),
            "adjacent tiles are felt"
        );
        assert!(!floor.is_visible(Point::new(5, 1)), "unlit tiles stay dark");
    }

    #[test]
    fn braziers_light_distant_rooms() {
        let (map, start) = prefab::parse("#############\n#@.........&#\n#############").unwrap();
        let mut floor = Floor::new(map, start);
        let events = floor.update_view(start, None);
        assert!(floor.is_visible(Point::new(10, 1)));
        assert_eq!(
            events,
            vec![Event::Spotted {
                tile: Tile::Brazier,
                at: Point::new(11, 1)
            }]
        );
    }

    #[test]
    fn memory_outlasts_sight_and_landmarks_are_spotted_once() {
        let (map, start) = prefab::parse("#######\n#@...&#\n#######").unwrap();
        let mut floor = Floor::new(map, start);
        assert_eq!(floor.update_view(start, None).len(), 1);
        assert!(floor.update_view(start, None).is_empty());
        assert!(floor.is_explored(Point::new(5, 1)));
    }
}
