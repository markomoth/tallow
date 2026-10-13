//! tallow-sim: plays many seeds with bots and reports where runs end,
//! whether any got stuck (softlocks), and whether every floor's way onward
//! can be reached at all. A balance and sanity check, not a good player.
//!
//! Each bot plays one [`bot::Style`]: `mixed` uses every answer the game
//! has, the others lean on one (see `bot::STYLES`). One trick alone should
//! rarely win; the mix should win clearly more often.
//!
//! cargo run --release -p tallow-sim -- [runs] [style | all | ablate]
//! SIM_TRACE=17 [SIM_STYLE=rites] cargo run --release -p tallow-sim

mod bot;

use std::collections::BTreeMap;

use bot::{Bot, STYLES, Style, safe_map};
use tallow_core::map::path;
use tallow_core::{Cause, Command, Direction, Event, MAX_DEPTH, Map, Point, Tile, World};

/// A run that takes this many commands without ending is stuck.
const MAX_COMMANDS: u32 = 30_000;
/// So is one that spends this many on a single floor.
const MAX_ON_FLOOR: u32 = 5_000;

#[derive(Default)]
struct Report {
    runs: u32,
    wins: u32,
    softlocks: Vec<(u64, String)>,
    deaths_by_floor: [u32; MAX_DEPTH as usize + 1],
    deaths_on_ascent: [u32; 6],
    turns: u64,
    levels: u32,
    /// Deepest floor reached, summed.
    depth: u32,
    bosses: u32,
    lords: u32,
    candles: u32,
    burned_out: u32,
    manifested: u32,
    casts: u32,
    shots: u32,
    kills: u32,
    /// Damage dealt by blows and by missiles.
    melee_damage: u64,
    missile_damage: u64,
    /// Fired and thrown, by item.
    shots_by: BTreeMap<String, u32>,
    /// What ended the runs that died, by creature (or cause).
    killers: BTreeMap<String, u32>,
}

fn main() {
    let named = std::env::var("SIM_STYLE").ok();
    if let Some(seed) = std::env::var("SIM_TRACE").ok().and_then(|s| s.parse().ok()) {
        trace(seed, find_style(named.as_deref().unwrap_or("mixed")));
        return;
    }
    let mut args = std::env::args().skip(1);
    let runs: u64 = args
        .next()
        .map_or(200, |a| a.parse().expect("a number of runs"));
    let which = args.next().unwrap_or_else(|| "mixed".into());
    if which == "all" {
        compare(STYLES, runs);
        return;
    }
    if which == "ablate" {
        compare(bot::ABLATIONS, runs);
        return;
    }
    let style = find_style(&which);
    let structural = check_floors(runs.min(300));
    let report = play_many(style, runs);
    println!("{}: {}", style.name, style.about);
    print(&report, &structural);
}

fn find_style(name: &str) -> &'static Style {
    bot::style(name).unwrap_or_else(|| {
        let names: Vec<_> = STYLES.iter().map(|s| s.name).collect();
        panic!("no style {name:?}; try one of {}", names.join(", "))
    })
}

