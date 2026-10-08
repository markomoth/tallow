//! How creatures fight beyond walking up and hitting (BUILD_GUIDE.md §9, M16).
//!
//! Each faction fights its own way, light and dark change them, and some
//! kinds have a move of their own: a leap, a grab, a volley, a retreat. Every
//! number here also shows in Look.

use crate::combat;
use crate::content::{Faction, Trait};
use crate::events::{Cause, Event};
use crate::geom::{Direction, Point};
use crate::map::Tile;
use crate::monster::{Mind, Monster, MonsterId};
use crate::world::World;

/// Each other swarm creature beside you makes a swarm creature this much surer...
pub const FLANK_ACCURACY: i32 = 10;
/// ...counting this many at most.
pub const MAX_FLANKERS: u32 = 2;
/// The Taken remember being people: in your candlelight they hold back...
pub const TAKEN_SHY_ACCURACY: i32 = 10;
/// ...and in the dark they forget themselves.
pub const TAKEN_DARK_DAMAGE: u32 = 1;
/// Swarms quicken in the dark.
pub const SWARM_DARK_SPEED: u32 = 3;
/// Light holds the Dreaming: they slow in it.
pub const DREAMING_LIT_SLOW: u32 = 2;
/// The Remnant keep to their posts: they won't follow you farther than this.
pub const GUARD_LEASH: i32 = 6;
/// A creature that mends turns to run below a third of its health, and turns
/// back once it has mended to two thirds.
const MEND_RUN_BELOW: u32 = 3;
/// It mends 1 an action once out of your sight or this far off.
const MEND_DISTANCE: i32 = 6;
/// Actions between leaps.
const LUNGE_COOLDOWN: u32 = 4;
/// Actions between volleys.
const VOLLEY_COOLDOWN: u32 = 3;
/// Guarding: your defense rises this much until your next action.
pub const GUARD_DEFENSE: i32 = 25;

impl World {
    /// How fast a creature is where it stands: swarms quicken in the dark, the
    /// Dreaming slow in the light.
    pub fn monster_speed(&self, m: &Monster) -> i32 {
        let def = self.content.monster(m.kind);
        let speed = match def.faction {
            Faction::Swarm if self.in_the_dark(m.pos) => def.speed + SWARM_DARK_SPEED,
            Faction::Dreaming if !self.in_the_dark(m.pos) => {
                def.speed.saturating_sub(DREAMING_LIT_SLOW)
            }
            _ => def.speed,
        };
        speed as i32
    }

    /// Other swarm creatures beside you, besides this one (at most `MAX_FLANKERS`).
    pub fn flankers(&self, m: &Monster) -> u32 {
        if self.content.monster(m.kind).faction != Faction::Swarm
            || m.pos.chebyshev(self.player.pos) != 1
        {
            return 0;
        }
        let here = self.player.pos;
        let count = self
            .floor
            .monsters()
            .filter(|(_, o)| {
                !o.phantom
                    && o.pos != m.pos
                    && o.compelled == 0
                    && o.pos.chebyshev(here) == 1
                    && self.content.monster(o.kind).faction == Faction::Swarm
            })
            .count() as u32;
        count.min(MAX_FLANKERS)
    }

    /// One of the Taken standing in light: it holds back.
    pub fn shy(&self, m: &Monster) -> bool {
        self.content.monster(m.kind).faction == Faction::Taken && !self.in_the_dark(m.pos)
    }

    /// One of the Remnant too far from its post to follow you any farther.
    pub(crate) fn past_leash(&self, m: &Monster) -> bool {
        let def = self.content.monster(m.kind);
        def.faction == Faction::Remnant
            && !def.boss
            && !def.has(|t| *t == Trait::Relentless)
            && m.compelled == 0
            && self.player.pos.chebyshev(m.post) > GUARD_LEASH
    }

