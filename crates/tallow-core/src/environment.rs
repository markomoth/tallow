//! The room is a weapon (BUILD_GUIDE.md §7): fire, oil, noise, doors,
//! braziers, holy ground.

use rand::RngExt;

use crate::content::{Faction, Trait};
use crate::events::{Cause, Event, Who};
use crate::geom::{Direction, Point};
use crate::map::{Tile, path};
use crate::monster::{Mind, MonsterId};
use crate::progress::Source;
use crate::world::World;

/// How far a pulled bell rope is heard, in steps.
pub const BELL_ROPE_NOISE: u32 = 18;
/// How far a handbell is heard.
pub const HANDBELL_NOISE: u32 = 12;
/// How far a fight is heard.
pub const FIGHT_NOISE: u32 = 5;
/// The Taken this close to a ringing bell cower instead of coming.
const BELL_FEAR_RANGE: u32 = 4;
const BELL_FEAR_ACTIONS: u32 = 4;
/// Damage a fire does each turn to whatever stands in it. The Swarm takes double.
pub const FIRE_DAMAGE: (u32, u32) = (2, 4);
/// Turns plain floor burns when a flask breaks on it.
const FLOOR_FUEL: u8 = 3;
/// Turns spilled oil burns.
const OIL_FUEL: u8 = 5;
/// Percent chance fire next door catches spilled oil.
const OIL_CATCH: u32 = 70;
/// Percent chance a step onto oil slips, losing an action.
pub const SLIP_CHANCE: u32 = 25;
/// Turns holy water keeps ground holy.
pub const HOLY_WATER_TURNS: u64 = 20;

/// What made a noise, for who answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Noise {
    /// Bells: the Taken close by cower.
    Bell,
    Fight,
}

impl World {
    /// Draws creatures within `radius` steps of `at` to come and look. Returns how many heard.
    pub(crate) fn make_noise(&mut self, at: Point, radius: u32, noise: Noise) -> u32 {
        let dist = path::distances(self.map(), at);
        let ids: Vec<MonsterId> = self.floor.monsters.keys().collect();
        let mut heard = 0;
        for id in ids {
            let m = &self.floor.monsters[id];
            let def = self.content.monster(m.kind);
            if m.phantom
                || m.compelled > 0
                || m.foe.is_some()
                || def.has(|t| *t == Trait::Relentless)
            {
                continue;
            }
            let Some(d) = dist.at(m.pos).filter(|&d| d <= radius) else {
                continue;
            };
            heard += 1;
            let m = &mut self.floor.monsters[id];
            if noise == Noise::Bell && def.faction == Faction::Taken && d <= BELL_FEAR_RANGE {
                m.terrified = m.terrified.max(BELL_FEAR_ACTIONS);
            } else {
                m.mind = Mind::Hunting { last_seen: at };
            }
        }
        heard
    }

    /// A fight makes noise, unless the rite of Hush is on you.
    pub(crate) fn fight_noise(&mut self) {
        if !self.hushed() {
            self.make_noise(self.player.pos, FIGHT_NOISE, Noise::Fight);
        }
    }

    pub(crate) fn ring_bell(&mut self, at: Point, radius: u32, events: &mut Vec<Event>) {
        self.make_noise(at, radius, Noise::Bell);
        events.push(Event::BellRung { at });
    }

    /// Sets a tile alight if anything there can burn. A flask's fire catches
    /// even bare floor, briefly. Returns whether it caught.
    pub(crate) fn ignite(&mut self, p: Point, flask: bool) -> bool {
        let tile = self.map().tile(p);
        let fuel = tile
            .fuel()
            .or(self.floor.has_oil(p).then_some(OIL_FUEL))
            .or((flask && tile.is_walkable() && !tile.is_water()).then_some(FLOOR_FUEL));
        let Some(fuel) = fuel else { return false };
        if self.floor.fire_at(p) >= fuel {
            return false;
        }
        self.floor.set_fire(p, fuel);
        true
    }

