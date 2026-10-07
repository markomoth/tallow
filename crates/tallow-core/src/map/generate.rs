//! Procedural floors.
//!
//! M1 has one generator, the crypt, used at every depth. Each biome gets its
//! own flavor in M8. Every floor is connected: rooms are joined by a spanning
//! tree of corridors plus a few loops, and obstacles are only placed where
//! they cannot cut a path.

use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

use super::{Map, Tile, path};
use crate::biome::Biome;
use crate::geom::{Direction, Point};

pub const WIDTH: i32 = 80;
pub const HEIGHT: i32 = 34;

/// Minimum tiles between two room interiors: wall, corridor, wall.
const ROOM_GAP: i32 = 3;

/// A freshly generated floor. `start` holds the up-stair you arrived by.
#[derive(Debug, Clone)]
pub struct Layout {
    pub map: Map,
    pub start: Point,
    /// A seep room's interior corners, if this floor has one.
    pub seep: Option<(Point, Point)>,
}

/// Generates a crypt floor. The down-stair is left out on the deepest floor.
pub fn crypt<R: Rng + ?Sized>(rng: &mut R, with_stairs_down: bool) -> Layout {
    floor(rng, 1, with_stairs_down)
}

/// Generates the floor at `depth`, furnished for its biome: chapels with pews
/// in the crypts, libraries below.
pub fn floor<R: Rng + ?Sized>(rng: &mut R, depth: u8, with_stairs_down: bool) -> Layout {
    // Room placement is random and can come up short; just roll again.
    loop {
        if let Some(layout) = try_crypt(rng, depth, with_stairs_down) {
            return layout;
        }
    }
}

/// Doors that start shut, in percent.
const CLOSED_DOORS: f64 = 0.4;
/// Braziers that start cold, in percent.
const COLD_BRAZIERS: f64 = 0.4;
/// Chance a floor has a bell rope.
const BELL_ROPE: f64 = 0.7;

#[derive(Debug, Clone, Copy)]
struct Room {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Room {
    fn center(&self) -> Point {
        Point::new(self.x + self.w / 2, self.y + self.h / 2)
    }

    fn interior(&self) -> impl Iterator<Item = Point> + use<> {
        let Room { x, y, w, h } = *self;
        (y..y + h).flat_map(move |py| (x..x + w).map(move |px| Point::new(px, py)))
    }

    fn too_close(&self, other: &Room) -> bool {
        !(self.x + self.w + ROOM_GAP <= other.x
            || other.x + other.w + ROOM_GAP <= self.x
            || self.y + self.h + ROOM_GAP <= other.y
            || other.y + other.h + ROOM_GAP <= self.y)
    }