    /// A creature that scatters from candlelight is standing in it, close to you.
    fn fears_light(&self, m: &Monster) -> bool {
        self.content
            .monster(m.kind)
            .has(|t| *t == Trait::FearsLight)
            && self.player.candle.is_lit()
            && !self.in_the_dark(m.pos)
            && m.pos.chebyshev(self.player.pos) <= 4
    }

    /// A hunting creature's own way of fighting. Returns true if that used its
    /// action; false to go on and close in as usual.
    pub(crate) fn tactics(&mut self, id: MonsterId, sees: bool, events: &mut Vec<Event>) -> bool {
        let m = self.floor.monsters[id].clone();
        let def = self.content.monster(m.kind);
        let kind = m.kind;
        let player = self.player.pos;
        let distance = m.pos.chebyshev(player);

        // Candlelight scatters some things; the dark gives them back their nerve.
        if self.fears_light(&m) {
            if !matches!(m.mind, Mind::Fleeing) && self.floor.is_visible(m.pos) {
                events.push(Event::Scatters { kind });
                self.witness(kind, Trait::FearsLight);
            }
            self.floor.monsters[id].mind = Mind::Fleeing;
            if !self.step_away(id) && distance == 1 {
                self.monster_attack(id, events);
            }
            return true;
        }

        // Badly hurt, it runs to mend, and comes back when it has.
        if def.has(|t| *t == Trait::Mends) && !def.boss {
            let low = m.health * MEND_RUN_BELOW <= m.max_health;
            let mended = m.health * 3 >= m.max_health * 2;
            match m.mind {
                Mind::Hunting { .. } if low => {
                    self.floor.monsters[id].mind = Mind::Fleeing;
                    if self.floor.is_visible(m.pos) {
                        events.push(Event::Retreats { kind });
                        self.witness(kind, Trait::Mends);
                    }
                    self.step_away(id);
                    return true;
                }
                Mind::Fleeing if mended => {
                    self.floor.monsters[id].mind = Mind::Hunting { last_seen: player };
                }
                Mind::Fleeing => {
                    if !self.floor.in_sight(m.pos) || distance >= MEND_DISTANCE {
                        let monster = &mut self.floor.monsters[id];
                        monster.health = (monster.health + 1).min(monster.max_health);
                    }
                    if !self.step_away(id) && distance == 1 {
                        self.monster_attack(id, events);
                    }
                    return true;
                }
                _ => {}
            }
        }

        if !matches!(self.floor.monsters[id].mind, Mind::Hunting { .. }) {
            return false;
        }

        // Volleys from range; too close, it backs off to shoot again.
        if let Some((damage, range)) = def.traits.iter().find_map(|t| match *t {
            Trait::Shoots { damage, range } => Some((damage, range)),
            _ => None,
        }) && sees
        {
            if distance == 1 && self.step_away(id) {
                return true;
            }
            if (2..=range).contains(&distance) && m.cooldown > 0 {
                // Between volleys it keeps its distance.
                if distance <= 2 {
                    self.step_away(id);
                }
                return true;
            }
            if (2..=range).contains(&distance) && self.floor.in_sight(m.pos) {
                self.floor.monsters[id].cooldown = VOLLEY_COOLDOWN;
                let (accuracy, _) = self.monster_strength(&m);
                let chance = combat::hit_chance(accuracy, self.guarded_defense());
                let hit = combat::roll_attack(&mut self.combat_rng, chance, damage);
                self.witness(kind, Trait::Shoots { damage, range });
                events.push(Event::Volley { kind, damage: hit });
                if let Some(hit) = hit {
                    self.hurt_player(hit, Cause::Attack(kind), events);
                }
                return true;
            }
        }

        // A marked leap: two tiles off in a straight line, it shows where it
        // will land, and next action it comes.
        if let Some(bonus) = def.traits.iter().find_map(|t| match *t {
            Trait::Lunges { bonus } => Some(bonus),
            _ => None,
        }) && sees
            && m.cooldown == 0
            && self.lunge_middle(id, m.pos, player).is_some()
        {
            let monster = &mut self.floor.monsters[id];
            monster.lunging = Some(player);
            monster.blow_ready = false;
            events.push(Event::Crouches {
                kind,
                target: player,
            });
            self.witness(kind, Trait::Lunges { bonus });
            return true;
        }
        false
    }

