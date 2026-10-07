//! A dumb bot that dives for the stairs, grabs tallow it sees, fights what
//! blocks it, and steps off marked tiles. Prints how deep it gets over many seeds: a balance smoke test
//! until `tallow-sim` exists (M11).
//!
//! cargo run --release -p tallow-core --example bot -- [runs]

use tallow_core::map::path;
use tallow_core::{Command, Direction, Event, MAX_DEPTH, Tile, World};

#[derive(Default)]
struct Stats {
    burned_out: u32,
    manifested: u32,
    peak_dread: u32,
    tallow_left_at_end: u32,
}

fn main() {
    let runs: u64 = std::env::args()
        .nth(1)
        .map_or(200, |a| a.parse().expect("a number"));
    let mut deaths = [0u32; MAX_DEPTH as usize + 1];
    let mut wins = 0;
    let mut turns = 0;
    let mut totals = Stats::default();
    let mut ran_dry = 0;
    for seed in 0..runs {
        let (world, stats) = play(seed);
        turns += world.turn();
        match world.death() {
            Some(death) => deaths[usize::from(death.depth)] += 1,
            None => wins += 1,
        }
        ran_dry += u32::from(stats.burned_out > 0);
        totals.manifested += stats.manifested;
        totals.peak_dread += stats.peak_dread;
        totals.tallow_left_at_end += stats.tallow_left_at_end;
    }
    let n = runs as u32;
    println!("{runs} runs, average {} turns", turns / runs);
    println!(
        "candle ran dry in {ran_dry} runs; {:.2} manifestations per run; average peak dread {}; average tallow left {}",
        f64::from(totals.manifested) / f64::from(n),
        totals.peak_dread / n,
        totals.tallow_left_at_end / n,
    );
    for (depth, &count) in deaths.iter().enumerate().skip(1) {
        println!(
            "died on floor {depth:>2}: {count:>4} {}",
            "#".repeat(count as usize * 60 / runs as usize)
        );
    }
    println!("reached floor {MAX_DEPTH}: {wins}");
}

fn play(seed: u64) -> (World, Stats) {
    let mut world = World::new(seed);
    let mut stats = Stats::default();
    for _ in 0..20_000 {
        if world.death().is_some() || world.depth() == MAX_DEPTH {
            break;
        }
        let command = choose(&world);
        for event in world.apply(command) {
            match event {
                Event::CandleBurnedOut => stats.burned_out += 1,
                Event::Manifested => stats.manifested += 1,
                _ => {}
            }
        }
        stats.peak_dread = stats.peak_dread.max(world.player().dread.value());
    }
    stats.tallow_left_at_end = world.player().candle.tallow();
    (world, stats)
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
    // Tallow in view is worth a detour; otherwise head for the stairs.
    let tallow = world
        .floor()
        .tallow()
        .iter()
        .find(|t| world.floor().is_visible(t.at))
        .map(|t| t.at);
    if tallow.is_none() && world.map().tile(here) == Tile::StairsDown {
        return Command::Descend;
    }
    let Some(goal) = tallow.or_else(|| world.map().find(Tile::StairsDown).next()) else {
        return Command::Wait;
    };
    let dist = path::distances(world.map(), goal);
    let toward = Direction::ALL
        .into_iter()
        .filter(|&d| free(here + d) && !telegraphed.contains(&(here + d)))
        .min_by_key(|&d| dist.at(here + d).unwrap_or(u32::MAX));
    toward.map_or(Command::Wait, Command::Move)
}
