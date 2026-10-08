//! tallow-sim: plays many seeds with a plain bot and reports where runs end,
//! whether any got stuck (softlocks), and whether every floor's way onward
//! can be reached at all. A balance and sanity check, not a good player.
//!
//! cargo run --release -p tallow-sim -- [runs]

use tallow_core::map::path;
use tallow_core::{
    Cause, Command, Direction, Event, ItemClass, MAX_DEPTH, Map, Phase, Point, Stage, Tile, World,
};

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
    deaths_on_ascent: [u32; 5],
    turns: u64,
    levels: u32,
    bosses: u32,
    lords: u32,
    candles: u32,
    burned_out: u32,
    manifested: u32,
    /// What ended the runs that died, by creature (or cause).
    killers: std::collections::BTreeMap<String, u32>,
}

fn main() {
    if let Some(seed) = std::env::var("SIM_TRACE").ok().and_then(|s| s.parse().ok()) {
        trace(seed);
        return;
    }
    let runs: u64 = std::env::args()
        .nth(1)
        .map_or(200, |a| a.parse().expect("a number of runs"));
    let structural = check_floors(runs.min(300));
    let mut report = Report::default();
    for seed in 0..runs {
        play(seed, &mut report);
    }
    print(&report, &structural);
}

/// Prints the last commands of one seed's run, to see why it ended (or didn't).
fn trace(seed: u64) {
    let mut world = World::new(seed);
    let mut lines = std::collections::VecDeque::new();
    for _ in 0..MAX_COMMANDS {
        if world.death().is_some() || world.victory().is_some() {
            break;
        }
        let command = choose(&world);
        let events = world.apply(command);
        lines.push_back(format!(
            "turn {} floor {} at {:?} goal {:?} hp {}: {command:?} -> {:?}",
            world.turn(),
            world.depth(),
            world.player().pos,
            goal(&world),
            world.player().health,
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
    println!(
        "tallow {} corpse here {:?}",
        world.player().candle.tallow(),
        world.corpse_here()
    );
    let here = world.player().pos;
    for d in Direction::ALL {
        let p = here + d;
        println!(
            "{d:?}: {:?} monster {:?} burning {} anomaly {:?}",
            world.map().tile(p),
            world
                .floor()
                .monster_at(p)
                .and_then(|id| world.floor().monster(id))
                .map(|m| (
                    world.content().monster(m.kind).id.clone(),
                    m.phantom,
                    m.compelled
                )),
            world.floor().is_burning(p),
            world.anomaly_at(p)
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

fn safe_map(map: &Map) -> Map {
    let mut solid = map.clone();
    for p in map.points() {
        if matches!(map.tile(p), Tile::RottenFloor | Tile::DeepWater | Tile::Pit) {
            solid.set(p, Tile::Wall);
        }
    }
    solid
}

fn play(seed: u64, report: &mut Report) {
    let mut world = World::new(seed);
    let mut on_floor = 0;
    let mut place = (world.depth(), world.stage());
    let mut stalled = false;
    for _ in 0..MAX_COMMANDS {
        if world.death().is_some() || world.victory().is_some() {
            break;
        }
        let before = world.turn();
        let mut command = choose(&world);
        if stalled {
            // The last step cost no time (a locked door, a warning): wait instead.
            command = Command::Wait;
        }
        let events = world.apply(command);
        stalled = world.turn() == before
            && !matches!(command, Command::ChooseBoon(_) | Command::MakeRoom(_));
        for event in events {
            match event {
                Event::CandleBurnedOut => report.burned_out += 1,
                Event::Manifested { .. } => report.manifested += 1,
                Event::BossDefeated { .. } => report.bosses += 1,
                Event::LordFalls => report.lords += 1,
                Event::VigilTaken => report.candles += 1,
                _ => {}
            }
        }
        let now = (world.depth(), world.stage());
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
        "{} runs · average {} turns · average final level {:.1}",
        r.runs,
        r.turns / u64::from(r.runs.max(1)),
        f64::from(r.levels) / n
    );
    println!(
        "{:.2} mini-bosses per run · Beelzebub fell in {} · candle taken in {} · won {}",
        f64::from(r.bosses) / n,
        r.lords,
        r.candles,
        r.wins
    );
    println!(
        "candle ran dry {} times · {:.2} manifestations per run",
        r.burned_out,
        f64::from(r.manifested) / n
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
    println!("softlocks: {}", r.softlocks.len());
    for (seed, what) in r.softlocks.iter().take(10) {
        println!("  seed {seed}: {what}");
    }
    println!("unreachable stairs: {}", structural.len());
    for line in structural.iter().take(10) {
        println!("  {line}");
    }
}

/// Below this much tallow the bot walks in the dark when nothing is near.
const SNUFF_BELOW: u32 = 150;
/// It lights up past this much dread, before nightmares come for it.
const FEAR_LIGHT: u32 = 60;
/// It goes out of its way for bodies to render while it has less than this.
const RENDER_BELOW: u32 = 350;
/// Creatures within this many tiles, in line of sight, make it light up.
const DANGER_NEAR: i32 = 7;

/// The bot: pick a boon, step off marked tiles, fight what's beside it (lit),
/// rest when hurt, render bodies and grab tallow, walk dark when low, then
/// head for wherever the run goes next.
fn choose(world: &World) -> Command {
    if world.pending_draft().is_some() {
        return Command::ChooseBoon(0);
    }
    if world.pending_rite().is_some() {
        return Command::MakeRoom(world.known_rites().first().copied());
    }
    let here = world.player().pos;
    let floor = world.floor();
    let telegraphed: Vec<Point> = world.telegraphs().collect();
    let free = |p: Point| {
        world.map().is_walkable(p)
            && floor.monster_at(p).is_none()
            && !floor.is_burning(p)
            && !matches!(world.map().tile(p), Tile::RottenFloor | Tile::DeepWater)
            && world.anomaly_at(p).is_none_or(|a| !a.revealed)
    };
    // Step off marked tiles, staying close to whatever raised the blow so the
    // next swing can answer it.
    let threat = floor
        .monsters()
        .filter(|(_, m)| m.winding_up.is_some())
        .map(|(_, m)| m.pos)
        .min_by_key(|p| p.distance_squared(here));
    if telegraphed.contains(&here)
        && let Some(dir) = Direction::ALL
            .into_iter()
            .filter(|&d| free(here + d) && !telegraphed.contains(&(here + d)))
            .min_by_key(|&d| threat.map_or(0, |t| (here + d).chebyshev(t)))
    {
        return Command::Move(dir);
    }
    let following = world.content().kind_by_id("the_following");
    let hostile_beside = Direction::ALL
        .into_iter()
        .filter_map(|d| floor.monster_at(here + d).map(|id| (d, id)))
        .filter(|&(_, id)| {
            floor
                .monster(id)
                .is_some_and(|m| m.compelled == 0 && Some(m.kind) != following)
        })
        .min_by_key(|&(_, id)| floor.monster(id).map_or(0, |m| m.health));
    // Light to fight; walk dark to save tallow when it runs low and nothing is near.
    let player = world.player();
    let danger = floor.monsters().any(|(_, m)| {
        !m.phantom
            && m.compelled == 0
            && m.pos.chebyshev(here) <= DANGER_NEAR
            && floor.in_sight(m.pos)
    });
    let tallow = player.candle.tallow();
    let lit = player.candle.is_lit();
    if !player.vigil && tallow > 0 {
        let want_lit = danger
            || hostile_beside.is_some()
            || tallow >= SNUFF_BELOW
            || player.dread.value() >= FEAR_LIGHT;
        if want_lit != lit {
            return Command::ToggleCandle;
        }
    }
    if let Some((dir, _)) = hostile_beside {
        return Command::Move(dir);
    }
    // Short of tallow, it hunts: renders the body underfoot, or goes after
    // whatever it can see (which also keeps rendering from being refused).
    let short = tallow < RENDER_BELOW && world.stage() == Stage::Descent;
    if short && world.corpse_here().is_some() && !rendering_refused(world) {
        return Command::Render;
    }
    // The nearest creature it could walk to: a stable target, seen or not.
    let chase = short
        .then(|| {
            // Walk only where it walks: no seep rooms.
            let mut solid = safe_map(world.map());
            for p in world.map().points() {
                if floor.is_seep(p) {
                    solid.set(p, Tile::Wall);
                }
            }
            let dist = path::distances(&solid, here);
            floor
                .monsters()
                .filter(|(_, m)| {
                    !m.phantom && m.compelled == 0 && !world.content().monster(m.kind).boss
                })
                .filter_map(|(_, m)| {
                    Direction::ALL
                        .into_iter()
                        .filter_map(|d| dist.at(m.pos + d))
                        .min()
                        .filter(|&d| d <= 20)
                        .map(|d| (d, m.pos))
                })
                .min()
                .map(|(_, at)| at)
        })
        .flatten();
    if player.health * 5 < player.max_health * 2
        && world.hostiles_in_view().is_empty()
        && player.dread.value() < 90
    {
        return Command::Rest;
    }
    let Some(mut goal) = chase.or_else(|| goal(world)) else {
        return Command::Wait;
    };
    let reachable = |goal: Point| {
        let mut solid = safe_map(world.map());
        solid.set(goal, Tile::Floor);
        for p in world.map().points() {
            if world.floor().is_seep(p) && p != goal {
                solid.set(p, Tile::Wall);
            }
        }
        path::distances(&solid, goal).at(here).is_some()
    };
    if !reachable(goal)
        && let Some(stairs) = world.map().find(Tile::StairsDown).next()
    {
        goal = stairs;
    }
    if goal == here {
        return match world.stage() {
            Stage::Descent if world.map().tile(here) == Tile::StairsDown => Command::Descend,
            _ if world.map().tile(here) == Tile::StairsUp => Command::Ascend,
            _ => Command::PickUp,
        };
    }
    let mut solid = safe_map(world.map());
    // Keep out of seep rooms and away from known anomalies.
    for p in world.map().points() {
        if world.floor().is_seep(p) && p != goal {
            solid.set(p, Tile::Wall);
        }
    }
    // The altar can't be stood on; measure from it as if it could.
    solid.set(goal, Tile::Floor);
    let dist = path::distances(&solid, goal);
    let altar = world.map().tile(goal) == Tile::Altar;
    Direction::ALL
        .into_iter()
        .filter(|&d| {
            (free(here + d) || (altar && here + d == goal)) && !telegraphed.contains(&(here + d))
        })
        .min_by_key(|&d| {
            if here + d == goal {
                0
            } else {
                dist.at(here + d).unwrap_or(u32::MAX)
            }
        })
        .map_or(Command::Wait, Command::Move)
}

/// Rendering the body underfoot would be refused right now.
fn rendering_refused(world: &World) -> bool {
    world.clone().apply(Command::Render) == [Event::RunRefused]
}

/// Where the bot wants to be next.
fn goal(world: &World) -> Option<Point> {
    let floor = world.floor();
    match world.stage() {
        Stage::Church => world.map().find(Tile::Altar).next(),
        Stage::Ascent(_) => world.map().find(Tile::StairsUp).next(),
        Stage::Descent if world.depth() == MAX_DEPTH => {
            let lord = world.lord()?;
            if world.player().vigil {
                return world.map().find(Tile::StairsUp).next();
            }
            if lord.phase == Phase::Fallen {
                return floor
                    .items()
                    .iter()
                    .find(|f| matches!(world.content().item(f.item.kind).class, ItemClass::Relic))
                    .map(|f| f.at);
            }
            // Go and fight whatever form he is in.
            floor
                .monsters()
                .filter(|(_, m)| world.content().monster(m.kind).boss)
                .map(|(_, m)| m.pos)
                .min_by_key(|p| p.distance_squared(world.player().pos))
        }
        Stage::Descent => {
            // Tallow it has seen and can walk to soon, or a body to render
            // while it's short; walking distance only shrinks on the way, so
            // it doesn't dither between goals.
            let here = world.player().pos;
            let dist = path::distances(&safe_map(world.map()), here);
            let light = world.burden() == tallow_core::Burden::Light;
            let tallow = floor
                .tallow()
                .iter()
                .filter(|t| light && floor.is_explored(t.at) && t.at != here)
                .map(|t| t.at);
            let short = world.player().candle.tallow() < RENDER_BELOW;
            let bodies = floor
                .corpses()
                .iter()
                .filter(|c| short && light && floor.is_explored(c.at) && c.at != here)
                .map(|c| c.at);
            tallow
                .chain(bodies)
                .filter_map(|at| dist.at(at).filter(|&d| d <= 25).map(|d| (d, at)))
                .min()
                .map(|(_, at)| at)
                .or_else(|| world.map().find(Tile::StairsDown).next())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_runs_all_end() {
        let mut report = Report::default();
        for seed in 0..3 {
            play(seed, &mut report);
        }
        assert_eq!(report.runs, 3);
        assert!(report.softlocks.is_empty(), "{:?}", report.softlocks);
    }
}
