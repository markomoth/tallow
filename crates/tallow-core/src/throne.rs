//! The bottom and the way back up (BUILD_GUIDE.md §3): Beelzebub's throne
//! room and his three phases, the Vigil Candle, the four floors of the ascent
//! with the Following behind you, and the altar.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::content::{KindId, Trait};
use crate::events::Event;
use crate::geom::{Direction, Point};
use crate::map::{Map, Tile};
use crate::monster::{Mind, Monster, MonsterId};
use crate::rites::{RiteEffect, School};
use crate::world::World;

/// Floors in the ascent.
pub const ASCENT_FLOORS: u8 = 4;
/// Turns after you arrive on an ascent floor before the Following comes up behind you.
pub const FOLLOWING_DELAY: [u64; ASCENT_FLOORS as usize] = [30, 25, 22, 18];
/// Turns of warning before it arrives.
pub const FOLLOWING_WARNING: u64 = 10;
/// The swarm coat Beelzebub starts the fight in.
pub const SWARM_MAX: u32 = 12;
/// Old bodies on the throne room floor: hosts for the second phase.
const THRONE_BODIES: usize = 5;
/// Turns a shut door holds the Following; a sealed one holds longer.
pub const GNAW_DOOR: u32 = 3;
pub const GNAW_SEAL: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Armored in a swarm that thickens in darkness.
    Court,
    /// Leaping between bodies.
    Possession,
    /// Slow and huge; every blow marked a turn ahead.
    Lord,
    /// Gone. The candle is free.
    Fallen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lord {
    pub phase: Phase,
    /// The coat of flies: absorbs blows while it lasts.
    pub swarm: u32,
    /// Pinned in his current body by a rite.
    pub bound: bool,
    /// Gathering over a body; he rises in it on this turn.
    pub leap: Option<(Point, u64)>,
}

/// How a run was won.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Victory {
    pub turn: u64,
    pub exorcised: u32,
    pub dread: u32,
    pub rites: u32,
    pub level: u32,
}

/// Where you are in the run, beyond plain depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Descent,
    /// Ascent floor 1..=4.
    Ascent(u8),
    /// The church above, with the altar.
    Church,
}

/// The throne room: one great hall with pillars, cold braziers, old bodies and
/// a dais to the north. Returns the map, the arrival stair, the dais, where
/// the bodies lie and where Beelzebub waits.
pub fn throne_room<R: Rng + ?Sized>(rng: &mut R) -> (Map, Point, Point, Vec<Point>, Point) {
    let (w, h) = (crate::map::generate::WIDTH, crate::map::generate::HEIGHT);
    let mut map = Map::filled(w, h);
    let (x0, x1, y0, y1) = (10, 69, 5, 22);
    for y in y0..=y1 {
        for x in x0..=x1 {
            map.set(Point::new(x, y), Tile::Floor);
        }
    }
    // The way in from the stair: a short corridor and an antechamber.
    for y in y1 + 1..=26 {
        map.set(Point::new(39, y), Tile::Floor);
    }
    map.set(Point::new(39, y1 + 1), Tile::DoorClosed);
    for y in 27..=31 {
        for x in 35..=44 {
            map.set(Point::new(x, y), Tile::Floor);
        }
    }
    let start = Point::new(39, 30);
    map.set(start, Tile::StairsUp);
    // Two rows of pillars, and cold braziers between them.
    for x in (x0 + 3..=x1 - 3).step_by(6) {
        map.set(Point::new(x, y0 + 4), Tile::Wall);
        map.set(Point::new(x, y1 - 4), Tile::Wall);
    }
    for &(x, y) in &[(19, 9), (19, 18), (60, 9), (60, 18), (31, 13), (48, 13)] {
        map.set(Point::new(x, y), Tile::ColdBrazier);
    }
    let dais = Point::new(39, y0 + 1);
    let lord = Point::new(39, y0 + 4);
    let open: Vec<Point> = map
        .points()
        .filter(|&p| {
            map.tile(p) == Tile::Floor
                && p.y > y0 + 2
                && p.y < y1 - 1
                && p.chebyshev(lord) > 3
                && Direction::ALL
                    .iter()
                    .all(|&d| map.tile(p + d) == Tile::Floor)
        })
        .collect();
    let bodies: Vec<Point> = open.sample(rng, THRONE_BODIES).copied().collect();
    (map, start, dais, bodies, lord)
}

