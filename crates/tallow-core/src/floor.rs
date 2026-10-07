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
pub const FIRE_RADIUS: i32 = 2;
pub const FIRE_COLOR: Rgb = [255, 112, 40];

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
    pub(crate) corpses: Vec<crate::corpse::Corpse>,
    /// Holy ground: the turn each sanctified tile stops being holy (0 = never was).
    sanctified: Grid<u64>,
    /// A False Flame burning somewhere on this floor.
    pub(crate) decoy: Option<crate::rites::Decoy>,
    /// Turns of fire left on each tile (0 = not burning).
    fire: Grid<u8>,
    /// Spilled lamp oil: slippery, and it burns.
    oil: Grid<bool>,
    /// Doors barred by Seal, and the turn each seal lapses.
    pub(crate) seals: Vec<(Point, u64)>,
    /// Doors locked against you by a boss, and when each gives.
    pub(crate) locks: Vec<(Point, u64)>,
    /// The seep room, if this floor has one (interior corners).
    pub(crate) seep: Option<(Point, Point)>,
    seep_seen: bool,
    pub(crate) anomalies: Vec<crate::leavings::Anomaly>,
    pub(crate) leavings: Vec<(Point, crate::leavings::LeavingId)>,
    leavings_seen: Vec<crate::leavings::LeavingId>,
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
        let lights = Self::braziers(&map);
        let (w, h) = (map.width(), map.height());
        Self {
            ambient: light::compute(&map, &lights),
            fire: Grid::new(w, h, 0),
            oil: Grid::new(w, h, false),
            seals: Vec::new(),
            locks: Vec::new(),
            seep: None,
            seep_seen: false,
            anomalies: Vec::new(),
            leavings: Vec::new(),
            leavings_seen: Vec::new(),
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
            corpses: Vec::new(),
            sanctified: Grid::new(w, h, 0),
            decoy: None,
        }
    }

    fn braziers(map: &Map) -> Vec<LightSource> {
        map.find(Tile::Brazier)
            .map(|at| LightSource {
                at,
                radius: BRAZIER_RADIUS,
                color: BRAZIER_COLOR,
            })
            .collect()
    }

    /// Fixed lights plus every fire.
    fn standing_lights(&self) -> Vec<LightSource> {
        let fires = self.burning().map(|at| LightSource {
            at,
            radius: FIRE_RADIUS,
            color: FIRE_COLOR,
        });
        self.lights.iter().copied().chain(fires).collect()
    }

    /// Recomputes light from braziers and fires after either changes.
    pub(crate) fn refresh_lights(&mut self) {
        self.lights = Self::braziers(&self.map);
        self.ambient = light::compute(&self.map, &self.standing_lights());
    }

    /// Changes a tile (a door opens, shelves burn away) and updates the light.
    pub(crate) fn set_tile(&mut self, p: Point, tile: Tile) {
        self.map.set(p, tile);
        self.refresh_lights();
    }

    pub fn is_burning(&self, p: Point) -> bool {
        self.fire.at(p) > 0
    }

    pub fn fire_at(&self, p: Point) -> u8 {
        self.fire.at(p)
    }

    /// Every burning tile.
    pub fn burning(&self) -> impl Iterator<Item = Point> + '_ {
        self.map.points().filter(|&p| self.fire.at(p) > 0)
    }

    pub(crate) fn set_fire(&mut self, p: Point, turns: u8) {
        self.fire.set(p, turns);
    }

    pub fn has_oil(&self, p: Point) -> bool {
        self.oil.at(p)
    }

    pub(crate) fn set_oil(&mut self, p: Point, oil: bool) {
        self.oil.set(p, oil);
    }

    /// Inside the seep room.
    pub fn is_seep(&self, p: Point) -> bool {
        self.seep
            .is_some_and(|(a, b)| p.x >= a.x && p.x <= b.x && p.y >= a.y && p.y <= b.y)
    }

    pub fn anomalies(&self) -> &[crate::leavings::Anomaly] {
        &self.anomalies
    }

    /// Leavings lying on this floor.
    pub fn leavings(&self) -> &[(Point, crate::leavings::LeavingId)] {
        &self.leavings
    }

    /// Bodies lying on this floor.
    pub fn corpses(&self) -> &[crate::corpse::Corpse] {
        &self.corpses
    }

    pub fn corpse_at(&self, p: Point) -> Option<&crate::corpse::Corpse> {
        self.corpses.iter().find(|c| c.at == p)
    }

    /// Holy ground the Dreaming can't cross.
    pub fn is_sanctified(&self, p: Point) -> bool {
        self.sanctified.at(p) > 0
    }

    pub(crate) fn sanctify(&mut self, p: Point, until: u64) {
        let now = self.sanctified.at(p);
        self.sanctified.set(p, now.max(until));
    }

    /// Lets lapsed holy ground go. True if any tile stopped being holy.
    pub(crate) fn expire_sanctity(&mut self, now: u64) -> bool {
        let mut any = false;
        for p in self.map.points() {
            let until = self.sanctified.at(p);
            if until > 0 && now >= until {
                self.sanctified.set(p, 0);
                any = true;
            }
        }
        any
    }

    /// The False Flame on this floor, if one burns.
    pub fn decoy(&self) -> Option<crate::rites::Decoy> {
        self.decoy
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
            .chain(self.leavings.iter().map(|&(at, _)| at))
            .filter(|&p| self.is_visible(p));
        tiles.chain(tallow).chain(items)
    }

    /// Recomputes light and sight from `eye`, carrying an optional light of its own.
    /// You see what is in line of sight and lit, plus whatever is right beside you,
    /// plus (with Borrowed Eyes) whatever another creature sees, lit or not.
    /// Returns a `Spotted` event for each landmark seen for the first time.
    pub(crate) fn update_view(
        &mut self,
        eye: Point,
        carried: Option<LightSource>,
        feel: i32,
        borrowed: Option<(Point, i32)>,
    ) -> Vec<Event> {
        let decoy = self.decoy.map(|d| LightSource {
            at: d.at,
            radius: crate::rites::DECOY_RADIUS,
            color: crate::rites::DECOY_COLOR,
        });
        let sources: Vec<LightSource> = self
            .standing_lights()
            .into_iter()
            .chain(carried)
            .chain(decoy)
            .collect();
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

        if let Some((other, radius)) = borrowed {
            let (map, visible) = (&self.map, &mut self.visible);
            fov::compute(
                other,
                radius,
                |p| map.blocks_sight(p),
                |p| visible.set(p, true),
            );
        }

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
        if !self.seep_seen
            && self
                .map
                .points()
                .any(|p| self.is_seep(p) && self.visible.at(p))
        {
            self.seep_seen = true;
            events.push(Event::SeepSpotted);
        }
        for &(at, id) in &self.leavings {
            if self.visible.at(at) && !self.leavings_seen.contains(&id) {
                self.leavings_seen.push(id);
                events.push(Event::SpottedLeaving { id });
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
        floor.update_view(start, None, 1, None);
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
        let events = floor.update_view(start, None, 1, None);
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
        assert_eq!(floor.update_view(start, None, 1, None).len(), 1);
        assert!(floor.update_view(start, None, 1, None).is_empty());
        assert!(floor.is_explored(Point::new(5, 1)));
    }
}