    /// Spills lamp oil on open ground around `at`. Fire already there lights it.
    pub(crate) fn spill_oil(&mut self, at: Point) {
        for p in std::iter::once(at).chain(Direction::ALL.iter().map(|&d| at + d)) {
            if self.map().is_walkable(p) && !self.map().tile(p).is_water() {
                self.floor.set_oil(p, true);
                if self.floor.is_burning(p) {
                    self.ignite(p, false);
                }
            }
        }
        self.floor.refresh_lights();
    }

    /// Fire burns down, spreads, and hurts what stands in it. Once per turn.
    pub(crate) fn tick_fire(&mut self, events: &mut Vec<Event>) {
        let burning: Vec<Point> = self.floor.burning().collect();
        if burning.is_empty() {
            return;
        }
        // Spread first, from what was burning at the start of the turn.
        for &p in &burning {
            for d in Direction::ALL {
                let q = p + d;
                if self.floor.is_burning(q) {
                    continue;
                }
                let tile = self.map().tile(q);
                let chance = if self.floor.has_oil(q) {
                    OIL_CATCH
                } else {
                    tile.catch_chance()
                };
                if chance > 0
                    && self.ai_rng.random_range(0..100) < chance
                    && self.ignite(q, false)
                    && self.floor.is_visible(q)
                    && tile == Tile::Bookshelf
                {
                    events.push(Event::FireSpread { at: q, tile });
                }
            }
        }
        // Burn down. What burned away is ash and open floor.
        for &p in &burning {
            let left = self.floor.fire_at(p) - 1;
            self.floor.set_fire(p, left);
            if left == 0 {
                self.floor.set_oil(p, false);
                if self.map().tile(p).fuel().is_some() {
                    self.floor.seals.retain(|&(at, _)| at != p);
                    self.floor.locks.retain(|&(at, _)| at != p);
                    self.floor.set_tile(p, Tile::Floor);
                }
            }
        }
        // Bodies burn.
        let now_burning: Vec<Point> = self.floor.burning().collect();
        self.floor.corpses.retain(|c| !now_burning.contains(&c.at));
        // Whatever stands in fire is burned.
        let in_fire: Vec<MonsterId> = self
            .floor
            .monsters
            .iter()
            .filter(|(_, m)| !m.phantom && now_burning.contains(&m.pos))
            .map(|(id, _)| id)
            .collect();
        for id in in_fire {
            let kind = self.floor.monsters[id].kind;
            let mut damage = crate::combat::roll_damage(&mut self.combat_rng, FIRE_DAMAGE);
            if self.content.monster(kind).faction == Faction::Swarm {
                damage *= 2;
            }
            if self.floor.is_visible(self.floor.monsters[id].pos) {
                events.push(Event::Burned {
                    who: Who::Monster(kind),
                    damage,
                });
            }
            self.damage_monster(id, damage, Source::Fire, events);
        }
        if now_burning.contains(&self.player.pos) {
            let damage = crate::combat::roll_damage(&mut self.combat_rng, FIRE_DAMAGE);
            events.push(Event::Burned {
                who: Who::Player,
                damage,
            });
            self.hurt_player(damage, Cause::Fire, events);
        }
        if now_burning.is_empty() {
            events.push(Event::FireOut);
        }
        self.floor.refresh_lights();
    }

    /// Seals lapse back into shut doors. Once per turn.
    pub(crate) fn tick_seals(&mut self, events: &mut Vec<Event>) {
        let now = self.turn();
        let lapsed: Vec<Point> = self
            .floor
            .seals
            .iter()
            .chain(self.floor.locks.iter())
            .filter(|&&(_, until)| now >= until)
            .map(|&(at, _)| at)
            .collect();
        for at in lapsed {
            self.floor.seals.retain(|&(p, _)| p != at);
            self.floor.locks.retain(|&(p, _)| p != at);
            if self.map().tile(at) == Tile::DoorSealed {
                self.floor.set_tile(at, Tile::DoorClosed);
                if self.floor.is_explored(at) {
                    events.push(Event::SealFaded { at });
                }
            }
        }
    }