/// The church above: the nave, pews, and the altar at the east end.
pub fn church() -> (Map, Point) {
    let text = "\
########################
#....&.......&......&..#
#.==.==.==.==.==.==....#
#......................#
#@...................A.#
#......................#
#.==.==.==.==.==.==....#
#....&.......&......&..#
########################";
    crate::map::prefab::parse(text).expect("the church is well drawn")
}

impl World {
    pub fn stage(&self) -> Stage {
        match self.ascent {
            0 => Stage::Descent,
            a if a <= ASCENT_FLOORS => Stage::Ascent(a),
            _ => Stage::Church,
        }
    }

    pub fn lord(&self) -> Option<Lord> {
        self.lord
    }

    pub fn victory(&self) -> Option<Victory> {
        self.victory
    }

    /// Turns until the Following comes up the stair, or `None` once it is here
    /// (or if nothing follows).
    pub fn following_in(&self) -> Option<u64> {
        self.following_at.map(|t| t.saturating_sub(self.turn()))
    }

    pub fn following_present(&self) -> bool {
        let kind = self.content.kind_by_id("the_following");
        self.floor.monsters().any(|(_, m)| Some(m.kind) == kind)
    }

    fn kind(&self, id: &str) -> KindId {
        self.content
            .kind_by_id(id)
            .expect("throne creatures are defined")
    }

    /// Sets up floor 12 for the fight.
    pub(crate) fn begin_throne(&mut self) {
        self.lord = Some(Lord {
            phase: Phase::Court,
            swarm: SWARM_MAX,
            bound: false,
            leap: None,
        });
    }

    /// Blows that don't land: the undying, and the swarm coat. True if absorbed.
    pub(crate) fn absorb_blow(
        &mut self,
        id: MonsterId,
        damage: u32,
        events: &mut Vec<Event>,
    ) -> bool {
        let kind = self.floor.monsters[id].kind;
        let def = self.content.monster(kind);
        if def.has(|t| *t == Trait::Undying) {
            events.push(Event::Undying { kind });
            return true;
        }
        if damage > 0
            && kind == self.kind("beelzebub")
            && let Some(lord) = &mut self.lord
            && lord.swarm > 0
        {
            lord.swarm = lord.swarm.saturating_sub(1 + damage / 3);
            events.push(Event::SwarmAbsorbs { left: lord.swarm });
            return true;
        }
        false
    }

    /// Phase changes when one of his bodies dies.
    pub(crate) fn throne_death(&mut self, kind: KindId, at: Point, events: &mut Vec<Event>) {
        let Some(mut lord) = self.lord else { return };
        if kind == self.kind("beelzebub") {
            lord.phase = Phase::Possession;
            events.push(Event::LordLeavesBody);
            self.lord = Some(lord);
            self.plan_leap(at, events);
        } else if kind == self.kind("beelzebub_host") {
            if lord.bound {
                self.lord = Some(lord);
                self.rise_as_lord(at, events);
            } else {
                self.plan_leap(at, events);
            }
        } else if kind == self.kind("beelzebub_lord") {
            lord.phase = Phase::Fallen;
            self.lord = Some(lord);
            events.push(Event::LordFalls);
        }
    }

    /// Picks the next body for him, a turn ahead; with none left, he rises as himself.
    fn plan_leap(&mut self, from: Point, events: &mut Vec<Event>) {
        let body = self
            .floor
            .corpses
            .iter()
            .filter(|c| self.floor.monster_at(c.at).is_none() && c.at != self.player.pos)
            .min_by_key(|c| c.at.distance_squared(from))
            .map(|c| c.at);
        let now = self.turn();
        match body {
            Some(at) => {
                if let Some(lord) = &mut self.lord {
                    lord.leap = Some((at, now + 1));
                    lord.bound = false;
                }
                events.push(Event::LordGathers { at });
            }
            None => self.rise_as_lord(from, events),
        }
    }