    /// The open tile a leap from `from` at `to` would cross, if it's a clean
    /// straight leap of two.
    fn lunge_middle(&self, id: MonsterId, from: Point, to: Point) -> Option<Point> {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let straight = from.chebyshev(to) == 2
            && (dx == 0 || dy == 0 || dx.abs() == dy.abs())
            && dx.abs() != 1
            && dy.abs() != 1;
        if !straight {
            return None;
        }
        let middle = Point::new(from.x + dx / 2, from.y + dy / 2);
        self.can_enter(id, middle).then_some(middle)
    }

    /// The leap comes down: onto the tile between, striking you if you're
    /// still where it marked.
    pub(crate) fn land_lunge(&mut self, id: MonsterId, target: Point, events: &mut Vec<Event>) {
        let m = self.floor.monsters[id].clone();
        let def = self.content.monster(m.kind);
        let bonus = def
            .traits
            .iter()
            .find_map(|t| match *t {
                Trait::Lunges { bonus } => Some(bonus),
                _ => None,
            })
            .unwrap_or(0);
        let monster = &mut self.floor.monsters[id];
        monster.lunging = None;
        monster.cooldown = LUNGE_COOLDOWN;
        let middle = Point::new((m.pos.x + target.x) / 2, (m.pos.y + target.y) / 2);
        if middle != m.pos && self.can_enter(id, middle) {
            self.floor.monsters[id].pos = middle;
        }
        if self.player.pos == target && self.floor.monsters[id].pos.chebyshev(target) == 1 {
            let (accuracy, (lo, hi)) = self.monster_strength(&self.floor.monsters[id]);
            let chance = combat::hit_chance(accuracy, self.guarded_defense());
            let damage =
                combat::roll_attack(&mut self.combat_rng, chance, (lo + bonus, hi + bonus));
            events.push(Event::Leaps {
                kind: m.kind,
                damage,
            });
            if let Some(damage) = damage {
                self.hurt_player(damage, Cause::Attack(m.kind), events);
            }
        } else {
            events.push(Event::LeapsShort { kind: m.kind });
        }
    }

    /// Your defense, raised while you guard.
    pub fn guarded_defense(&self) -> i32 {
        self.player_defense()
            + if self.player.guarding {
                GUARD_DEFENSE
            } else {
                0
            }
    }

    /// Guard: wait a turn braced; misses against you are answered at once.
    pub(crate) fn guard(&mut self) -> Vec<Event> {
        self.player.guarding = true;
        let mut events = vec![Event::Guarding];
        self.pass_time(&mut events);
        events
    }

    /// Shove what's beside you a step back, into whatever is behind it.
    pub(crate) fn shove(&mut self, dir: Direction) -> Vec<Event> {
        let at = self.player.pos + dir;
        let Some(id) = self.floor.monster_at(at) else {
            return vec![Event::NothingToShove];
        };
        let m = self.floor.monsters[id].clone();
        let def = self.content.monster(m.kind);
        if m.phantom {
            self.floor.monsters.remove(id);
            let mut events = vec![Event::PhantomFaded {
                kind: m.kind,
                struck: true,
            }];
            self.pass_time(&mut events);
            return events;
        }
        if def.boss {
            return vec![Event::TooBigToShove { kind: m.kind }];
        }
        let to = at + dir;
        let tile = self.map().tile(to);
        let open = tile.is_walkable()
            && !matches!(tile, Tile::DoorClosed | Tile::DoorSealed)
            && self.floor.monster_at(to).is_none()
            && self.anomaly_at(to).is_none();
        if !open {
            return vec![Event::ShoveBlocked { kind: m.kind }];
        }
        let mut events = vec![Event::YouShove { kind: m.kind }];
        self.wake(id);
        self.fight_noise();
        if tile == Tile::RottenFloor {
            // Through the boards it goes, to the floor below.
            self.floor.monsters.remove(id);
            self.floor.set_tile(to, Tile::Pit);
            events.push(Event::ShovedThrough { kind: m.kind });
        } else {
            let monster = &mut self.floor.monsters[id];
            monster.pos = to;
            monster.winding_up = None;
            monster.lunging = None;
            monster.chanting = false;
        }
        // Struggling free of a grip.
        if self.player.held.is_some_and(|(holder, _)| holder == id) {
            self.player.held = None;
        }
        self.pass_time(&mut events);
        events
    }