    /// The ring of wall around the interior, without its corners, paired with
    /// the two directions that run along the wall at that point.
    fn wall_ring(&self) -> Vec<(Point, [Direction; 2])> {
        let Room { x, y, w, h } = *self;
        let mut ring = Vec::new();
        for px in x..x + w {
            ring.push((Point::new(px, y - 1), [Direction::W, Direction::E]));
            ring.push((Point::new(px, y + h), [Direction::W, Direction::E]));
        }
        for py in y..y + h {
            ring.push((Point::new(x - 1, py), [Direction::N, Direction::S]));
            ring.push((Point::new(x + w, py), [Direction::N, Direction::S]));
        }
        ring
    }
}

fn try_crypt<R: Rng + ?Sized>(rng: &mut R, depth: u8, with_stairs_down: bool) -> Option<Layout> {
    let mut map = Map::filled(WIDTH, HEIGHT);
    let rooms = place_rooms(rng);
    if rooms.len() < 5 {
        return None;
    }
    for room in &rooms {
        for p in room.interior() {
            map.set(p, Tile::Floor);
        }
    }
    let biome = Biome::for_depth(depth);
    for room in &rooms {
        let big = room.w >= 7 && room.h >= 5;
        let (shelves, pews) = match biome {
            Biome::Crypts => (0.0, 0.25),
            Biome::Collegium => (0.5, 0.0),
            Biome::DrownedStacks => (0.3, 0.0),
            Biome::RotCourt | Biome::Throne => (0.0, 0.4),
        };
        if big && rng.random_bool(shelves) {
            add_shelves(&mut map, room);
        } else if big && rng.random_bool(pews) {
            add_pews(&mut map, room);
        } else if room.w >= 9 && room.h >= 5 && rng.random_bool(0.6) {
            add_pillars(&mut map, room);
        }
    }
    if biome == Biome::DrownedStacks {
        for room in &rooms {
            if rng.random_bool(0.5) {
                flood(rng, &mut map, room);
            }
        }
    }
    for (a, b) in connections(rng, &rooms) {
        carve_corridor(rng, &mut map, rooms[a].center(), rooms[b].center());
    }
    for room in &rooms {
        add_doors(rng, &mut map, room);
    }
    for room in &rooms {
        if rng.random_bool(0.25) {
            let count = if room.w * room.h >= 40 { 2 } else { 1 };
            for _ in 0..count {
                let tile = if rng.random_bool(COLD_BRAZIERS) {
                    Tile::ColdBrazier
                } else {
                    Tile::Brazier
                };
                place_somewhere(rng, &mut map, room, tile);
            }
        }
    }
    // Rotten boards, only where floor surrounds them, so no path needs them.
    if matches!(biome, Biome::Crypts | Biome::Collegium) {
        for _ in 0..rng.random_range(0..=2) {
            if let Some(room) = rooms.choose(rng) {
                place_somewhere(rng, &mut map, room, Tile::RottenFloor);
            }
        }
    }
    if rng.random_bool(BELL_ROPE)
        && let Some(room) = rooms.choose(rng)
    {
        let walls: Vec<Point> = room
            .wall_ring()
            .into_iter()
            .map(|(p, _)| p)
            .filter(|&p| map.tile(p) == Tile::Wall)
            .collect();
        if let Some(&p) = walls.choose(rng) {
            map.set(p, Tile::BellRope);
        }
    }

    let start_room = rng.random_range(0..rooms.len());
    let start = *rooms[start_room]
        .interior()
        .filter(|&p| map.tile(p) == Tile::Floor)
        .collect::<Vec<_>>()
        .choose(rng)?;
    map.set(start, Tile::StairsUp);

    if with_stairs_down {
        let dist = path::distances(&map, start);
        let candidates: Vec<(Point, u32)> = rooms
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != start_room)
            .flat_map(|(_, room)| room.interior())
            .filter(|&p| map.tile(p) == Tile::Floor)
            .filter_map(|p| dist.at(p).map(|d| (p, d)))
            .collect();
        let farthest = candidates.iter().map(|&(_, d)| d).max()?;
        let far_enough: Vec<Point> = candidates
            .iter()
            .filter(|&&(_, d)| d * 4 >= farthest * 3)
            .map(|&(p, _)| p)
            .collect();
        map.set(*far_enough.choose(rng)?, Tile::StairsDown);
    }

    let seep_chance = match depth {
        11 => 1.0,
        2..=10 => 0.4,
        _ => 0.0,
    };
    let seep = rng.random_bool(seep_chance).then(|| {
        let candidates: Vec<&Room> = rooms
            .iter()
            .enumerate()
            .filter(|&(i, r)| {
                i != start_room
                    && (4..=10).contains(&r.w)
                    && (3..=7).contains(&r.h)
                    && r.interior().all(|p| map.tile(p) != Tile::StairsDown)
            })
            .map(|(_, r)| r)
            .collect();
        candidates.choose(rng).map(|r| {
            (
                Point::new(r.x, r.y),
                Point::new(r.x + r.w - 1, r.y + r.h - 1),
            )
        })
    });
    Some(Layout {
        map,
        start,
        seep: seep.flatten(),
    })
}

fn place_rooms<R: Rng + ?Sized>(rng: &mut R) -> Vec<Room> {
    let target = rng.random_range(9..=13);
    let mut rooms: Vec<Room> = Vec::new();
    for attempt in 0..400 {
        if rooms.len() >= target {
            break;
        }
        let (w, h) = if attempt == 0 && rng.random_bool(0.6) {
            (rng.random_range(14..=20), rng.random_range(6..=9)) // a great hall
        } else if rng.random_bool(0.15) {
            (rng.random_range(3..=4), 3) // a cell
        } else {
            (rng.random_range(4..=11), rng.random_range(3..=7))
        };
        // Keep a wall ring inside the map border.
        let room = Room {
            x: rng.random_range(2..=WIDTH - 2 - w),
            y: rng.random_range(2..=HEIGHT - 2 - h),
            w,
            h,
        };
        if rooms.iter().all(|other| !room.too_close(other)) {
            rooms.push(room);
        }
    }
    rooms
}