    fn rise_as_lord(&mut self, at: Point, events: &mut Vec<Event>) {
        if let Some(lord) = &mut self.lord {
            lord.phase = Phase::Lord;
            lord.leap = None;
        }
        let spot = self.free_near(at);
        let kind = self.kind("beelzebub_lord");
        self.spawn_at(kind, spot);
        events.push(Event::LordRises);
    }

    /// A Binding rite or Exorcise on his borrowed body pins him in it.
    pub(crate) fn maybe_bind_lord(
        &mut self,
        victim: Option<MonsterId>,
        school: School,
        effect: RiteEffect,
        events: &mut Vec<Event>,
    ) {
        let Some(id) = victim else { return };
        let Some(m) = self.floor.monsters.get(id) else {
            return;
        };
        let vessel = self.content.monster(m.kind).has(|t| *t == Trait::Vessel);
        let binds = school == School::Binding || matches!(effect, RiteEffect::Exorcise { .. });
        if vessel
            && binds
            && let Some(lord) = &mut self.lord
            && !lord.bound
        {
            lord.bound = true;
            events.push(Event::LordBound);
        }
    }

    /// Once per turn: the swarm thins in light and burns in fire, thickens in
    /// darkness; a planned leap lands; the Following comes up.
    pub(crate) fn tick_throne(&mut self, events: &mut Vec<Event>) {
        let now = self.turn();
        if let Some(mut lord) = self.lord {
            let court = self.kind("beelzebub");
            if lord.phase == Phase::Court
                && let Some((_, m)) = self.floor.monsters().find(|(_, m)| m.kind == court)
            {
                let at = m.pos;
                let before = lord.swarm;
                if self.floor.is_burning(at) {
                    lord.swarm = lord.swarm.saturating_sub(6);
                } else if self.floor.ambient_light(at).is_lit() {
                    lord.swarm = lord.swarm.saturating_sub(2);
                } else if !self.floor.light(at).is_lit() && now.is_multiple_of(4) {
                    // Only true darkness feeds it; even your candle holds it back.
                    lord.swarm = (lord.swarm + 1).min(SWARM_MAX);
                }
                if lord.swarm < before && self.floor.is_visible(at) {
                    events.push(Event::SwarmThins { left: lord.swarm });
                }
            }
            if let Some((at, when)) = lord.leap
                && now >= when
            {
                lord.leap = None;
                self.lord = Some(lord);
                if self.floor.corpse_at(at).is_some() && self.floor.monster_at(at).is_none() {
                    self.floor.corpses.retain(|c| c.at != at);
                    let kind = self.kind("beelzebub_host");
                    self.spawn_at(kind, at);
                    events.push(Event::LordPossesses { at });
                } else {
                    // The body is gone: he looks for another.
                    self.plan_leap(at, events);
                }
                lord = self.lord.expect("still set");
            }
            self.lord = Some(lord);
        }
        if let Stage::Ascent(_) = self.stage()
            && let Some(at) = self.following_at
        {
            if now + FOLLOWING_WARNING == at {
                events.push(Event::FollowingNear);
            }
            if now >= at {
                self.following_at = None;
                let kind = self.kind("the_following");
                let spot = self.free_near(self.floor.arrival());
                self.spawn_at(kind, spot);
                events.push(Event::FollowingArrives);
            }
        }
    }

    fn free_near(&self, at: Point) -> Point {
        std::iter::once(at)
            .chain(Direction::ALL.iter().map(|&d| at + d))
            .find(|&p| {
                self.map().is_walkable(p)
                    && self.floor.monster_at(p).is_none()
                    && p != self.player.pos
            })
            .unwrap_or(at)
    }

    fn spawn_at(&mut self, kind: KindId, at: Point) {
        let mut m = Monster::new(kind, self.content.monster(kind), at);
        m.mind = Mind::Hunting {
            last_seen: self.player.pos,
        };
        self.floor.monsters.insert(m);
    }