    /// Held fast by something beside you: you can't step away.
    pub fn held_by(&self) -> Option<MonsterId> {
        let (id, until) = self.player.held?;
        let m = self.floor.monsters.get(id)?;
        (self.turn() < until && m.pos.chebyshev(self.player.pos) == 1).then_some(id)
    }

    /// A grab lands: you're held for a few turns.
    pub(crate) fn grabbed(&mut self, id: MonsterId, turns: u32, events: &mut Vec<Event>) {
        let kind = self.floor.monsters[id].kind;
        self.player.held = Some((id, self.turn() + u64::from(turns)));
        events.push(Event::Grabbed { kind });
        self.witness(kind, Trait::Grabs { turns });
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
        let mut world = world_from(
            "##############################\n\
             #............................#\n\
             #..............@.............#\n\
             #............................#\n\
             ##############################",
        );
        world.player.health = 999;
        world.player.max_health = 999;
        world
    }

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn at(world: &World, dx: i32, dy: i32) -> Point {
        let p = world.player().pos;
        Point::new(p.x + dx, p.y + dy)
    }

    #[test]
    fn swarms_flank_and_quicken_in_the_dark() {
        let mut world = hall();
        let a = world.spawn_monster(kind("fly_swarm"), at(&world, 1, 0));
        let alone = world.inspect(a).unwrap().its_hit_chance;
        world.spawn_monster(kind("fly_swarm"), at(&world, -1, 0));
        let info = world.inspect(a).unwrap();
        assert_eq!(info.flankers, 1);
        assert_eq!(info.its_hit_chance, alone + FLANK_ACCURACY as u32);
        let m = world.floor().monster(a).unwrap().clone();
        let lit = world.monster_speed(&m);
        world.apply(Command::ToggleCandle);
        assert_eq!(world.monster_speed(&m), lit + SWARM_DARK_SPEED as i32);
    }

    #[test]
    fn light_slows_the_dreaming() {
        let mut world = hall();
        let id = world.spawn_monster(kind("lantern_eater"), at(&world, 2, 0));
        let m = world.floor().monster(id).unwrap().clone();
        let def = Content::bundled().monster(m.kind);
        assert_eq!(
            world.monster_speed(&m),
            (def.speed - DREAMING_LIT_SLOW) as i32
        );
    }

    #[test]
    fn the_remnant_keep_to_their_posts() {
        let mut world = hall();
        let post = at(&world, -12, 0);
        let id = world.spawn_monster(kind("proctor"), post);
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        for _ in 0..20 {
            world.apply(Command::Wait);
        }
        let m = world.floor().monster(id).unwrap();
        assert!(
            m.pos.chebyshev(post) <= GUARD_LEASH,
            "it never strays past its leash"
        );
        assert!(m.pos.chebyshev(world.player().pos) > 1);
    }