fn play_many(style: &'static Style, runs: u64) -> Report {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get() as u64);
    let reports: Vec<Report> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                scope.spawn(move || {
                    let mut report = Report::default();
                    for seed in (t..runs).step_by(threads as usize) {
                        play(seed, style, &mut report);
                    }
                    report
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut total = Report::default();
    for r in reports {
        total.merge(r);
    }
    total.softlocks.sort();
    total
}

/// Every style on the same seeds, side by side.
fn compare(styles: &'static [Style], runs: u64) {
    println!(
        "{:<9} {:>6} {:>6} {:>6} {:>6} {:>5} {:>6} {:>6} {:>6} {:>6} {:>5}  top killers",
        "style",
        "won",
        "lord",
        "depth",
        "turns",
        "level",
        "manif",
        "dry",
        "casts",
        "shot%",
        "stuck"
    );
    for style in styles {
        let r = play_many(style, runs);
        let n = f64::from(r.runs.max(1));
        let mut killers: Vec<_> = r.killers.iter().collect();
        killers.sort_by_key(|&(name, &count)| (std::cmp::Reverse(count), name.clone()));
        println!(
            "{:<9} {:>5.1}% {:>5.1}% {:>6.1} {:>6} {:>5.1} {:>6.2} {:>6.2} {:>6.1} {:>5.0}% {:>5}  {}",
            style.name,
            100.0 * f64::from(r.wins) / n,
            100.0 * f64::from(r.lords) / n,
            f64::from(r.depth) / n,
            r.turns / u64::from(r.runs.max(1)),
            f64::from(r.levels) / n,
            f64::from(r.manifested) / n,
            f64::from(r.burned_out) / n,
            f64::from(r.casts) / n,
            100.0 * r.missile_damage as f64 / (r.melee_damage + r.missile_damage).max(1) as f64,
            r.softlocks.len(),
            killers
                .iter()
                .take(4)
                .map(|(name, count)| format!("{name} {count}"))
                .collect::<Vec<_>>()
                .join(" · ")
        );
    }
}

impl Report {
    fn merge(&mut self, o: Report) {
        self.runs += o.runs;
        self.wins += o.wins;
        self.softlocks.extend(o.softlocks);
        for (a, b) in self.deaths_by_floor.iter_mut().zip(o.deaths_by_floor) {
            *a += b;
        }
        for (a, b) in self.deaths_on_ascent.iter_mut().zip(o.deaths_on_ascent) {
            *a += b;
        }
        self.turns += o.turns;
        self.levels += o.levels;
        self.depth += o.depth;
        self.bosses += o.bosses;
        self.lords += o.lords;
        self.candles += o.candles;
        self.burned_out += o.burned_out;
        self.manifested += o.manifested;
        self.casts += o.casts;
        self.shots += o.shots;
        self.kills += o.kills;
        self.melee_damage += o.melee_damage;
        self.missile_damage += o.missile_damage;
        for (k, v) in o.shots_by {
            *self.shots_by.entry(k).or_default() += v;
        }
        for (k, v) in o.killers {
            *self.killers.entry(k).or_default() += v;
        }
    }
}

/// Prints the last commands of one seed's run, to see why it ended (or didn't).
fn trace(seed: u64, style: &'static Style) {
    let mut world = World::new(seed);
    let mut bot = Bot::new(style);
    let mut lines = std::collections::VecDeque::new();
    let mut stalled = 0;
    for _ in 0..MAX_COMMANDS {
        if world.death().is_some() || world.victory().is_some() {
            break;
        }
        let thought = bot.debug(&world);
        let before = world.turn();
        let mut command = bot.choose(&world);
        if stalled >= 2 {
            command = Command::Wait;
        }
        let events = world.apply(command);
        bot.saw(&events);
        stalled = if world.turn() == before {
            stalled + 1
        } else {
            0
        };
        lines.push_back(format!(
            "turn {} floor {} at {:?} hp {} tallow {} dread {} {}: {command:?} -> {:?}",
            world.turn(),
            world.depth(),
            world.player().pos,
            world.player().health,
            world.player().candle.tallow(),
            world.player().dread.value(),
            thought,
            events.iter().take(3).collect::<Vec<_>>()
        ));
        if lines.len() > 25 {
            lines.pop_front();
        }
    }
    for line in lines {
        println!("{line}");
    }
    for id in world.hostiles_in_view() {
        let m = world.floor().monster(id).unwrap();
        println!(
            "in view: {} at {:?} {:?} phantom {}",
            world.content().monster(m.kind).id,
            m.pos,
            m.mind,
            m.phantom
        );
    }
    let here = world.player().pos;
    for d in Direction::ALL {
        let p = here + d;
        println!(
            "{d:?}: {:?} monster {:?} burning {}",
            world.map().tile(p),
            world
                .floor()
                .monster_at(p)
                .and_then(|id| world.floor().monster(id))
                .map(|m| (world.content().monster(m.kind).id.clone(), m.phantom)),
            world.floor().is_burning(p),
        );
    }
}

/// Every floor of every seed: is the stair onward reachable from where you arrive?
fn check_floors(seeds: u64) -> Vec<String> {
    let mut problems = Vec::new();
    for seed in 0..seeds {
        let mut world = World::new(seed);
        for depth in 1..MAX_DEPTH {
            if depth > 1 {
                world.dev_skip_to(depth);
            }
            if !reachable(world.map(), world.player().pos, Tile::StairsDown) {
                problems.push(format!("seed {seed} floor {depth}: no way down"));
            }
        }
        for floor in 1..=4 {
            let mut world = World::new(seed);
            world.dev_ascent(floor);
            if !reachable(world.map(), world.player().pos, Tile::StairsUp) {
                problems.push(format!("seed {seed} ascent {floor}: no way up"));
            }
        }
    }
    problems
}

/// Reachable without stepping on rotten boards or into deep water.
fn reachable(map: &Map, from: Point, tile: Tile) -> bool {
    let solid = safe_map(map);
    let dist = path::distances(&solid, from);
    map.find(tile).any(|p| dist.at(p).is_some())
}

fn play(seed: u64, style: &'static Style, report: &mut Report) {
    let mut world = World::new(seed);
    let mut bot = Bot::new(style);
    let mut on_floor = 0;
    let mut place = (world.depth(), world.stage());
    let mut deepest = 1;
    let mut stalled = 0;
    for _ in 0..MAX_COMMANDS {
        if world.death().is_some() || world.victory().is_some() {
            break;
        }
        let before = world.turn();
        let mut command = bot.choose(&world);
        if stalled >= 2 {
            // The last steps cost no time (a locked door, a warning): wait instead.
            command = Command::Wait;
        }
        let events = world.apply(command);
        bot.saw(&events);
        stalled = if world.turn() == before
            && !matches!(command, Command::ChooseBoon(_) | Command::MakeRoom(_))
        {
            stalled + 1
        } else {
            0
        };
        for event in events {
            match event {
                Event::CandleBurnedOut => report.burned_out += 1,
                Event::Manifested { .. } => report.manifested += 1,
                Event::BossDefeated { .. } => report.bosses += 1,
                Event::LordFalls => report.lords += 1,
                Event::VigilTaken => report.candles += 1,
                Event::Cast { rite } => {
                    report.casts += 1;
                    let id = format!("cast {}", world.content().rite(rite).id);
                    *report.shots_by.entry(id).or_default() += 1;
                }
                Event::Attack {
                    attacker: tallow_core::Who::Player,
                    damage: Some(d),
                    ..
                } => report.melee_damage += u64::from(d),
                Event::Fired { kind } => {
                    let id = format!("fired {}", world.content().item(kind).id);
                    *report.shots_by.entry(id).or_default() += 1;
                }
                Event::Thrown { kind } => {
                    let id = format!("threw {}", world.content().item(kind).id);
                    *report.shots_by.entry(id).or_default() += 1;
                }
                Event::ProjectileHit {
                    damage: Some(d), ..
                } => {
                    report.missile_damage += u64::from(d);
                }
                _ => {}
            }
        }
        if matches!(command, Command::Fire { .. } | Command::Throw { .. }) {
            report.shots += 1;
        }
        let now = (world.depth(), world.stage());
        deepest = deepest.max(world.depth());
        if now == place {
            on_floor += 1;
        } else {
            place = now;
            on_floor = 0;
        }
        if on_floor > MAX_ON_FLOOR {
            break;
        }
    }
    report.runs += 1;
    report.turns += world.turn();
    report.levels += world.player().level;
    report.depth += u32::from(deepest);
    match (world.death(), world.victory()) {
        (Some(death), _) => {
            if death.ascent > 0 {
                report.deaths_on_ascent[usize::from(death.ascent)] += 1;
            } else {
                report.deaths_by_floor[usize::from(death.depth)] += 1;
            }
            let killer = match death.cause {
                Cause::Attack(kind) | Cause::HeavyBlow(kind) | Cause::Chant(kind) => {
                    world.content().monster(kind).id.clone()
                }
                other => format!("{other:?}").to_lowercase(),
            };
            *report.killers.entry(killer).or_default() += 1;
        }
        (None, Some(_)) => report.wins += 1,
        (None, None) => report.softlocks.push((
            seed,
            format!(
                "stuck on {:?} floor {} at turn {}",
                world.stage(),
                world.depth(),
                world.turn()
            ),
        )),
    }
}

fn print(r: &Report, structural: &[String]) {
    let n = f64::from(r.runs.max(1));
    println!(
        "{} runs · average {} turns · average final level {:.1} · deepest floor {:.1}",
        r.runs,
        r.turns / u64::from(r.runs.max(1)),
        f64::from(r.levels) / n,
        f64::from(r.depth) / n
    );
    println!(
        "{:.2} mini-bosses per run · Beelzebub fell in {} · candle taken in {} · won {}",
        f64::from(r.bosses) / n,
        r.lords,
        r.candles,
        r.wins
    );
    println!(
        "candle ran dry {} times · {:.2} manifestations per run · {:.1} rites and {:.1} shots per run",
        r.burned_out,
        f64::from(r.manifested) / n,
        f64::from(r.casts) / n,
        f64::from(r.shots) / n
    );
    for (depth, &count) in r.deaths_by_floor.iter().enumerate().skip(1) {
        println!(
            "died on floor {depth:>2}: {count:>4} {}",
            "#".repeat((f64::from(count) * 60.0 / n) as usize)
        );
    }
    for (floor, &count) in r.deaths_on_ascent.iter().enumerate().skip(1) {
        println!("died on ascent {floor}: {count:>4}");
    }
    let mut killers: Vec<_> = r.killers.iter().collect();
    killers.sort_by_key(|&(name, &count)| (std::cmp::Reverse(count), name.clone()));
    println!(
        "killed by: {}",
        killers
            .iter()
            .take(8)
            .map(|(name, count)| format!("{name} {count}"))
            .collect::<Vec<_>>()
            .join(" · ")
    );
    println!(
        "used: {}",
        r.shots_by
            .iter()
            .map(|(k, v)| format!("{k} {:.1}", f64::from(*v) / n))
            .collect::<Vec<_>>()
            .join(" · ")
    );
    println!("softlocks: {}", r.softlocks.len());
    for (seed, what) in r.softlocks.iter().take(10) {
        println!("  seed {seed}: {what}");
    }
    println!("unreachable stairs: {}", structural.len());
    for line in structural.iter().take(10) {
        println!("  {line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_runs_of_every_style_end() {
        for style in STYLES {
            let mut report = Report::default();
            for seed in 0..2 {
                play(seed, style, &mut report);
            }
            assert_eq!(report.runs, 2);
            assert!(
                report.softlocks.is_empty(),
                "{}: {:?}",
                style.name,
                report.softlocks
            );
        }
    }
}