    /// Takes the Vigil Candle from the dais, if he is gone.
    pub(crate) fn take_relic(&mut self, events: &mut Vec<Event>) -> bool {
        if self.lord.is_some_and(|l| l.phase != Phase::Fallen) {
            events.push(Event::CandleGuarded);
            return false;
        }
        self.player.vigil = true;
        self.player.candle.light();
        events.push(Event::VigilTaken);
        true
    }

    /// Up the stair: only with the candle, and then on toward the church.
    pub(crate) fn ascend(&mut self) -> Vec<Event> {
        if self.map().tile(self.player.pos) != Tile::StairsUp {
            return vec![Event::NoStairsHere];
        }
        if !self.player.vigil {
            return vec![Event::StairsSealed {
                depth: self.depth(),
            }];
        }
        self.ascent += 1;
        let mut events = Vec::new();
        self.player.rite_state.borrowed = None;
        if self.stage() == Stage::Church {
            let (map, start) = church();
            self.floor = crate::floor::Floor::new(map, start);
            self.player.pos = start;
            self.following_at = None;
            events.push(Event::ReachedChurch);
        } else {
            self.floor = self.generate_ascent(self.ascent);
            self.player.pos = self.floor.arrival();
            let delay = FOLLOWING_DELAY[usize::from(self.ascent - 1)];
            self.following_at = Some(self.turn() + delay);
            events.push(Event::Ascended { floor: self.ascent });
        }
        self.pass_time(&mut events);
        events
    }

    /// An ascent floor: re-rolled, corrupted, with the stairs reversed: you
    /// come up through a stair down and the way on is a stair up.
    fn generate_ascent(&mut self, floor: u8) -> crate::floor::Floor {
        let like = [10, 7, 4, 2][usize::from(floor - 1)];
        let mut rng = crate::rng::floor_rng(self.seed() ^ 0xA5CE_17ED, floor);
        let layout = crate::map::generate::floor(&mut rng, like, true);
        let mut map = layout.map;
        let goal = map
            .find(Tile::StairsDown)
            .next()
            .expect("generated floors have a stair down");
        map.set(goal, Tile::StairsUp);
        map.set(layout.start, Tile::StairsDown);
        let mut spawns = crate::spawn::populate(&mut rng, self.content, &map, layout.start, like);
        spawns.truncate(spawns.len().div_ceil(2));
        let tallow = crate::spawn::place_tallow(&mut rng, &map, layout.start);
        let mut floor = crate::floor::Floor::new(map, layout.start);
        for (kind, pos) in spawns {
            floor
                .monsters
                .insert(Monster::new(kind, self.content.monster(kind), pos));
        }
        floor.tallow = tallow
            .into_iter()
            .map(|(at, amount)| crate::floor::Tallow::new(at, amount))
            .collect();
        floor
    }

    /// Starts on ascent floor `floor` (5 = the church) with the candle. For testing.
    pub fn dev_ascent(&mut self, floor: u8) {
        self.dev_skip_to(crate::MAX_DEPTH);
        if let Some(lord) = &mut self.lord {
            lord.phase = Phase::Fallen;
        }
        self.player.vigil = true;
        self.ascent = floor.clamp(1, ASCENT_FLOORS + 1) - 1;
        let stair = self
            .map()
            .find(Tile::StairsUp)
            .next()
            .expect("the throne has a stair up");
        self.player.pos = stair;
        self.ascend();
    }

    /// Walking into the altar with the candle wins the run.
    pub(crate) fn place_candle(&mut self) -> Vec<Event> {
        if !self.player.vigil {
            return vec![Event::AltarEmpty];
        }
        self.victory = Some(Victory {
            turn: self.turn(),
            exorcised: self.stats.exorcised,
            dread: self.player.dread.value(),
            rites: self.player.rites.len() as u32,
            level: self.player.level,
        });
        vec![Event::Won]
    }