    #[test]
    fn a_leap_is_marked_and_can_be_dodged() {
        let mut world = hall();
        world.apply(Command::ToggleCandle);
        let id = world.spawn_monster(kind("gnawer"), at(&world, 2, 0));
        world.spawn_monster(kind("gnawer"), at(&world, 3, 1));
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let mut events = Vec::new();
        for _ in 0..6 {
            events.extend(world.apply(Command::Wait));
            if world
                .floor()
                .monster(id)
                .is_some_and(|m| m.lunging.is_some())
            {
                break;
            }
        }
        let target = world
            .floor()
            .monster(id)
            .unwrap()
            .lunging
            .expect("crouched");
        assert!(
            world.telegraphs().any(|p| p == target),
            "the landing is marked"
        );
        let events = world.apply(Command::Move(crate::geom::Direction::N));
        assert!(
            events.iter().any(|e| matches!(e, Event::LeapsShort { .. })),
            "{events:?}"
        );
    }

    #[test]
    fn a_grip_holds_you_until_you_shove_it_off() {
        let mut world = hall();
        let id = world.spawn_monster(kind("parishioner"), at(&world, 1, 0));
        let mut events = Vec::new();
        world.grabbed(id, 2, &mut events);
        let here = world.player().pos;
        let refused = world.apply(Command::Move(crate::geom::Direction::W));
        assert!(matches!(refused[..], [Event::HeldFast { .. }]));
        assert_eq!(world.player().pos, here);
        world.apply(Command::Shove(crate::geom::Direction::E));
        assert_eq!(world.floor().monster(id).unwrap().pos, at(&world, 2, 0));
        assert!(world.held_by().is_none(), "shoved off");
    }

    #[test]
    fn shoved_onto_rotten_boards_it_falls_through() {
        let mut world = world_from("#######\n#@.,..#\n#######");
        let id = world.spawn_monster(kind("parishioner"), Point::new(2, 1));
        world.floor.set_tile(Point::new(3, 1), Tile::RottenFloor);
        let events = world.apply(Command::Shove(crate::geom::Direction::E));
        assert!(events.contains(&Event::ShovedThrough {
            kind: kind("parishioner")
        }));
        assert!(world.floor().monster(id).is_none());
    }

    #[test]
    fn folios_strike_from_afar() {
        let mut world = hall();
        let id = world.spawn_monster(kind("bound_folio"), at(&world, 4, 0));
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let events: Vec<Event> = (0..6).flat_map(|_| world.apply(Command::Wait)).collect();
        assert!(events.iter().any(|e| matches!(e, Event::Volley { .. })));
        assert!(
            world
                .floor()
                .monster(id)
                .unwrap()
                .pos
                .chebyshev(world.player().pos)
                > 1
        );
    }

    #[test]
    fn hurt_menders_fall_back_and_mend() {
        let mut world = hall();
        let id = world.spawn_monster(kind("proctor"), at(&world, 2, 0));
        world.floor.monsters[id].post = at(&world, 2, 0);
        world.floor.monsters[id].health = 3;
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let events = world.apply(Command::Wait);
        assert!(events.contains(&Event::Retreats {
            kind: kind("proctor")
        }));
        world.apply(Command::ToggleCandle);
        world.player.pos = Point::new(1, 1);
        for _ in 0..30 {
            world.apply(Command::Wait);
        }
        assert!(world.floor().monster(id).unwrap().health > 3, "it mended");
    }

    #[test]
    fn guarding_raises_defense_and_answers_misses() {
        let mut world = hall();
        let before = world.guarded_defense();
        let id = world.spawn_monster(kind("parishioner"), at(&world, 1, 0));
        world.floor.monsters[id].health = 999;
        let mut ripostes = 0;
        for _ in 0..40 {
            let events = world.apply(Command::Guard);
            ripostes += events
                .iter()
                .filter(|e| matches!(e, Event::Riposte { .. }))
                .count();
        }
        assert!(ripostes > 0, "guarding answers misses");
        world.player.guarding = true;
        assert_eq!(world.guarded_defense(), before + GUARD_DEFENSE);
        world.apply(Command::Wait);
        assert!(!world.player().guarding, "only until your next action");
    }
}
