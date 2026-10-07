//! Placing monsters on a new floor.

use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

use crate::content::{Content, KindId};
use crate::geom::{Direction, Point};
use crate::map::{Map, Tile, path};

/// No monster starts closer than this many steps to the arrival stair.
const SAFE_RADIUS: u32 = 12;

/// Every floor has one stub this big (BUILD_GUIDE.md §1: a guaranteed tallow
/// source). Most of your light comes from bodies, not the floor.
pub const STUB: (u32, u32) = (50, 90);
/// Percent chance of a second stub.
const SECOND_STUB: u32 = 25;

/// Spawn budget in threat points for a depth.
pub fn budget(depth: u8) -> u32 {
    3 + 2 * u32::from(depth)
}

/// Picks monster groups for a floor and where they stand.
pub fn populate<R: Rng + ?Sized>(
    rng: &mut R,
    content: &Content,
    map: &Map,
    start: Point,
    depth: u8,
) -> Vec<(KindId, Point)> {
    let eligible = eligible(content, depth);
    let dist = path::distances(map, start);
    let open: Vec<Point> = map
        .points()
        .filter(|&p| map.tile(p) == Tile::Floor && dist.at(p).is_some_and(|d| d >= SAFE_RADIUS))
        .collect();

    let mut placed: Vec<(KindId, Point)> = Vec::new();
    let mut budget = budget(depth);
    for _ in 0..40 {
        let affordable: Vec<KindId> = eligible
            .iter()
            .copied()
            .filter(|&k| content.monster(k).threat <= budget)
            .collect();
        let Ok(&kind) = affordable.choose_weighted(rng, |&k| content.monster(k).weight) else {
            break;
        };
        let def = content.monster(kind);
        let max_group = def.group.1.min(budget / def.threat);
        let size = rng.random_range(def.group.0.min(max_group)..=max_group);
        let free = |p: &Point| !placed.iter().any(|(_, q)| q == p);
        let Some(&leader) = open
            .iter()
            .filter(|p| free(p))
            .collect::<Vec<_>>()
            .choose(rng)
        else {
            break;
        };

        let mut group = vec![*leader];
        let mut frontier = vec![*leader];
        while group.len() < size as usize {
            let Some(from) = frontier.pop() else { break };
            for dir in Direction::ALL {
                let p = from + dir;
                if group.len() < size as usize
                    && open.contains(&p)
                    && free(&p)
                    && !group.contains(&p)
                {
                    group.push(p);
                    frontier.insert(0, p);
                }
            }
        }
        budget -= def.threat * group.len() as u32;
        placed.extend(group.into_iter().map(|p| (kind, p)));
    }
    placed
}

/// The creatures that live at a depth.
pub fn eligible(content: &Content, depth: u8) -> Vec<KindId> {
    let in_range: Vec<KindId> = content
        .kinds()
        .filter(|(_, d)| d.natural && d.depth.0 <= depth && depth <= d.depth.1)
        .map(|(k, _)| k)
        .collect();
    // Until deeper biomes get their own creatures, the deepest known ones stand in.
    if in_range.is_empty() {
        content
            .kinds()
            .filter(|(_, d)| d.natural && d.depth.0 <= depth)
            .map(|(k, _)| k)
            .collect()
    } else {
        in_range
    }
}

/// Percent chance a floor below the first has writing that shows only in the dark.
const WRITING_CHANCE: u32 = 60;

/// Maybe a wall with dark-only writing on it, and the floor tile in front of it.
pub fn place_writing<R: Rng + ?Sized>(
    rng: &mut R,
    map: &Map,
    start: Point,
    depth: u8,
) -> Option<(Point, Point)> {
    if depth < 2 || rng.random_range(0..100) >= WRITING_CHANCE {
        return None;
    }
    let dist = path::distances(map, start);
    let walls: Vec<(Point, Point)> = map
        .points()
        .filter(|&p| map.tile(p) == Tile::Wall)
        .filter_map(|p| {
            Direction::CARDINAL
                .into_iter()
                .map(|d| p + d)
                .find(|&q| map.tile(q) == Tile::Floor && dist.at(q).is_some_and(|d| d >= 6))
                .map(|q| (p, q))
        })
        .collect();
    walls.choose(rng).copied()
}

