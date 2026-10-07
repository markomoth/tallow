//! A dumb bot that dives for the stairs, fights what blocks it, and steps off
//! marked tiles. Prints how deep it gets over many seeds: a balance smoke test
//! until `tallow-sim` exists (M11).
//!
//! cargo run --release -p tallow-core --example bot -- [runs]

use tallow_core::map::path;
use tallow_core::{Command, Direction, MAX_DEPTH, Tile, World};

fn main() {
    let runs: u64 = std::env::args()
        .nth(1)
        .map_or(200, |a| a.parse().expect("a number"));
    let mut deaths = [0u32; MAX_DEPTH as usize + 1];
    let mut wins = 0;
    let mut turns = 0;
    for seed in 0..runs {
        let world = play(seed);
        turns += world.turn();
        match world.death() {
            Some(death) => deaths[usize::from(death.depth)] += 1,
            None => wins += 1,
        }
    }
    println!("{runs} runs, average {} turns", turns / runs);
    for (depth, &count) in deaths.iter().enumerate().skip(1) {
        println!(
            "died on floor {depth:>2}: {count:>4} {}",
            "#".repeat(count as usize * 60 / runs as usize)
        );
    }
    println!("reached floor {MAX_DEPTH}: {wins}");
}

fn play(seed: u64) -> World {
    let mut world = World::new(seed);
    for _ in 0..20_000 {
        if world.death().is_some() || world.depth() == MAX_DEPTH {
            break;
        }
        let command = choose(&world);
        world.apply(command);
    }
    world
}

fn choose(world: &World) -> Command {
    let here = world.player().pos;
    let telegraphed: Vec<_> = world.telegraphs().collect();
    let free = |p| world.map().is_walkable(p) && world.floor().monster_at(p).is_none();

    if telegraphed.contains(&here)
        && let Some(dir) = Direction::ALL
            .into_iter()
            .find(|&d| free(here + d) && !telegraphed.contains(&(here + d)))
    {
        return Command::Move(dir);
    }
    let weakest_adjacent = Direction::ALL
        .into_iter()
        .filter_map(|d| world.floor().monster_at(here + d).map(|id| (d, id)))
        .min_by_key(|&(_, id)| world.floor().monster(id).map_or(0, |m| m.health));
    if let Some((dir, _)) = weakest_adjacent {
        return Command::Move(dir);
    }
    if world.map().tile(here) == Tile::StairsDown {
        return Command::Descend;
    }
    let Some(stairs) = world.map().find(Tile::StairsDown).next() else {
        return Command::Wait;
    };
    let dist = path::distances(world.map(), stairs);
    let toward = Direction::ALL
        .into_iter()
        .filter(|&d| free(here + d) && !telegraphed.contains(&(here + d)))
        .min_by_key(|&d| dist.at(here + d).unwrap_or(u32::MAX));
    toward.map_or(Command::Wait, Command::Move)
}
