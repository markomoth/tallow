//! Creatures' special tricks (BUILD_GUIDE.md §9): chants, songs, drags,
//! bursts, summoning, raising the dead, rewriting rooms, choruses.
//! Every trick that can hurt badly is announced a turn ahead.

use rand::RngExt;

use crate::content::Trait;
use crate::events::{Cause, Event};
use crate::geom::{Direction, Point};
use crate::map::{Tile, path};
use crate::monster::{Mind, Monster, MonsterId};
use crate::world::World;

/// How long the Provost's locks hold.
const LOCK_TURNS: u64 = 15;
/// How far the Provost's word reaches.
const REWRITE_RANGE: i32 = 8;
/// One in this many actions, a singer's song is mentioned in the log.
const SONG_NOTE_ODDS: u32 = 8;

impl World {
    /// Runs a creature's special trick if it has one ready. Returns true if
    /// that used up its action.
    pub(crate) fn use_ability(
        &mut self,
        id: MonsterId,
        sees: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        let m = self.floor.monsters[id].clone();
        let def = self.content.monster(m.kind);
        let kind = m.kind;

        // Songs cost no action: they simply go on while it can see you.
        if let Some(dread) = def.traits.iter().find_map(|t| match t {
            Trait::Sings { dread } => Some(*dread),
            _ => None,
        }) && self.floor.in_sight(m.pos)
            && m.pos.chebyshev(self.player.pos) <= def.sight
        {
            self.shift_dread(dread as i32, events);
            self.witness(kind, Trait::Sings { dread });
            if self.floor.is_visible(m.pos) && self.ai_rng.random_ratio(1, SONG_NOTE_ODDS) {
                events.push(Event::Singing { kind });
            }
        }

        // Snuffing costs no action either: it comes close, and your light dies.
        let snuff = &mut self.floor.monsters[id].snuff_cooldown;
        *snuff = snuff.saturating_sub(1);
        if let Some((range, cooldown)) = def.traits.iter().find_map(|t| match *t {
            Trait::Snuffs { range, cooldown } => Some((range, cooldown)),
            _ => None,
        }) && m.snuff_cooldown == 0
            && self.player.candle.is_lit()
            && !self.player.vigil
            && self.floor.in_sight(m.pos)
            && m.pos.chebyshev(self.player.pos) <= range
        {
            self.player.candle.snuff();
            self.floor.monsters[id].snuff_cooldown = cooldown;
            events.push(Event::CandleSnuffedBy { kind });
            self.witness(kind, Trait::Snuffs { range, cooldown });
            self.wake_leavings(crate::leavings::Wake::EnteringDarkness, events);
        }

        if m.chanting {
            let monster = &mut self.floor.monsters[id];
            monster.chanting = false;
            let Some((damage, cooldown)) = def.traits.iter().find_map(|t| match *t {
                Trait::Chants { damage, cooldown } => Some((damage, cooldown)),
                _ => None,
            }) else {
                return false;
            };
            monster.ability_cooldown = cooldown;
            if self.floor.in_sight(m.pos) {
                let damage = crate::combat::roll_damage(&mut self.combat_rng, damage);
                events.push(Event::ChantLands { kind, damage });
                self.shift_dread(300, events);
                self.hurt_player(damage, Cause::Chant(kind), events);
            } else {
                events.push(Event::ChantBroken { kind });
            }
            return true;
        }

        let monster = &mut self.floor.monsters[id];
        monster.ability_cooldown = monster.ability_cooldown.saturating_sub(1);
        if m.ability_cooldown > 0 {
            return false;
        }
        for t in def.traits.clone() {
            match t {
                Trait::Chants { .. }
                    if sees
                        && self.floor.is_visible(m.pos)
                        && m.pos.chebyshev(self.player.pos) > 1 =>
                {
                    self.floor.monsters[id].chanting = true;
                    events.push(Event::ChantBegins { kind });
                    self.witness(kind, t);
                    return true;
                }
                Trait::Summons { cooldown, max } if !self.floor.light(m.pos).is_lit() || !sees => {
                    let Some(into) = def.spawn else { continue };
                    let alive = self
                        .floor
                        .monsters()
                        .filter(|(_, o)| o.kind == into)
                        .count();
                    if alive >= max as usize || !matches!(m.mind, Mind::Hunting { .. }) {
                        continue;
                    }
                    if let Some(at) = self.free_beside(m.pos) {
                        self.spawn_hunter(into, at);
                        self.floor.monsters[id].ability_cooldown = cooldown;
                        if self.floor.is_visible(m.pos) {
                            events.push(Event::Summoned { kind, into });
                            self.witness(kind, t);
                        }
                        return true;
                    }
                }
                Trait::Doubles { cooldown, max } if sees && self.floor.is_visible(m.pos) => {
                    let copies = self
                        .floor
                        .monsters()
                        .filter(|(_, o)| o.kind == kind && o.phantom)
                        .count();
                    if copies >= max as usize {
                        continue;
                    }
                    if let Some(at) = self.free_beside(m.pos) {
                        self.spawn_phantom_at(kind, at);
                        self.floor.monsters[id].ability_cooldown = cooldown;
                        events.push(Event::Doubled { kind });
                        self.witness(kind, t);
                        return true;
                    }
                }
                Trait::Raises { range, cooldown } => {
                    let Some(into) = def.spawn else { continue };
                    let body = self
                        .floor
                        .corpses
                        .iter()
                        .filter(|c| {
                            c.at.chebyshev(m.pos) <= range
                                && self.floor.monster_at(c.at).is_none()
                                && c.at != self.player.pos
                        })
                        .min_by_key(|c| c.at.distance_squared(m.pos))
                        .copied();
                    let Some(body) = body else { continue };
                    self.floor.corpses.retain(|c| c.at != body.at);
                    self.spawn_hunter(into, body.at);
                    self.floor.monsters[id].ability_cooldown = cooldown;
                    if self.floor.is_visible(body.at) || self.floor.is_visible(m.pos) {
                        events.push(Event::Raised {
                            kind,
                            body: body.kind,
                            into,
                        });
                        self.witness(kind, t);
                    }
                    return true;
                }
                Trait::Rewrites { cooldown } if sees => {
                    self.floor.monsters[id].ability_cooldown = cooldown;
                    self.witness(kind, t);
                    // Alternate: lock the doors, then set the books alight.
                    let doors: Vec<Point> = self
                        .map()
                        .points()
                        .filter(|&p| {
                            matches!(self.map().tile(p), Tile::Door | Tile::DoorClosed)
                                && p.chebyshev(m.pos) <= REWRITE_RANGE
                                && self.floor.monster_at(p).is_none()
                                && p != self.player.pos
                        })
                        .collect();
                    let shelves: Vec<Point> = self
                        .map()
                        .points()
                        .filter(|&p| {
                            self.map().tile(p) == Tile::Bookshelf
                                && p.chebyshev(m.pos) <= REWRITE_RANGE
                                && !self.floor.is_burning(p)
                        })
                        .collect();
                    let lock =
                        !doors.is_empty() && (shelves.is_empty() || self.ai_rng.random_bool(0.5));
                    if lock {
                        let until = self.turn() + LOCK_TURNS;
                        for p in doors {
                            self.floor.locks.retain(|&(q, _)| q != p);
                            self.floor.locks.push((p, until));
                            self.floor.set_tile(p, Tile::DoorSealed);
                        }
                        events.push(Event::DoorsLocked { kind });
                        return true;
                    }
                    let index = self.ai_rng.random_range(0..shelves.len().max(1));
                    if let Some(&p) = shelves.get(index) {
                        self.ignite(p, false);
                        self.floor.refresh_lights();
                        events.push(Event::BooksIgnited { kind });
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// A struck creature's tricks: its hit blinds, or drags you to the water.
    pub(crate) fn on_hit_player(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        let kind = self.floor.monsters[id].kind;
        let def = self.content.monster(kind);
        for t in def.traits.clone() {
            match t {
                Trait::Blinds { turns } => {
                    let until = self.turn() + u64::from(turns);
                    self.player.blind_until =
                        Some(self.player.blind_until.map_or(until, |u| u.max(until)));
                    events.push(Event::Blinded { kind });
                    self.witness(kind, t);
                }
                Trait::Grabs { turns } => self.grabbed(id, turns, events),
                Trait::Feeds { dread } => {
                    self.shift_dread(dread as i32 * 100, events);
                    events.push(Event::FedOnFear { kind, dread });
                    self.witness(kind, t);
                }
                Trait::Drags => {
                    if let Some(step) = self.step_toward_deep_water() {
                        self.player.pos = step;
                        events.push(Event::Dragged { kind });
                        self.witness(kind, t);
                        if self.map().tile(step) == Tile::DeepWater
                            && self.player.candle.is_lit()
                            && !self.player.vigil
                        {
                            self.player.candle.snuff();
                            events.push(Event::CandleDrowned);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// One step from you toward the nearest deep water within 8 steps, if free.
    fn step_toward_deep_water(&self) -> Option<Point> {
        let here = self.player.pos;
        let water = self
            .map()
            .find(Tile::DeepWater)
            .filter(|p| p.chebyshev(here) <= 8)
            .min_by_key(|p| p.distance_squared(here))?;
        let dist = path::distances(self.map(), water);
        let now = dist.at(here)?;
        Direction::ALL
            .iter()
            .map(|&d| here + d)
            .filter(|&p| self.floor.monster_at(p).is_none() && dist.at(p).is_some_and(|n| n < now))
            .min_by_key(|&p| dist.at(p))
    }

    /// After a creature dies: choruses die together, bloated things burst, bosses are announced.
    pub(crate) fn on_monster_death(
        &mut self,
        kind: crate::content::KindId,
        at: Point,
        events: &mut Vec<Event>,
    ) {
        let def = self.content.monster(kind);
        if def.has(|t| *t == Trait::Bursts)
            && let Some(into) = def.spawn
        {
            let spot = if self.floor.monster_at(at).is_none() && at != self.player.pos {
                Some(at)
            } else {
                self.free_beside(at)
            };
            if let Some(spot) = spot {
                self.spawn_hunter(into, spot);
                if self.floor.is_visible(at) {
                    events.push(Event::Burst { kind, into });
                    self.witness(kind, Trait::Bursts);
                }
            }
        }
        self.throne_death(kind, at, events);
        let last_of_kind = !self.floor.monsters().any(|(_, m)| m.kind == kind);
        if def.boss && last_of_kind {
            self.defeated.insert(kind);
        }
        // The mini-bosses; Beelzebub's forms announce themselves.
        if def.boss && last_of_kind && def.boss_floor.is_some() {
            events.push(Event::BossDefeated { kind });
            let named = self
                .content
                .leavings
                .named
                .iter()
                .find(|n| n.from.as_deref() == Some(def.id.as_str()));
            if let Some(named) = named {
                let leaving = crate::leavings::Leaving::from_named(named);
                let id = self.register_leaving(leaving);
                self.floor.leavings.push((at, id));
            }
        }
    }

    /// A chorus shares its wounds: every other body of the kind takes the same.
    pub(crate) fn chorus_share(&mut self, id: MonsterId, health: u32) -> Vec<MonsterId> {
        let kind = self.floor.monsters[id].kind;
        if !self.content.monster(kind).has(|t| *t == Trait::Chorus) {
            return Vec::new();
        }
        let others: Vec<MonsterId> = self
            .floor
            .monsters
            .iter()
            .filter(|&(o, m)| o != id && m.kind == kind && !m.phantom)
            .map(|(o, _)| o)
            .collect();
        for &o in &others {
            self.floor.monsters[o].health = health;
        }
        others
    }

    fn free_beside(&self, at: Point) -> Option<Point> {
        Direction::ALL.iter().map(|&d| at + d).find(|&p| {
            self.map().tile(p) == Tile::Floor
                && self.floor.monster_at(p).is_none()
                && p != self.player.pos
        })
    }

    /// Puts a creature on the floor already hunting you.
    fn spawn_hunter(&mut self, kind: crate::content::KindId, at: Point) {
        let mut monster = Monster::new(kind, self.content.monster(kind), at);
        monster.mind = Mind::Hunting {
            last_seen: self.player.pos,
        };
        self.floor.monsters.insert(monster);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Command;
    use crate::content::{Content, KindId};
    use crate::map::prefab;

    fn world_from(text: &str) -> World {
        let (map, start) = prefab::parse(text).unwrap();
        World::from_map(map, start)
    }

    fn hall() -> World {
        world_from(
            "##############################\n\
             #@...........................#\n\
             #............................#\n\
             #............................#\n\
             ##############################",
        )
    }

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn wait(world: &mut World, turns: u32) -> Vec<Event> {
        (0..turns)
            .flat_map(|_| world.apply(Command::Wait))
            .collect()
    }

    #[test]
    fn a_chant_is_announced_and_can_be_broken() {
        let mut world = world_from(
            "##########\n\
             #@.......#\n\
             #.########\n\
             #.########\n\
             ##########",
        );
        let scholar = world.spawn_monster(kind("feverish_scholar"), Point::new(6, 1));
        world.floor.monsters[scholar].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let mut events = Vec::new();
        for _ in 0..10 {
            events = world.apply(Command::Wait);
            if events
                .iter()
                .any(|e| matches!(e, Event::ChantBegins { .. }))
            {
                break;
            }
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::ChantBegins { .. }))
        );
        world.floor.monsters[scholar].pinned = 5;
        let events = world.apply(Command::Move(Direction::S));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::ChantBroken { .. })),
            "{events:?}"
        );
    }

    #[test]
    fn the_choir_shares_its_wounds_and_dies_together() {
        let mut world = hall();
        let a = world.spawn_monster(kind("drowned_choir"), Point::new(3, 2));
        let b = world.spawn_monster(kind("drowned_choir"), Point::new(8, 2));
        let mut events = Vec::new();
        world.damage_monster(a, 10, crate::progress::Source::Rite, &mut events);
        assert_eq!(world.floor().monster(b).unwrap().health, 30);
        world.damage_monster(b, 99, crate::progress::Source::Rite, &mut events);
        assert!(world.floor().monster(a).is_none());
        assert!(events.contains(&Event::BossDefeated {
            kind: kind("drowned_choir")
        }));
    }

    #[test]
    fn a_bloatfly_bursts_into_flies() {
        let mut world = hall();
        let fly = world.spawn_monster(kind("bloatfly"), Point::new(3, 2));
        let mut events = Vec::new();
        world.damage_monster(fly, 99, crate::progress::Source::Rite, &mut events);
        assert!(events.iter().any(|e| matches!(e, Event::Burst { .. })));
        assert!(
            world
                .floor()
                .monsters()
                .any(|(_, m)| m.kind == kind("fly_swarm"))
        );
    }

    #[test]
    fn the_sexton_raises_the_dead() {
        let mut world = hall();
        world.leave_corpse(kind("parishioner"), Point::new(6, 2), &mut Vec::new());
        let sexton = world.spawn_monster(kind("sexton"), Point::new(9, 2));
        world.floor.monsters[sexton].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        world.player.health = 999;
        let events = wait(&mut world, 6);
        assert!(
            events.iter().any(|e| matches!(e, Event::Raised { .. })),
            "{events:?}"
        );
        assert!(
            world
                .floor()
                .monsters()
                .any(|(_, m)| m.kind == kind("risen_husk"))
        );
    }

    #[test]
    fn an_inkling_blinds_and_sight_returns() {
        let mut world = hall();
        let ink = world.spawn_monster(kind("inkling"), Point::new(2, 1));
        world.player.health = 999;
        let mut blinded = false;
        for _ in 0..30 {
            let events = world.apply(Command::Wait);
            if events.iter().any(|e| matches!(e, Event::Blinded { .. })) {
                blinded = true;
                break;
            }
        }
        assert!(blinded);
        assert!(!world.floor().is_visible(Point::new(6, 1)), "light shrank");
        world.floor.monsters.remove(ink);
        let events = wait(&mut world, 14);
        assert!(events.contains(&Event::SightReturns));
    }

    #[test]
    fn a_deacon_drags_you_toward_deep_water() {
        let mut world = world_from(
            "##########\n\
             #@...WWWW#\n\
             #........#\n\
             ##########",
        );
        world.spawn_monster(kind("drowned_deacon"), Point::new(1, 2));
        world.player.health = 999;
        let start = world.player().pos;
        let mut dragged = false;
        for _ in 0..40 {
            if world
                .apply(Command::Wait)
                .iter()
                .any(|e| matches!(e, Event::Dragged { .. }))
            {
                dragged = true;
                break;
            }
        }
        assert!(dragged);
        assert_ne!(world.player().pos, start);
    }

    #[test]
    fn the_provost_locks_doors_you_cannot_open() {
        let mut world = world_from(
            "############\n\
             #@.........#\n\
             #####+######\n\
             #..........#\n\
             ############",
        );
        let provost = world.spawn_monster(kind("provost"), Point::new(8, 1));
        world.floor.monsters[provost].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        world.floor.monsters[provost].pinned = 99;
        world.player.health = 999;
        let events = wait(&mut world, 3);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::DoorsLocked { .. })),
            "{events:?}"
        );
        world.player.pos = Point::new(5, 1);
        assert_eq!(
            world.apply(Command::Move(Direction::S)),
            vec![Event::DoorLocked]
        );
    }
}