/// A floor's mini-boss, if it has one, waiting by the stair down.
pub fn place_boss<R: Rng + ?Sized>(
    rng: &mut R,
    content: &Content,
    map: &Map,
    depth: u8,
) -> Vec<(KindId, Point)> {
    let Some((kind, def)) = content.kinds().find(|(_, d)| d.boss_floor == Some(depth)) else {
        return Vec::new();
    };
    let Some(stairs) = map.find(Tile::StairsDown).next() else {
        return Vec::new();
    };
    let dist = path::distances(map, stairs);
    let mut spots: Vec<Point> = map
        .points()
        .filter(|&p| {
            matches!(map.tile(p), Tile::Floor | Tile::ShallowWater)
                && dist.at(p).is_some_and(|d| (1..=5).contains(&d))
        })
        .collect();
    let count = rng.random_range(def.group.0..=def.group.1) as usize;
    let mut placed = Vec::new();
    while placed.len() < count && !spots.is_empty() {
        let at = spots.swap_remove(rng.random_range(0..spots.len()));
        placed.push((kind, at));
    }
    placed
}

/// Old bodies lying about before you came: the Sexton's crypt and the Rot Court.
pub fn place_remains<R: Rng + ?Sized>(
    rng: &mut R,
    content: &Content,
    map: &Map,
    start: Point,
    depth: u8,
) -> Vec<(KindId, Point)> {
    let Some(kind) = content.kind_by_id("parishioner") else {
        return Vec::new();
    };
    let (count, near_stairs) = match depth {
        3 => (3, true),
        10..=12 => (rng.random_range(3..=5), false),
        _ => return Vec::new(),
    };
    let anchor = if near_stairs {
        map.find(Tile::StairsDown).next().unwrap_or(start)
    } else {
        start
    };
    let dist = path::distances(map, anchor);
    let mut spots: Vec<Point> = map
        .points()
        .filter(|&p| {
            map.tile(p) == Tile::Floor
                && dist
                    .at(p)
                    .is_some_and(|d| if near_stairs { d <= 7 } else { d >= 6 })
        })
        .collect();
    let mut placed = Vec::new();
    for _ in 0..count {
        if spots.is_empty() {
            break;
        }
        placed.push((kind, spots.swap_remove(rng.random_range(0..spots.len()))));
    }
    placed
}

/// What a seep room holds.
pub struct Seep {
    pub anomalies: Vec<crate::leavings::Anomaly>,
    pub leaving: crate::leavings::Leaving,
    pub leaving_at: Point,
    /// Someone tried before you: a few stones by the way in.
    pub stones_at: Point,
    pub stones: u32,
}

/// Fills a seep room: a Leaving in the middle, invisible anomalies around it,
/// and stones lying just outside to throw.
pub fn fill_seep<R: Rng + ?Sized>(
    rng: &mut R,
    content: &Content,
    map: &Map,
    (a, b): (Point, Point),
    start: Point,
    depth: u8,
) -> Seep {
    use crate::leavings::{Anomaly, AnomalyKind, Leaving};
    let inside: Vec<Point> = map
        .points()
        .filter(|p| p.x >= a.x && p.x <= b.x && p.y >= a.y && p.y <= b.y)
        .filter(|&p| map.tile(p) == Tile::Floor)
        .collect();
    let center = Point::new((a.x + b.x) / 2, (a.y + b.y) / 2);
    let leaving_at = inside
        .iter()
        .copied()
        .min_by_key(|p| p.distance_squared(center))
        .unwrap_or(center);
    let mut spots: Vec<Point> = inside
        .iter()
        .copied()
        .filter(|&p| p != leaving_at)
        .collect();
    let kinds = [
        AnomalyKind::Heat,
        AnomalyKind::Snare,
        AnomalyKind::Pocket,
        AnomalyKind::Swap,
    ];
    let mut anomalies = Vec::new();
    for _ in 0..rng.random_range(3..=6) {
        if spots.is_empty() {
            break;
        }
        // Anomalies cluster around what they guard.
        spots.sort_by_key(|p| p.distance_squared(leaving_at));
        let near = spots.len().min(8);
        let at = spots.swap_remove(rng.random_range(0..near));
        anomalies.push(Anomaly {
            at,
            kind: *kinds.choose(rng).expect("kinds"),
            revealed: false,
        });
    }
    let leaving = content
        .leavings
        .named
        .iter()
        .find(|n| n.seep_floor == Some(depth))
        .map_or_else(
            || Leaving::roll(rng, &content.leavings),
            Leaving::from_named,
        );
    // Stones by the way in from the stair: open floor just outside the room,
    // on the near side, so you never have to cross it to reach them.
    let to_room = path::distances(map, leaving_at);
    let from_start = path::distances(map, start);
    let outside: Vec<Point> = map
        .points()
        .filter(|&p| {
            map.tile(p) == Tile::Floor
                && !(p.x >= a.x - 1 && p.x <= b.x + 1 && p.y >= a.y - 1 && p.y <= b.y + 1)
                && to_room.at(p).is_some_and(|d| d <= 8)
        })
        .collect();
    let stones_at = outside
        .iter()
        .copied()
        .min_by_key(|&p| from_start.at(p).unwrap_or(u32::MAX))
        .unwrap_or(leaving_at);
    Seep {
        anomalies,
        leaving,
        leaving_at,
        stones_at,
        stones: rng.random_range(3..=5),
    }
}