/// Pairs of rooms to join: a minimum spanning tree over room centers, plus a
/// few extra links to the nearest rooms so the floor has loops.
fn connections<R: Rng + ?Sized>(rng: &mut R, rooms: &[Room]) -> Vec<(usize, usize)> {
    let dist = |a: usize, b: usize| rooms[a].center().distance_squared(rooms[b].center());
    let mut joined = vec![0];
    let mut links = Vec::new();
    while joined.len() < rooms.len() {
        let (a, b) = joined
            .iter()
            .flat_map(|&a| {
                (0..rooms.len())
                    .filter(|b| !joined.contains(b))
                    .map(move |b| (a, b))
            })
            .min_by_key(|&(a, b)| dist(a, b))
            .expect("some room is still unjoined");
        joined.push(b);
        links.push((a, b));
    }
    for _ in 0..rooms.len() / 4 {
        let a = rng.random_range(0..rooms.len());
        let mut nearest: Vec<usize> = (0..rooms.len()).filter(|&b| b != a).collect();
        nearest.sort_by_key(|&b| dist(a, b));
        if let Some(&b) = nearest[..nearest.len().min(3)].choose(rng)
            && !links.contains(&(a, b))
            && !links.contains(&(b, a))
        {
            links.push((a, b));
        }
    }
    links
}

/// An L-shaped corridor between two points, bending at a random corner.
fn carve_corridor<R: Rng + ?Sized>(rng: &mut R, map: &mut Map, from: Point, to: Point) {
    let corner = if rng.random_bool(0.5) {
        Point::new(to.x, from.y)
    } else {
        Point::new(from.x, to.y)
    };
    for (a, b) in [(from, corner), (corner, to)] {
        for x in a.x.min(b.x)..=a.x.max(b.x) {
            for y in a.y.min(b.y)..=a.y.max(b.y) {
                let p = Point::new(x, y);
                if !map.is_walkable(p) {
                    map.set(p, Tile::Floor);
                }
            }
        }
    }
}

/// Puts doors in some one-tile gaps where corridors break through a room's wall.
fn add_doors<R: Rng + ?Sized>(rng: &mut R, map: &mut Map, room: &Room) {
    for (p, [side_a, side_b]) in room.wall_ring() {
        let is_gap = map.tile(p) == Tile::Floor
            && map.tile(p + side_a) == Tile::Wall
            && map.tile(p + side_b) == Tile::Wall;
        if is_gap && rng.random_bool(0.55) {
            let tile = if rng.random_bool(CLOSED_DOORS) {
                Tile::DoorClosed
            } else {
                Tile::Door
            };
            map.set(p, tile);
        }
    }
}

/// Two colonnades along the long walls, leaving the middle aisle open.
fn add_pillars(map: &mut Map, room: &Room) {
    for p in room.interior() {
        let colonnade = p.y == room.y + 1 || p.y == room.y + room.h - 2;
        if colonnade && (p.x - room.x) % 3 == 1 {
            place_obstacle(map, p, Tile::Wall);
        }
    }
}

/// Rows of shelves across a room, each an island with floor all around it,
/// so a library never cuts the floor in two.
fn add_shelves(map: &mut Map, room: &Room) {
    let (x0, x1) = (room.x + 1, room.x + room.w - 2);
    let mut y = room.y + 1;
    while y <= room.y + room.h - 2 {
        let ring_clear = (x0 - 1..=x1 + 1).all(|x| {
            [y - 1, y, y + 1]
                .iter()
                .all(|&ry| map.tile(Point::new(x, ry)) == Tile::Floor)
        });
        if ring_clear && x1 > x0 {
            for x in x0..=x1 {
                map.set(Point::new(x, y), Tile::Bookshelf);
            }
        }
        y += 2;
    }
}

/// Floods a room: shallow water, with a deep pool in the middle of big rooms.
fn flood<R: Rng + ?Sized>(rng: &mut R, map: &mut Map, room: &Room) {
    let deep = room.w >= 6 && room.h >= 5 && rng.random_bool(0.6);
    for p in room.interior() {
        if map.tile(p) != Tile::Floor {
            continue;
        }
        let inner = p.x > room.x + 1
            && p.x < room.x + room.w - 2
            && p.y > room.y + 1
            && p.y < room.y + room.h - 2;
        map.set(
            p,
            if deep && inner {
                Tile::DeepWater
            } else {
                Tile::ShallowWater
            },
        );
    }
}