    /// Shuts every open door beside you that nothing stands in.
    pub(crate) fn close_doors(&mut self) -> Vec<Event> {
        let here = self.player.pos;
        let doors: Vec<Point> = Direction::ALL
            .iter()
            .map(|&d| here + d)
            .filter(|&p| self.map().tile(p) == Tile::Door)
            .collect();
        if doors.is_empty() {
            return vec![Event::NoDoorToClose];
        }
        let free: Vec<Point> = doors
            .into_iter()
            .filter(|&p| {
                self.floor.monster_at(p).is_none()
                    && self.floor.items_at(p).next().is_none()
                    && self.floor.corpse_at(p).is_none()
                    && !self.floor.is_burning(p)
            })
            .collect();
        if free.is_empty() {
            return vec![Event::DoorBlocked];
        }
        let mut events = Vec::new();
        for p in free {
            self.floor.set_tile(p, Tile::DoorClosed);
            events.push(Event::DoorShut { at: p });
        }
        self.pass_time(&mut events);
        events
    }

    /// Walking into something that isn't open floor: doors, bell ropes,
    /// cold braziers. Returns `None` if it's just in the way.
    pub(crate) fn use_tile(&mut self, at: Point) -> Option<Vec<Event>> {
        let mut events = Vec::new();
        match self.map().tile(at) {
            Tile::DoorClosed => {
                self.floor.set_tile(at, Tile::Door);
                events.push(Event::DoorOpened { at });
            }
            Tile::DoorSealed if self.floor.locks.iter().any(|&(p, _)| p == at) => {
                return Some(vec![Event::DoorLocked]);
            }
            Tile::DoorSealed => {
                self.floor.seals.retain(|&(p, _)| p != at);
                self.floor.set_tile(at, Tile::Door);
                events.push(Event::SealBroken { at });
            }
            // The bell is heard all around whoever pulled the rope.
            Tile::Altar => return Some(self.place_candle()),
            Tile::BellRope => {
                let here = self.player.pos;
                self.make_noise(here, BELL_ROPE_NOISE, Noise::Bell);
                events.push(Event::BellRung { at });
            }
            Tile::ColdBrazier => {
                if !self.player.candle.is_lit() {
                    return Some(vec![Event::NeedFlame]);
                }
                self.floor.set_tile(at, Tile::Brazier);
                events.push(Event::BrazierLit { at });
            }
            _ => return None,
        }
        self.pass_time(&mut events);
        Some(events)
    }

    /// Steps onto oil may slip. True if this step slipped.
    pub(crate) fn slips(&mut self, at: Point) -> bool {
        self.floor.has_oil(at) && self.ai_rng.random_range(0..100) < SLIP_CHANCE
    }

    /// Holy water leaves the ground around where it breaks holy for a while.
    pub(crate) fn consecrate(&mut self, at: Point, events: &mut Vec<Event>) {
        let until = self.turn() + HOLY_WATER_TURNS;
        for p in std::iter::once(at).chain(Direction::ALL.iter().map(|&d| at + d)) {
            if self.map().is_walkable(p) {
                self.floor.sanctify(p, until);
            }
        }
        events.push(Event::Consecrated { at });
    }