/// `at` if it is outside the room, else the nearest open floor outside it.
pub fn outside_room(map: &Map, (a, b): (Point, Point), at: Point) -> Point {
    let inside = |p: Point| p.x >= a.x - 1 && p.x <= b.x + 1 && p.y >= a.y - 1 && p.y <= b.y + 1;
    if !inside(at) {
        return at;
    }
    map.points()
        .filter(|&p| map.tile(p) == Tile::Floor && !inside(p))
        .min_by_key(|p| (p.distance_squared(at), p.y, p.x))
        .unwrap_or(at)
}

/// Items lying about on a fresh floor.
const ITEMS_PER_FLOOR: (u32, u32) = (3, 5);

/// Picks a floor's loose items (kind, count) and where they lie.
pub fn place_items<R: Rng + ?Sized>(
    rng: &mut R,
    content: &Content,
    map: &Map,
    start: Point,
    depth: u8,
) -> Vec<(Point, crate::item::ItemKindId, u32)> {
    let kinds: Vec<_> = content
        .item_kinds()
        .filter(|(_, d)| d.frequency > 0 && d.depth.0 <= depth && depth <= d.depth.1)
        .map(|(k, _)| k)
        .collect();
    let mut spots: Vec<Point> = map
        .points()
        .filter(|&p| map.tile(p) == Tile::Floor && p != start)
        .collect();
    let mut placed = Vec::new();
    for _ in 0..rng.random_range(ITEMS_PER_FLOOR.0..=ITEMS_PER_FLOOR.1) {
        let Ok(&kind) = kinds.choose_weighted(rng, |&k| content.item(k).frequency) else {
            break;
        };
        if spots.is_empty() {
            break;
        }
        let at = spots.swap_remove(rng.random_range(0..spots.len()));
        let (lo, hi) = content.item(kind).stack;
        placed.push((at, kind, rng.random_range(lo..=hi)));
    }
    // The first floor always has something to read, so every run can learn a rite.
    let is_text = |k: crate::item::ItemKindId| {
        matches!(content.item(k).class, crate::item::ItemClass::Text { .. })
    };
    if depth == 1 && !placed.iter().any(|&(_, k, _)| is_text(k)) && !spots.is_empty() {
        let texts: Vec<_> = kinds.iter().copied().filter(|&k| is_text(k)).collect();
        if let Some(&kind) = texts.choose(rng) {
            let at = spots.swap_remove(rng.random_range(0..spots.len()));
            placed.push((at, kind, 1));
        }
    }
    placed
}