/// Rows of pews facing an aisle. Pews can be walked over.
fn add_pews(map: &mut Map, room: &Room) {
    let aisle = room.x + room.w / 2;
    for p in room.interior() {
        let row = (p.y - room.y) % 2 == 1 && p.y < room.y + room.h - 1;
        let edge = p.x == room.x || p.x == room.x + room.w - 1;
        if row && !edge && p.x != aisle && map.tile(p) == Tile::Floor {
            map.set(p, Tile::Pew);
        }
    }
}

fn place_somewhere<R: Rng + ?Sized>(rng: &mut R, map: &mut Map, room: &Room, tile: Tile) {
    let spots: Vec<Point> = room.interior().collect();
    for _ in 0..20 {
        if let Some(&p) = spots.choose(rng)
            && place_obstacle(map, p, tile)
        {
            return;
        }
    }
}

/// Places a blocking tile only if all eight neighbors are plain floor. Such a
/// tile can never disconnect the map, since its neighbors form a ring around it.
fn place_obstacle(map: &mut Map, p: Point, tile: Tile) -> bool {
    let clear = map.tile(p) == Tile::Floor
        && Direction::ALL
            .iter()
            .all(|&d| map.tile(p + d) == Tile::Floor);
    if clear {
        map.set(p, tile);
    }
    clear
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_pcg::Pcg64Mcg;

    fn layouts(count: u64, stairs: bool) -> impl Iterator<Item = (u64, Layout)> {
        (0..count).map(move |seed| {
            let mut rng = Pcg64Mcg::seed_from_u64(seed);
            (seed, crypt(&mut rng, stairs))
        })
    }

    #[test]
    fn every_floor_is_fully_connected() {
        for (seed, Layout { map, start, .. }) in layouts(300, true) {
            let dist = path::distances(&map, start);
            for p in map.points() {
                assert!(
                    !map.is_walkable(p) || dist.at(p).is_some(),
                    "seed {seed}: walkable {p:?} unreachable from start"
                );
            }
        }
    }

    #[test]
    fn stairs_are_placed_once_and_far_apart() {
        for (seed, Layout { map, start, .. }) in layouts(300, true) {
            assert_eq!(map.tile(start), Tile::StairsUp, "seed {seed}");
            assert_eq!(map.find(Tile::StairsUp).count(), 1, "seed {seed}");
            let down: Vec<Point> = map.find(Tile::StairsDown).collect();
            assert_eq!(down.len(), 1, "seed {seed}");
            assert!(
                down[0].chebyshev(start) >= 5,
                "seed {seed}: stairs too close"
            );
        }
    }

    #[test]
    fn deepest_floor_has_no_way_down() {
        for (seed, Layout { map, .. }) in layouts(50, false) {
            assert_eq!(map.find(Tile::StairsDown).count(), 0, "seed {seed}");
        }
    }

    #[test]
    fn border_is_solid_wall() {
        for (seed, Layout { map, .. }) in layouts(100, true) {
            for p in map.points() {
                let edge = p.x == 0 || p.y == 0 || p.x == WIDTH - 1 || p.y == HEIGHT - 1;
                assert!(
                    !edge || map.tile(p) == Tile::Wall,
                    "seed {seed}: hole at {p:?}"
                );
            }
        }
    }

    #[test]
    fn deep_floors_have_libraries_and_stay_connected() {
        let mut shelves = 0;
        for seed in 0..200 {
            let mut rng = Pcg64Mcg::seed_from_u64(seed);
            let Layout { map, start, .. } = floor(&mut rng, 5, true);
            shelves += map.find(Tile::Bookshelf).count();
            let dist = path::distances(&map, start);
            for p in map.points() {
                assert!(
                    !map.is_walkable(p) || dist.at(p).is_some(),
                    "seed {seed}: walkable {p:?} unreachable from start"
                );
            }
        }
        assert!(shelves > 0);
    }

    #[test]
    fn the_drowned_stacks_are_wet_and_still_have_dry_stairs() {
        let mut deep = 0;
        for seed in 0..100 {
            let mut rng = Pcg64Mcg::seed_from_u64(seed);
            let Layout { map, start, .. } = floor(&mut rng, 8, true);
            deep += map.find(Tile::DeepWater).count();
            assert!(map.find(Tile::StairsDown).count() == 1);
            assert_eq!(map.tile(start), Tile::StairsUp);
        }
        assert!(deep > 0);
    }

    #[test]
    fn same_seed_same_floor() {
        let a = crypt(&mut Pcg64Mcg::seed_from_u64(42), true);
        let b = crypt(&mut Pcg64Mcg::seed_from_u64(42), true);
        assert_eq!(a.map, b.map);
        assert_eq!(a.start, b.start);
    }
}