    /// The cross a sweeping blow aimed at `target` covers.
    pub fn blow_area(&self, id: MonsterId, target: Point) -> Vec<Point> {
        let sweeps = self
            .floor
            .monsters
            .get(id)
            .is_some_and(|m| self.content.monster(m.kind).has(|t| *t == Trait::Sweeps));
        let mut area = vec![target];
        if sweeps {
            area.extend(
                [Direction::N, Direction::S, Direction::E, Direction::W]
                    .iter()
                    .map(|&d| target + d),
            );
        }
        area
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Command;
    use crate::map::path;

    /// A run standing at the bottom.
    fn at_the_throne() -> World {
        let mut world = World::new(3);
        world.dev_skip_to(crate::MAX_DEPTH);
        world
    }

    fn kind(world: &World, id: &str) -> KindId {
        world.content().kind_by_id(id).unwrap()
    }

    fn find(world: &World, id: &str) -> Option<MonsterId> {
        let k = kind(world, id);
        world
            .floor()
            .monsters()
            .find(|(_, m)| m.kind == k)
            .map(|(i, _)| i)
    }

    #[test]
    fn the_throne_room_is_whole_and_set() {
        let world = at_the_throne();
        let dist = path::distances(world.map(), world.player().pos);
        for p in world.map().points() {
            assert!(!world.map().is_walkable(p) || dist.at(p).is_some(), "{p:?}");
        }
        assert!(find(&world, "beelzebub").is_some());
        assert_eq!(world.floor().corpses().len(), THRONE_BODIES);
        let candle = world.content().item_by_id("vigil_candle").unwrap();
        assert!(world.floor().items().iter().any(|f| f.item.kind == candle));
        assert_eq!(world.lord().unwrap().phase, Phase::Court);
    }

    #[test]
    fn the_swarm_absorbs_blows_until_it_is_gone() {
        let mut world = at_the_throne();
        let id = find(&world, "beelzebub").unwrap();
        let mut events = Vec::new();
        world.damage_monster(id, 5, crate::progress::Source::Rite, &mut events);
        assert_eq!(world.floor().monster(id).unwrap().health, 24);
        assert!(events.contains(&Event::SwarmAbsorbs {
            left: SWARM_MAX - 2
        }));
        world.lord.as_mut().unwrap().swarm = 0;
        world.damage_monster(id, 5, crate::progress::Source::Rite, &mut events);
        assert_eq!(world.floor().monster(id).unwrap().health, 19);
    }

    #[test]
    fn three_phases_then_the_candle() {
        let mut world = at_the_throne();
        world.player.health = 9999;
        world.player.max_health = 9999;
        let court = find(&world, "beelzebub").unwrap();
        world.lord.as_mut().unwrap().swarm = 0;
        let mut events = Vec::new();
        world.damage_monster(court, 999, crate::progress::Source::Rite, &mut events);
        assert_eq!(world.lord().unwrap().phase, Phase::Possession);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::LordGathers { .. }))
        );

        // He leaps from body to body; each leap is announced, then lands.
        let mut hosts = 0;
        for _ in 0..200 {
            let events = world.apply(Command::Wait);
            if let Some(host) = find(&world, "beelzebub_host") {
                hosts += 1;
                let mut e = Vec::new();
                world.damage_monster(host, 999, crate::progress::Source::Rite, &mut e);
            }
            if world.lord().unwrap().phase == Phase::Lord {
                break;
            }
            let _ = events;
        }
        assert!(hosts >= 1, "he possessed at least one body");
        assert_eq!(
            world.lord().unwrap().phase,
            Phase::Lord,
            "out of bodies, he rises"
        );
        let lord = find(&world, "beelzebub_lord").unwrap();

        // The candle stays guarded until he falls.
        let candle = world.content().item_by_id("vigil_candle").unwrap();
        let dais = world
            .floor()
            .items()
            .iter()
            .find(|f| f.item.kind == candle)
            .unwrap()
            .at;
        world.player.pos = dais;
        assert!(world.apply(Command::PickUp).contains(&Event::CandleGuarded));
        let mut events = Vec::new();
        world.damage_monster(lord, 999, crate::progress::Source::Rite, &mut events);
        assert!(events.contains(&Event::LordFalls));
        assert!(world.apply(Command::PickUp).contains(&Event::VigilTaken));
        assert!(world.player().vigil);
    }

    #[test]
    fn binding_a_host_pins_him_so_its_death_ends_the_phase() {
        let mut world = at_the_throne();
        world.player.health = 9999;
        let court = find(&world, "beelzebub").unwrap();
        world.lord.as_mut().unwrap().swarm = 0;
        let mut events = Vec::new();
        world.damage_monster(court, 999, crate::progress::Source::Rite, &mut events);
        world.apply(Command::Wait);
        world.apply(Command::Wait);
        let host = find(&world, "beelzebub_host").expect("he took a body");
        let kneel = world.content().rite_by_id("kneel").unwrap();
        world.teach_rite(kneel);
        let at = world.floor().monster(host).unwrap().pos;
        let mut events = Vec::new();
        world.maybe_bind_lord(
            Some(host),
            School::Binding,
            world.content().rite(kneel).effect,
            &mut events,
        );
        assert!(events.contains(&Event::LordBound));
        let mut events = Vec::new();
        world.damage_monster(host, 999, crate::progress::Source::Rite, &mut events);
        assert!(events.contains(&Event::LordRises));
        let _ = at;
    }

    #[test]
    fn the_candle_burns_forever_and_opens_the_way_up() {
        let mut world = at_the_throne();
        world.lord.as_mut().unwrap().phase = Phase::Fallen;
        world.despawn_all();
        let mut events = Vec::new();
        world.take_relic(&mut events);
        let tallow = world.player().candle.tallow();
        for _ in 0..20 {
            world.apply(Command::Wait);
        }
        assert_eq!(world.player().candle.tallow(), tallow);
        world.player.pos = world.floor().arrival();
        let events = world.apply(Command::Ascend);
        assert!(events.contains(&Event::Ascended { floor: 1 }));
        assert_eq!(world.stage(), Stage::Ascent(1));
        assert_eq!(world.map().tile(world.player().pos), Tile::StairsDown);
        assert_eq!(world.map().find(Tile::StairsUp).count(), 1);
        assert_eq!(world.following_in(), Some(FOLLOWING_DELAY[0] - 1));
    }

    #[test]
    fn the_following_comes_cannot_die_and_chews_doors() {
        let mut world = at_the_throne();
        world.lord.as_mut().unwrap().phase = Phase::Fallen;
        world.despawn_all();
        let mut events = Vec::new();
        world.take_relic(&mut events);
        world.player.pos = world.floor().arrival();
        world.apply(Command::Ascend);
        world.despawn_all();
        world.player.health = 9999;
        let mut all = Vec::new();
        for _ in 0..FOLLOWING_DELAY[0] + 2 {
            all.extend(world.apply(Command::Wait));
        }
        assert!(all.contains(&Event::FollowingNear));
        assert!(all.contains(&Event::FollowingArrives));
        let id = find(&world, "the_following").unwrap();
        let mut events = Vec::new();
        world.damage_monster(id, 500, crate::progress::Source::Rite, &mut events);
        assert!(world.floor().monster(id).is_some());
        assert!(events.iter().any(|e| matches!(e, Event::Undying { .. })));
    }

    #[test]
    fn the_altar_takes_the_candle_and_the_run_is_won() {
        let mut world = at_the_throne();
        world.player.vigil = true;
        world.ascent = ASCENT_FLOORS;
        world.despawn_all();
        // From the last ascent floor, up into the church.
        let (map, start) = crate::map::prefab::parse("#####\n#@<.#\n#####").unwrap();
        world.floor = crate::floor::Floor::new(map, start);
        world.player.pos = Point::new(2, 1);
        let events = world.apply(Command::Ascend);
        assert!(events.contains(&Event::ReachedChurch));
        let altar = world.map().find(Tile::Altar).next().unwrap();
        world.player.pos = altar + Direction::W;
        let events = world.apply(Command::Move(Direction::E));
        assert_eq!(events, vec![Event::Won]);
        assert!(world.victory().is_some());
        assert!(world.apply(Command::Wait).is_empty(), "the run is over");
    }
}
