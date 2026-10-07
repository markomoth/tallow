//! Placing monsters on a new floor.

use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

use crate::content::{Content, KindId};
use crate::geom::{Direction, Point};
use crate::map::{Map, Tile, path};

/// No monster starts closer than this many steps to the arrival stair.
const SAFE_RADIUS: u32 = 12;

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
    let in_range: Vec<KindId> = content
        .kinds()
        .filter(|(_, d)| d.depth.0 <= depth && depth <= d.depth.1)
        .map(|(k, _)| k)
        .collect();
    // Until deeper biomes get their own creatures, the deepest known ones stand in.
    let eligible = if in_range.is_empty() {
        content
            .kinds()
            .filter(|(_, d)| d.depth.0 <= depth)
            .map(|(k, _)| k)
            .collect()
    } else {
        in_range
    };

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
