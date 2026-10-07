//! One floor of the labyrinth: its map plus what the player sees and remembers.

use slotmap::SlotMap;

use crate::events::Event;
use crate::geom::Point;
use crate::grid::Grid;
use crate::map::light::{self, Light, LightSource, Rgb};
use crate::map::{Map, Tile, fov};
use crate::monster::{Monster, MonsterId};

pub const BRAZIER_RADIUS: i32 = 5;
pub const BRAZIER_COLOR: Rgb = [255, 138, 64];

/// How far the eye reaches along an unbroken line. You still only see what is lit.
const SIGHT_RADIUS: i32 = 40;

#[derive(Debug, Clone)]
pub struct Floor {
    map: Map,
    arrival: Point,
    lights: Vec<LightSource>,
    /// Light from fixed sources only (braziers), without the player's candle.
    ambient: Grid<Light>,
    light: Grid<Light>,
    /// In unbroken line of sight from the player, lit or not. Sight is
    /// symmetric, so this is also everywhere that can see the player.
    in_sight: Grid<bool>,
    visible: Grid<bool>,
    explored: Grid<bool>,
    pub(crate) monsters: SlotMap<MonsterId, Monster>,
    pub(crate) tallow: Vec<Tallow>,
    pub(crate) items: Vec<FloorItem>,
}

/// An item lying on the floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorItem {
    pub at: Point,
    pub item: crate::item::Item,
    seen: bool,
}

impl FloorItem {
    pub fn new(at: Point, item: crate::item::Item) -> Self {
        Self {
            at,
            item,
            seen: false,
        }
    }

    /// Something the player put there, so no need to announce it.
    pub fn seen(at: Point, item: crate::item::Item) -> Self {
        Self {
            at,
            item,
            seen: true,
        }
    }
}

/// A lump or stub of tallow lying on the floor. Walk over it to take it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tallow {
    pub at: Point,
    /// Turns of burning it adds.
    pub amount: u32,
    seen: bool,
}

impl Tallow {
    pub fn new(at: Point, amount: u32) -> Self {
        Self {
            at,
            amount,
            seen: false,
        }
    }
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
            .collect::<Vec<_>>();
        let (w, h) = (map.width(), map.height());
        Self {
            ambient: light::compute(&map, &lights),
            map,
            arrival,
            lights,
            light: Grid::new(w, h, Light::default()),
            in_sight: Grid::new(w, h, false),
            visible: Grid::new(w, h, false),
            explored: Grid::new(w, h, false),
            monsters: SlotMap::with_key(),
            tallow: Vec::new(),
            items: Vec::new(),
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

    /// Light from braziers alone, ignoring the player's candle.
    pub fn ambient_light(&self, p: Point) -> Light {
        self.ambient.at(p)
    }

    /// In line of sight of the player (lit or not).
    pub fn in_sight(&self, p: Point) -> bool {
        self.in_sight.at(p)
    }

    pub fn monsters(&self) -> impl Iterator<Item = (MonsterId, &Monster)> {
        self.monsters.iter()
    }

    pub fn monster(&self, id: MonsterId) -> Option<&Monster> {
        self.monsters.get(id)
    }

    pub fn monster_at(&self, p: Point) -> Option<MonsterId> {
        self.monsters
            .iter()
            .find(|(_, m)| m.pos == p)
            .map(|(id, _)| id)
    }

    /// Monsters the player can see right now, nearest first.
    pub fn visible_monsters(&self, from: Point) -> Vec<MonsterId> {
        let mut seen: Vec<(i32, MonsterId)> = self
            .monsters
            .iter()
            .filter(|(_, m)| self.is_visible(m.pos))
            .map(|(id, m)| (m.pos.distance_squared(from), id))
            .collect();
        seen.sort_by_key(|&(d, _)| d);
        seen.into_iter().map(|(_, id)| id).collect()
    }

    /// In view right now.
    pub fn is_visible(&self, p: Point) -> bool {
        self.visible.at(p)
    }

    /// Seen at some point on this floor.
    pub fn is_explored(&self, p: Point) -> bool {
        self.explored.at(p)
    }

    /// Tallow lying on this floor.
    pub fn tallow(&self) -> &[Tallow] {
        &self.tallow
    }

    pub fn tallow_at(&self, p: Point) -> Option<&Tallow> {
        self.tallow.iter().find(|t| t.at == p)
    }

    /// Items lying on this floor, in the order they were put down.
    pub fn items(&self) -> &[FloorItem] {
        &self.items
    }

    /// Items on one tile; the last is on top.
    pub fn items_at(&self, p: Point) -> impl Iterator<Item = &FloorItem> {
        self.items.iter().filter(move |f| f.at == p)
    }

    /// Learns the shape of the floor within `radius` of `center`, as if seen.
    pub(crate) fn reveal(&mut self, center: Point, radius: i32) {
        for p in self.map.points() {
            let near = p.distance_squared(center) <= radius * radius;
            let shaped = self.map.is_walkable(p)
                || crate::geom::Direction::ALL
                    .iter()
                    .any(|&d| self.map.is_walkable(p + d));
            if near && shaped {
                self.explored.set(p, true);
            }
        }
    }

    /// Landmarks and tallow in view right now: things worth stopping a run for.
    pub fn visible_landmarks(&self) -> impl Iterator<Item = Point> + '_ {
        let tiles = self
            .map
            .points()
            .filter(|&p| self.is_visible(p) && self.map.tile(p).is_landmark());
        let tallow = self
            .tallow
            .iter()
            .map(|t| t.at)
            .filter(|&p| self.is_visible(p));
        let items = self
            .items
            .iter()
            .map(|f| f.at)
            .filter(|&p| self.is_visible(p));
        tiles.chain(tallow).chain(items)
    }

    /// Recomputes light and sight from `eye`, carrying an optional light of its own.
    /// You see what is in line of sight and lit, plus whatever is right beside you.
    /// Returns a `Spotted` event for each landmark seen for the first time.
    pub(crate) fn update_view(
        &mut self,
        eye: Point,
        carried: Option<LightSource>,
        feel: i32,
    ) -> Vec<Event> {
        let sources: Vec<LightSource> = self.lights.iter().copied().chain(carried).collect();
        self.light = light::compute(&self.map, &sources);

        self.visible.fill(false);
        self.in_sight.fill(false);
        let (map, light) = (&self.map, &self.light);
        let (visible, in_sight) = (&mut self.visible, &mut self.in_sight);
        fov::compute(
            eye,
            SIGHT_RADIUS,
            |p| map.blocks_sight(p),
            |p| {
                in_sight.set(p, true);
                if light.at(p).is_lit() || p.chebyshev(eye) <= feel {
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
        for tallow in &mut self.tallow {
            if !tallow.seen && self.visible.at(tallow.at) {
                tallow.seen = true;
                events.push(Event::SpottedTallow { at: tallow.at });
            }
        }
        for item in &mut self.items {
            if !item.seen && self.visible.at(item.at) {
                item.seen = true;
                events.push(Event::SpottedItem {
                    kind: item.item.kind,
                    at: item.at,
                });
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
        floor.update_view(start, None, 1);
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
        let events = floor.update_view(start, None, 1);
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
        assert_eq!(floor.update_view(start, None, 1).len(), 1);
        assert!(floor.update_view(start, None, 1).is_empty());
        assert!(floor.is_explored(Point::new(5, 1)));
    }
}