/// Places a floor's tallow: one guaranteed stub, sometimes two, on plain floor.
pub fn place_tallow<R: Rng + ?Sized>(rng: &mut R, map: &Map, start: Point) -> Vec<(Point, u32)> {
    let dist = path::distances(map, start);
    let mut spots: Vec<Point> = map
        .points()
        .filter(|&p| map.tile(p) == Tile::Floor && dist.at(p).is_some_and(|d| d >= 4))
        .collect();
    let mut placed = Vec::new();
    let stubs = 1 + u32::from(rng.random_range(0..100) < SECOND_STUB);
    for _ in 0..stubs {
        if spots.is_empty() {
            break;
        }
        let at = spots.swap_remove(rng.random_range(0..spots.len()));
        placed.push((at, rng.random_range(STUB.0..=STUB.1)));
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::generate;
    use crate::rng;

    #[test]
    fn spawns_stay_within_budget_and_away_from_the_start() {
        let content = Content::bundled();
        for seed in 0..60 {
            for depth in [1, 3, 6, 12] {
                let mut r = rng::floor_rng(seed, depth);
                let layout = generate::crypt(&mut r, true);
                let spawns = populate(&mut r, content, &layout.map, layout.start, depth);
                assert!(!spawns.is_empty(), "seed {seed} depth {depth}: empty floor");
                let spent: u32 = spawns.iter().map(|&(k, _)| content.monster(k).threat).sum();
                assert!(spent <= budget(depth));
                let dist = path::distances(&layout.map, layout.start);
                for &(_, p) in &spawns {
                    assert!(dist.at(p).is_some_and(|d| d >= SAFE_RADIUS));
                    assert_eq!(layout.map.tile(p), Tile::Floor);
                }
                let mut spots: Vec<Point> = spawns.iter().map(|&(_, p)| p).collect();
                spots.sort();
                spots.dedup();
                assert_eq!(spots.len(), spawns.len(), "two monsters on one tile");
            }
        }
    }

    #[test]
    fn every_floor_has_reachable_tallow() {
        for seed in 0..200 {
            let mut r = rng::floor_rng(seed, 1);
            let layout = generate::crypt(&mut r, true);
            let tallow = place_tallow(&mut r, &layout.map, layout.start);
            let dist = path::distances(&layout.map, layout.start);
            assert!(
                tallow.iter().any(|&(_, amount)| amount >= STUB.0),
                "seed {seed}"
            );
            for &(at, _) in &tallow {
                assert!(dist.at(at).is_some(), "seed {seed}: tallow out of reach");
                assert_eq!(layout.map.tile(at), Tile::Floor);
            }
        }
    }

    #[test]
    fn the_first_floor_always_has_a_text() {
        let content = Content::bundled();
        for seed in 0..100 {
            let mut r = rng::floor_rng(seed, 1);
            let layout = generate::crypt(&mut r, true);
            let items = place_items(&mut r, content, &layout.map, layout.start, 1);
            assert!(
                items.iter().any(|&(_, k, _)| matches!(
                    content.item(k).class,
                    crate::item::ItemClass::Text { .. }
                )),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn mini_bosses_wait_by_the_stairs_on_their_floors() {
        let content = Content::bundled();
        for (depth, id, count) in [(3, "sexton", 1), (6, "provost", 1), (9, "drowned_choir", 3)] {
            for seed in 0..30 {
                let mut r = rng::floor_rng(seed, depth);
                let layout = generate::floor(&mut r, depth, true);
                let bosses = place_boss(&mut r, content, &layout.map, depth);
                assert_eq!(bosses.len(), count, "{id} seed {seed}");
                assert!(
                    bosses
                        .iter()
                        .all(|&(k, _)| k == content.kind_by_id(id).unwrap())
                );
            }
        }
        let mut r = rng::floor_rng(1, 4);
        let layout = generate::floor(&mut r, 4, true);
        assert!(place_boss(&mut r, content, &layout.map, 4).is_empty());
    }

    #[test]
    fn manifestations_never_spawn_with_a_floor() {
        let content = Content::bundled();
        let manifestation = content.kind_by_id("manifestation").unwrap();
        for seed in 0..60 {
            for depth in 1..=crate::MAX_DEPTH {
                let mut r = rng::floor_rng(seed, depth);
                let layout = generate::crypt(&mut r, true);
                let spawns = populate(&mut r, content, &layout.map, layout.start, depth);
                assert!(spawns.iter().all(|&(k, _)| k != manifestation));
            }
        }
    }

    #[test]
    fn shallow_floors_only_get_shallow_monsters() {
        let content = Content::bundled();
        let pallbearer = content.kind_by_id("pallbearer").unwrap();
        for seed in 0..60 {
            let mut r = rng::floor_rng(seed, 1);
            let layout = generate::crypt(&mut r, true);
            let spawns = populate(&mut r, content, &layout.map, layout.start, 1);
            assert!(spawns.iter().all(|&(k, _)| k != pallbearer));
        }
    }
}