    /// Whether `a` would attack `b` on sight: faction hatred, or a turncoat
    /// and its old kin.
    pub(crate) fn hostile(&self, a: MonsterId, b: MonsterId) -> bool {
        let (Some(ma), Some(mb)) = (self.floor.monsters.get(a), self.floor.monsters.get(b)) else {
            return false;
        };
        if a == b || ma.phantom || mb.phantom || mb.compelled > 0 || ma.compelled > 0 {
            return false;
        }
        let (da, db) = (self.content.monster(ma.kind), self.content.monster(mb.kind));
        // The great ones below only want you.
        if da.boss || db.boss {
            return false;
        }
        let (fa, fb) = (da.faction, db.faction);
        fa.hates(fb) || (fa == fb && ma.turned != mb.turned)
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

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn wait(world: &mut World, turns: u32) -> Vec<Event> {
        (0..turns)
            .flat_map(|_| world.apply(Command::Wait))
            .collect()
    }

    #[test]
    fn closed_doors_block_sight_and_open_when_walked_into() {
        let mut world = world_from("#########\n#@D.....#\n#########");
        assert!(!world.floor().is_visible(Point::new(4, 1)));
        let events = world.apply(Command::Move(Direction::E));
        assert!(events.contains(&Event::DoorOpened {
            at: Point::new(2, 1)
        }));
        assert_eq!(world.player().pos, Point::new(1, 1), "opening isn't moving");
        assert!(world.floor().is_visible(Point::new(4, 1)));
        world.apply(Command::CloseDoor);
        assert_eq!(world.map().tile(Point::new(2, 1)), Tile::DoorClosed);
    }

    #[test]
    fn swarms_and_nightmares_cant_open_doors_but_the_taken_can() {
        let mut world = world_from("##########\n#@.D.....#\n##########");
        let rat = world.spawn_monster(kind("gnawer"), Point::new(7, 1));
        world.spawn_monster(kind("gnawer"), Point::new(8, 1));
        world.floor.monsters[rat].mind = Mind::Hunting {
            last_seen: Point::new(1, 1),
        };
        wait(&mut world, 15);
        assert_eq!(world.map().tile(Point::new(3, 1)), Tile::DoorClosed);
        let mut world = world_from("##########\n#@.D.....#\n##########");
        let man = world.spawn_monster(kind("parishioner"), Point::new(7, 1));
        world.floor.monsters[man].mind = Mind::Hunting {
            last_seen: Point::new(1, 1),
        };
        wait(&mut world, 15);
        assert_eq!(world.map().tile(Point::new(3, 1)), Tile::Door);
    }

    #[test]
    fn a_bell_draws_creatures_to_it() {
        let mut world = world_from(
            "##############################\n\
             #@...........................#\n\
             ###########|##################",
        );
        let eater = world.spawn_monster(kind("lantern_eater"), Point::new(27, 1));
        world.player.pos = Point::new(10, 1);
        let events = world.apply(Command::Move(Direction::SE));
        assert!(events.contains(&Event::BellRung {
            at: Point::new(11, 2)
        }));
        assert_eq!(world.player().pos, Point::new(10, 1));
        assert!(matches!(
            world.floor().monster(eater).unwrap().mind,
            Mind::Hunting { .. }
        ));
    }

    #[test]
    fn rival_factions_fight_when_they_meet() {
        let mut world = world_from(
            "##############################\n\
             #@...........................#\n\
             ##############################",
        );
        world.apply(Command::ToggleCandle);
        let eater = world.spawn_monster(kind("lantern_eater"), Point::new(20, 1));
        let rat = world.spawn_monster(kind("gnawer"), Point::new(23, 1));
        world.spawn_monster(kind("gnawer"), Point::new(24, 1));
        assert!(world.hostile(eater, rat));
        assert!(!world.hostile(rat, eater), "vermin don't hate the dark");
        world.floor.monsters[eater].mind = Mind::Hunting {
            last_seen: Point::new(23, 1),
        };
        world.player.health = 999;
        for _ in 0..40 {
            world.apply(Command::Wait);
            if world.floor().monster(rat).is_none() {
                break;
            }
        }
        assert!(
            world.floor().monster(rat).is_none() || world.floor().monster(eater).is_none(),
            "one of them died"
        );
    }

    #[test]
    fn fire_spreads_along_shelves_and_burns_them_away() {
        let mut world = world_from(
            "##########\n\
             #@.......#\n\
             #..BBBB..#\n\
             #........#\n\
             ##########",
        );
        world.ignite(Point::new(3, 2), false);
        world.floor.refresh_lights();
        let events = wait(&mut world, 40);
        assert!(events.iter().any(|e| matches!(e, Event::FireSpread { .. })));
        assert!(events.contains(&Event::FireOut));
        for x in 3..=6 {
            assert_eq!(world.map().tile(Point::new(x, 2)), Tile::Floor, "x {x}");
        }
    }

    #[test]
    fn fire_hurts_what_stands_in_it_and_doubly_hurts_the_swarm() {
        let mut world = world_from("#######\n#@....#\n#######");
        let flies = world.spawn_monster(kind("fly_swarm"), Point::new(4, 1));
        world.floor.monsters[flies].pinned = 99;
        world.ignite(Point::new(4, 1), true);
        let events = world.apply(Command::Wait);
        let burned = events.iter().find_map(|e| match e {
            Event::Burned {
                who: Who::Monster(_),
                damage,
            } => Some(*damage),
            _ => None,
        });
        assert!(burned.is_some_and(|d| d >= 4), "{events:?}");
    }

    #[test]
    fn creatures_wont_walk_into_fire() {
        let mut world = world_from("#########\n#@......#\n#########");
        for x in 3..=4 {
            world.ignite(Point::new(x, 1), true);
            world.floor.set_fire(Point::new(x, 1), 50);
        }
        let man = world.spawn_monster(kind("parishioner"), Point::new(7, 1));
        world.floor.monsters[man].mind = Mind::Hunting {
            last_seen: Point::new(1, 1),
        };
        wait(&mut world, 15);
        assert!(world.floor().monster(man).unwrap().pos.x >= 5);
    }

    #[test]
    fn a_cold_brazier_lights_from_your_candle() {
        let mut world = world_from("#######\n#@o...#\n#######");
        assert!(!world.floor().ambient_light(Point::new(4, 1)).is_lit());
        let events = world.apply(Command::Move(Direction::E));
        assert!(events.contains(&Event::BrazierLit {
            at: Point::new(2, 1)
        }));
        assert!(world.floor().ambient_light(Point::new(4, 1)).is_lit());
    }

    #[test]
    fn the_taken_cower_from_a_close_bell() {
        let mut world = world_from("#########\n#@......#\n####|####");
        let man = world.spawn_monster(kind("parishioner"), Point::new(5, 1));
        world.player.pos = Point::new(3, 1);
        world.apply(Command::Move(Direction::SE));
        assert!(world.floor().monster(man).unwrap().terrified > 0);
    }

    fn give(world: &mut World, id: &str) -> crate::item::ItemId {
        let kind = Content::bundled().item_by_id(id).unwrap();
        world.place_item(world.player().pos, kind, 1);
        world.apply(Command::PickUp);
        world
            .player()
            .inventory
            .iter()
            .find(|it| it.kind == kind)
            .unwrap()
            .id
    }

    #[test]
    fn a_fire_flask_sets_a_library_alight() {
        let mut world = world_from(
            "############\n\
             #@.........#\n\
             #....BBBB..#\n\
             #..........#\n\
             ############",
        );
        let flask = give(&mut world, "fire_flask");
        let events = world.apply(Command::Throw {
            item: flask,
            target: Point::new(5, 3),
        });
        assert!(
            events.iter().any(|e| matches!(e, Event::Ignited { .. })),
            "{events:?}"
        );
        assert!(
            world.floor().is_burning(Point::new(5, 2)),
            "the shelf above caught"
        );
        assert!(
            world.floor().ambient_light(Point::new(6, 3)).is_lit(),
            "fire gives light"
        );
    }

    #[test]
    fn a_thrown_handbell_rings_where_it_lands() {
        let mut world = world_from(
            "##############################\n\
             #@...........................#\n\
             ##############################",
        );
        let eater = world.spawn_monster(kind("lantern_eater"), Point::new(18, 1));
        let bell = give(&mut world, "handbell");
        let events = world.apply(Command::Throw {
            item: bell,
            target: Point::new(7, 1),
        });
        assert!(events.contains(&Event::BellRung {
            at: Point::new(7, 1)
        }));
        assert_eq!(
            world.floor().monster(eater).unwrap().mind,
            Mind::Hunting {
                last_seen: Point::new(7, 1)
            }
        );
    }

    #[test]
    fn walking_into_fire_needs_a_second_step() {
        let mut world = world_from("#######\n#@....#\n#######");
        world.ignite(Point::new(2, 1), true);
        let events = world.apply(Command::Move(Direction::E));
        assert_eq!(
            events,
            vec![Event::FireAhead {
                at: Point::new(2, 1)
            }]
        );
        assert_eq!(world.turn(), 0);
        world.apply(Command::Move(Direction::E));
        assert_eq!(world.player().pos, Point::new(2, 1));
    }

    #[test]
    fn holy_water_consecrates_where_it_breaks() {
        let mut world = world_from("#########\n#@......#\n#########");
        let water = give(&mut world, "holy_water");
        world.apply(Command::Throw {
            item: water,
            target: Point::new(5, 1),
        });
        assert!(world.floor().is_sanctified(Point::new(5, 1)));
    }

    #[test]
    fn a_flask_with_nowhere_to_fly_stays_in_your_hand() {
        let mut world = world_from("#####\n#@B.#\n#####");
        let flask = give(&mut world, "fire_flask");
        let events = world.apply(Command::Throw {
            item: flask,
            target: Point::new(3, 1),
        });
        assert_eq!(events, vec![Event::BadTarget]);
        assert!(world.inventory_item(flask).is_some());
        assert!(!world.floor().is_burning(Point::new(1, 1)));
    }

    #[test]
    fn a_fire_flask_never_lights_your_own_tile() {
        let mut world = world_from("######\n#@...#\n######");
        let flask = give(&mut world, "fire_flask");
        world.apply(Command::Throw {
            item: flask,
            target: Point::new(2, 1),
        });
        assert!(world.floor().is_burning(Point::new(2, 1)));
        assert!(!world.floor().is_burning(Point::new(1, 1)));
    }

    #[test]
    fn deep_water_warns_then_drowns_your_candle() {
        let mut world = world_from("#######\n#@WW..#\n#######");
        let events = world.apply(Command::Move(Direction::E));
        assert_eq!(
            events,
            vec![Event::DeepWaterAhead {
                at: Point::new(2, 1)
            }]
        );
        let events = world.apply(Command::Move(Direction::E));
        assert!(events.contains(&Event::CandleDrowned));
        assert!(!world.player().candle.is_lit());
        assert_eq!(world.turn(), 2, "wading costs double");
        assert_eq!(
            world.apply(Command::ToggleCandle),
            vec![Event::CantLightInWater]
        );
        let events = world.apply(Command::Move(Direction::E));
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::DeepWaterAhead { .. })),
            "no second warning"
        );
    }

    #[test]
    fn rotten_boards_warn_then_drop_you_a_floor() {
        let mut world = World::new(5);
        world.despawn_all();
        let here = world.player().pos;
        let next = Direction::ALL
            .into_iter()
            .map(|d| here + d)
            .find(|&p| world.map().tile(p) == Tile::Floor)
            .unwrap();
        world.floor.set_tile(next, Tile::RottenFloor);
        let dir = Direction::ALL
            .into_iter()
            .find(|&d| here + d == next)
            .unwrap();
        assert!(matches!(
            world.apply(Command::Move(dir))[..],
            [Event::RottenAhead { .. }]
        ));
        let health = world.player().health;
        let events = world.apply(Command::Move(dir));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Fell { depth: 2, .. }))
        );
        assert_eq!(world.depth(), 2);
        assert!(world.player().health < health && world.player().health > 0);
    }
}
