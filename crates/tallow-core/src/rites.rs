//! Rites: situational magic that costs dread (BUILD_GUIDE.md §5).
//!
//! Every rite changes a situation. Dread pays for rites, and dread makes them
//! stronger: the deeper your fear, the further your words reach.

use rand::seq::IndexedRandom;
use serde::Deserialize;

use crate::boons::{Passive, Trigger};
use crate::content::{Faction, Trait};
use crate::dread::DreadBand;
use crate::events::Event;
use crate::geom::Point;
use crate::monster::{Mind, MonsterId};
use crate::progress::Source;
use crate::skills::{Skill, Technique};
use crate::world::World;

/// Index of a rite definition in [`crate::Content::rites`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RiteId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum School {
    Binding,
    Communion,
    Veil,
    Warding,
}

impl School {
    pub const ALL: [School; 4] = [
        School::Binding,
        School::Communion,
        School::Veil,
        School::Warding,
    ];

    pub const fn skill(self) -> Skill {
        match self {
            School::Binding => Skill::Binding,
            School::Communion => Skill::Communion,
            School::Veil => Skill::Veil,
            School::Warding => Skill::Warding,
        }
    }
}

/// What a rite does, with its numbers at 1.0× potency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RiteEffect {
    /// The creature fights for you and follows you.
    Compel { actions: u32 },
    /// The creature can't move (it can still strike).
    Kneel { actions: u32 },
    /// Drain health from the creature into yours.
    Leech { amount: u32 },
    /// Your dread goes into the creature, which flees.
    Transference { dread: u32, actions: u32 },
    /// You also see what the creature sees.
    BorrowedEyes { turns: u32, radius: i32 },
    /// The creature forgets you and can't notice you for a while.
    Unsee { actions: u32 },
    /// A decoy light that draws creatures away from you.
    FalseFlame { turns: u32 },
    /// Nobody notices you unless they're beside you.
    Shroud { turns: u32 },
    /// Holy ground around you: the Dreaming can't cross, the Taken flinch.
    Sanctify { turns: u32, radius: i32 },
    /// Frees one of the Taken. Bosses only take damage.
    Exorcise { damage: u32 },
}

/// What a rite is aimed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiteTarget {
    Creature,
    /// Open ground in view.
    Tile,
    Myself,
}

impl RiteEffect {
    pub const fn target(self) -> RiteTarget {
        match self {
            RiteEffect::FalseFlame { .. } => RiteTarget::Tile,
            RiteEffect::Shroud { .. } | RiteEffect::Sanctify { .. } => RiteTarget::Myself,
            _ => RiteTarget::Creature,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiteDef {
    pub id: String,
    pub name: String,
    pub school: School,
    pub description: String,
    /// Dread added when cast.
    pub cost: u32,
    pub range: i32,
    pub effect: RiteEffect,
}

/// Why a rite couldn't be cast. Costs no time and no dread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiteFailure {
    /// You don't know that rite.
    Unknown,
    /// No creature you can see there.
    NoTarget,
    OutOfRange,
    /// Needs open ground you can see.
    NotOpenGround,
    /// Exorcise only works on the Taken.
    NotTaken,
    /// Your own dread does not obey.
    Immune,
    /// That creature already carries your dread.
    AlreadyCarries,
}

/// A decoy light from False Flame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoy {
    pub at: Point,
    pub until: u64,
}

pub const DECOY_RADIUS: i32 = 3;
pub const DECOY_COLOR: crate::map::light::Rgb = [190, 170, 255];
/// Creatures this close to a False Flame go to it.
pub const DECOY_PULL: i32 = 10;
/// ...unless you are this close to them.
pub const DECOY_IGNORED_WITHIN: i32 = 2;

/// Rites that last on the acolyte rather than on a creature or a floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rites {
    /// Shroud lasts until this turn.
    pub shroud_until: Option<u64>,
    /// Borrowed Eyes: whose, until when, and how far they see.
    pub borrowed: Option<(MonsterId, u64, i32)>,
}

/// Insight for learning a rite.
pub const INSIGHT_RITE: u32 = 10;
/// Insight for reading a text that has nothing new.
pub const INSIGHT_OLD_TEXT: u32 = 6;
/// Insight for freeing one of the Taken.
pub const INSIGHT_EXORCISM: u32 = 10;
/// Bosses resist Binding: its durations are divided by this.
const BOSS_RESISTANCE: u32 = 3;

impl World {
    /// Rites the acolyte knows, in the order learned.
    pub fn known_rites(&self) -> &[RiteId] {
        &self.player.rites
    }

    pub fn knows(&self, rite: RiteId) -> bool {
        self.player.rites.contains(&rite)
    }

    /// Teaches a rite directly. For tests and scripted scenes.
    pub fn teach_rite(&mut self, rite: RiteId) -> Vec<Event> {
        let mut events = Vec::new();
        self.learn_rite(rite, &mut events);
        events
    }

    pub(crate) fn learn_rite(&mut self, rite: RiteId, events: &mut Vec<Event>) {
        if self.knows(rite) {
            return;
        }
        self.player.rites.push(rite);
        events.push(Event::RiteLearned { rite });
        self.gain_insight(INSIGHT_RITE, events);
    }

    /// A random rite of `school` you don't know yet, if any remain.
    pub(crate) fn unknown_rite(&mut self, school: School) -> Option<RiteId> {
        let unknown: Vec<RiteId> = self
            .content
            .rite_ids()
            .filter(|&r| self.content.rite(r).school == school && !self.knows(r))
            .collect();
        unknown.choose(&mut self.rite_rng).copied()
    }

    /// Rite strength in percent: dread makes rites stronger, and so does practice.
    pub fn rite_potency(&self, rite: RiteId) -> u32 {
        let fear = match self.player.dread.band() {
            DreadBand::Calm => 100,
            DreadBand::Uneasy => 125,
            DreadBand::Frayed | DreadBand::Manifest => 150,
        };
        let school = self.content.rite(rite).school;
        fear * (100 + self.school_technique(school, true)) / 100
    }

    /// Dread a rite costs right now.
    pub fn rite_cost(&self, rite: RiteId) -> u32 {
        let def = self.content.rite(rite);
        let boon = if self.has_passive(Passive::RiteThrift) {
            25
        } else {
            0
        };
        let off = (self.school_technique(def.school, false) + boon).min(90);
        def.cost * (100 - off) / 100
    }

    /// The school's Deepen (or, with `deepen` false, Thrift) percent.
    fn school_technique(&self, school: School, deepen: bool) -> u32 {
        self.techniques(school.skill())
            .into_iter()
            .find_map(|t| match (t, deepen) {
                (Technique::Deepen { percent }, true) | (Technique::Thrift { percent }, false) => {
                    Some(percent)
                }
                _ => None,
            })
            .unwrap_or(0)
    }

    /// Scales a base amount by potency. Never below 1.
    fn potent(&self, rite: RiteId, base: u32) -> u32 {
        (base * self.rite_potency(rite) / 100).max(1)
    }

    /// Whether a rite could be cast at `target` right now, and on whom.
    pub fn rite_check(
        &self,
        rite: RiteId,
        target: Point,
    ) -> Result<Option<MonsterId>, RiteFailure> {
        if !self.knows(rite) {
            return Err(RiteFailure::Unknown);
        }
        let def = self.content.rite(rite);
        let here = self.player.pos;
        match def.effect.target() {
            RiteTarget::Myself => Ok(None),
            RiteTarget::Tile => {
                if !self.floor.is_visible(target) || !self.map().is_walkable(target) {
                    Err(RiteFailure::NotOpenGround)
                } else if target.chebyshev(here) > def.range {
                    Err(RiteFailure::OutOfRange)
                } else {
                    Ok(None)
                }
            }
            RiteTarget::Creature => {
                let id = self
                    .floor
                    .monster_at(target)
                    .filter(|&id| self.floor.is_visible(target) && !self.floor.monsters[id].phantom)
                    .ok_or(RiteFailure::NoTarget)?;
                if target.chebyshev(here) > def.range {
                    return Err(RiteFailure::OutOfRange);
                }
                let m = &self.floor.monsters[id];
                let mdef = self.content.monster(m.kind);
                let relentless = mdef.has(|t| *t == Trait::Relentless);
                match def.effect {
                    RiteEffect::Exorcise { .. } if mdef.faction != Faction::Taken => {
                        Err(RiteFailure::NotTaken)
                    }
                    RiteEffect::Transference { .. } if m.carries_dread => {
                        Err(RiteFailure::AlreadyCarries)
                    }
                    RiteEffect::Compel { .. }
                    | RiteEffect::Unsee { .. }
                    | RiteEffect::Transference { .. }
                        if relentless =>
                    {
                        Err(RiteFailure::Immune)
                    }
                    _ => Ok(Some(id)),
                }
            }
        }
    }

    pub(crate) fn cast(&mut self, rite: RiteId, target: Point) -> Vec<Event> {
        let victim = match self.rite_check(rite, target) {
            Ok(victim) => victim,
            Err(why) => return vec![Event::RiteFailed { rite, why }],
        };
        let def = self.content.rite(rite);
        let cost = self.rite_cost(rite);
        let mut events = vec![Event::Cast { rite }];
        match def.effect {
            RiteEffect::Compel { actions } => {
                let id = victim.expect("checked");
                let boss = self.content.monster(self.floor.monsters[id].kind).boss;
                let actions = self.potent(rite, actions) / if boss { BOSS_RESISTANCE } else { 1 };
                let m = &mut self.floor.monsters[id];
                m.compelled = actions.max(1);
                m.winding_up = None;
                m.foe = None;
                m.mind = Mind::Hunting {
                    last_seen: self.player.pos,
                };
                events.push(Event::Compelled { kind: m.kind });
            }
            RiteEffect::Kneel { actions } => {
                let id = victim.expect("checked");
                let boss = self.content.monster(self.floor.monsters[id].kind).boss;
                let actions = self.potent(rite, actions) / if boss { BOSS_RESISTANCE } else { 1 };
                let m = &mut self.floor.monsters[id];
                m.pinned = m.pinned.max(actions.max(1));
                events.push(Event::Knelt { kind: m.kind });
                self.wake(id);
            }
            RiteEffect::Leech { amount } => {
                let id = victim.expect("checked");
                let m = &self.floor.monsters[id];
                let kind = m.kind;
                let amount = self.potent(rite, amount).min(m.health);
                events.push(Event::Leeched { kind, amount });
                self.player.health = (self.player.health + amount).min(self.player.max_health);
                self.wake(id);
                self.damage_monster(id, amount, Source::Rite, &mut events);
            }
            RiteEffect::Transference { dread, actions } => {
                let id = victim.expect("checked");
                let amount = self.potent(rite, dread).min(self.player.dread.value());
                let actions = self.potent(rite, actions);
                let m = &mut self.floor.monsters[id];
                m.carries_dread = true;
                m.terrified = actions;
                m.winding_up = None;
                let kind = m.kind;
                events.push(Event::Transferred { kind, amount });
                self.shift_dread(-(amount as i32) * 100, &mut events);
            }
            RiteEffect::BorrowedEyes { turns, radius } => {
                let id = victim.expect("checked");
                let until = self.turn() + u64::from(self.potent(rite, turns));
                self.player.rite_state.borrowed = Some((id, until, radius));
                events.push(Event::EyesBorrowed {
                    kind: self.floor.monsters[id].kind,
                });
            }
            RiteEffect::Unsee { actions } => {
                let id = victim.expect("checked");
                let actions = self.potent(rite, actions);
                let m = &mut self.floor.monsters[id];
                m.unseeing = actions;
                if matches!(m.mind, Mind::Hunting { .. }) {
                    m.mind = Mind::Unaware;
                }
                m.winding_up = None;
                events.push(Event::Unseen { kind: m.kind });
            }
            RiteEffect::FalseFlame { turns } => {
                let until = self.turn() + u64::from(self.potent(rite, turns));
                self.floor.decoy = Some(Decoy { at: target, until });
                events.push(Event::FalseFlameLit { at: target });
            }
            RiteEffect::Shroud { turns } => {
                let until = self.turn() + u64::from(self.potent(rite, turns));
                self.player.rite_state.shroud_until = Some(until);
                events.push(Event::Shrouded);
            }
            RiteEffect::Sanctify { turns, radius } => {
                let until = self.turn() + u64::from(self.potent(rite, turns));
                let here = self.player.pos;
                let tiles: Vec<Point> = self
                    .map()
                    .points()
                    .filter(|&p| {
                        p.distance_squared(here) <= radius * radius + 1
                            && self.map().is_walkable(p)
                            && self.floor.in_sight(p)
                    })
                    .collect();
                for p in tiles {
                    self.floor.sanctify(p, until);
                }
                events.push(Event::Sanctified);
            }
            RiteEffect::Exorcise { damage } => {
                let id = victim.expect("checked");
                let m = &self.floor.monsters[id];
                let (kind, at) = (m.kind, m.pos);
                if self.content.monster(kind).boss {
                    let damage = self.potent(rite, damage);
                    events.push(Event::ExorciseResisted { kind, damage });
                    self.wake(id);
                    self.damage_monster(id, damage, Source::Rite, &mut events);
                } else {
                    self.floor.monsters.remove(id);
                    events.push(Event::Exorcised { kind, at });
                    self.stats.exorcised += 1;
                    self.gain_insight(INSIGHT_EXORCISM, &mut events);
                }
            }
        }
        self.shift_dread(cost as i32 * 100, &mut events);
        self.train(def.school.skill(), cost.max(4), &mut events);
        self.stats.casts += 1;
        self.trigger(Trigger::Cast, &mut events);
        self.pass_time(&mut events);
        events
    }

    /// Ends rites whose time is up. Called once per turn.
    pub(crate) fn tick_rites(&mut self, events: &mut Vec<Event>) {
        let now = self.turn();
        if self
            .player
            .rite_state
            .shroud_until
            .is_some_and(|until| now >= until)
        {
            self.player.rite_state.shroud_until = None;
            events.push(Event::ShroudFaded);
        }
        if let Some((id, until, _)) = self.player.rite_state.borrowed
            && (now >= until || !self.floor.monsters.contains_key(id))
        {
            self.player.rite_state.borrowed = None;
            events.push(Event::EyesReturned);
        }
        if self.floor.decoy.is_some_and(|d| now >= d.until) {
            self.floor.decoy = None;
            events.push(Event::FalseFlameOut);
        }
        if self.floor.expire_sanctity(now) {
            events.push(Event::SanctityFaded);
        }
    }

    /// Shroud is on: only what's beside you notices you.
    pub fn shrouded(&self) -> bool {
        self.player.rite_state.shroud_until.is_some()
    }

    /// The creature whose eyes you're borrowing, if any.
    pub fn borrowed_eyes(&self) -> Option<MonsterId> {
        self.player.rite_state.borrowed.map(|(id, _, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Command;
    use crate::content::{Content, KindId};
    use crate::geom::Direction;
    use crate::map::prefab;

    fn hall() -> World {
        let (map, start) = prefab::parse(
            "##############################\n\
             #@...........................#\n\
             #............................#\n\
             #............................#\n\
             ##############################",
        )
        .unwrap();
        World::from_map(map, start)
    }

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn rite(id: &str) -> RiteId {
        Content::bundled().rite_by_id(id).unwrap()
    }

    fn learn(world: &mut World, id: &str) -> RiteId {
        let r = rite(id);
        world.teach_rite(r);
        r
    }

    fn wait(world: &mut World, turns: u32) -> Vec<Event> {
        (0..turns)
            .flat_map(|_| world.apply(Command::Wait))
            .collect()
    }

    #[test]
    fn unknown_rites_cannot_be_cast() {
        let mut world = hall();
        let events = world.apply(Command::Cast {
            rite: rite("shroud"),
            target: world.player().pos,
        });
        assert_eq!(
            events,
            vec![Event::RiteFailed {
                rite: rite("shroud"),
                why: RiteFailure::Unknown
            }]
        );
        assert_eq!(world.turn(), 0);
    }

    #[test]
    fn casting_costs_dread_and_trains_the_school() {
        let mut world = hall();
        let shroud = learn(&mut world, "shroud");
        let before = world.player().dread.value();
        world.apply(Command::Cast {
            rite: shroud,
            target: world.player().pos,
        });
        assert_eq!(world.player().dread.value(), before + 8);
        assert!(world.player().skills.xp(Skill::Veil) > 0);
        assert!(world.shrouded());
        assert_eq!(world.turn(), 1);
    }

    #[test]
    fn dread_makes_rites_stronger() {
        let mut world = hall();
        let leech = learn(&mut world, "leech");
        assert_eq!(world.rite_potency(leech), 100);
        world.player.dread.set(50);
        assert_eq!(world.rite_potency(leech), 125);
        world.player.dread.set(80);
        assert_eq!(world.rite_potency(leech), 150);
    }

    #[test]
    fn a_compelled_creature_fights_another_and_then_wakes_hostile() {
        let mut world = hall();
        let compel = learn(&mut world, "compel");
        let proctor = world.spawn_monster(kind("proctor"), Point::new(4, 2));
        let taken = world.spawn_monster(kind("parishioner"), Point::new(7, 2));
        world.player.health = 999;
        world.player.max_health = 999;
        let events = world.apply(Command::Cast {
            rite: compel,
            target: Point::new(4, 2),
        });
        assert!(events.contains(&Event::Compelled {
            kind: kind("proctor")
        }));
        let mut log = Vec::new();
        for _ in 0..40 {
            log.extend(world.apply(Command::Wait));
            if world.floor().monster(taken).is_none() {
                break;
            }
        }
        assert!(
            log.iter().any(|e| matches!(
                e,
                Event::Attack {
                    attacker: crate::events::Who::Monster(a),
                    defender: crate::events::Who::Monster(d),
                    ..
                } if *a == kind("proctor") && *d == kind("parishioner")
            )),
            "the proctor attacks the parishioner"
        );
        assert!(
            log.iter().all(|e| !matches!(
                e,
                Event::Attack {
                    attacker: crate::events::Who::Monster(a),
                    defender: crate::events::Who::Player,
                    ..
                } if *a == kind("proctor")
            )),
            "a thrall never strikes you"
        );
        let more = wait(&mut world, 40);
        assert!(more.contains(&Event::CompelEnded {
            kind: kind("proctor")
        }));
        let m = world.floor().monster(proctor).unwrap();
        assert_eq!(m.compelled, 0);
        assert!(matches!(m.mind, Mind::Hunting { .. }));
    }

    #[test]
    fn you_swap_places_with_a_thrall() {
        let mut world = hall();
        let compel = learn(&mut world, "compel");
        let east = world.player().pos + Direction::E;
        let id = world.spawn_monster(kind("parishioner"), east);
        world.apply(Command::Cast {
            rite: compel,
            target: east,
        });
        let here = world.player().pos;
        let pos = world.floor().monster(id).unwrap().pos;
        let dir = Direction::ALL
            .into_iter()
            .find(|&d| here + d == pos)
            .unwrap();
        let events = world.apply(Command::Move(dir));
        assert!(events.contains(&Event::SwappedPlaces {
            kind: kind("parishioner")
        }));
        assert_eq!(world.player().pos, pos);
        assert_eq!(world.floor().monster(id).unwrap().pos, here);
    }

    #[test]
    fn exorcise_frees_the_taken_and_refuses_others() {
        let mut world = hall();
        let exorcise = learn(&mut world, "exorcise");
        let east = Point::new(3, 1);
        world.spawn_monster(kind("gnawer"), east);
        assert_eq!(
            world.apply(Command::Cast {
                rite: exorcise,
                target: east
            }),
            vec![Event::RiteFailed {
                rite: exorcise,
                why: RiteFailure::NotTaken
            }]
        );
        world.despawn_all();
        let id = world.spawn_monster(kind("parishioner"), east);
        let events = world.apply(Command::Cast {
            rite: exorcise,
            target: east,
        });
        assert!(events.iter().any(|e| matches!(e, Event::Exorcised { .. })));
        assert!(world.floor().monster(id).is_none());
        assert_eq!(world.stats.exorcised, 1);
    }

    #[test]
    fn leech_heals_you_by_what_it_takes() {
        let mut world = hall();
        let leech = learn(&mut world, "leech");
        world.player.health = 10;
        let id = world.spawn_monster(kind("pallbearer"), Point::new(3, 1));
        world.apply(Command::Cast {
            rite: leech,
            target: Point::new(3, 1),
        });
        let lost = 20 - world.floor().monster(id).map_or(20, |m| m.health);
        assert_eq!(lost, 5);
        assert!(world.player().health >= 15 - 3, "healed 5, maybe hit since");
    }

    #[test]
    fn transference_moves_dread_once_per_creature() {
        let mut world = hall();
        let give = learn(&mut world, "transference");
        world.player.dread.set(60);
        let at = Point::new(4, 1);
        let id = world.spawn_monster(kind("parishioner"), at);
        world.apply(Command::Cast {
            rite: give,
            target: at,
        });
        // 20 at 1.25× potency, less the shock of first sight (+5).
        assert_eq!(world.player().dread.value(), 60 - 25 + 5);
        assert!(world.floor().monster(id).unwrap().carries_dread);
        let at = world.floor().monster(id).unwrap().pos;
        if at.chebyshev(world.player().pos) <= 5 {
            assert!(matches!(
                world.apply(Command::Cast {
                    rite: give,
                    target: at
                })[..],
                [Event::RiteFailed {
                    why: RiteFailure::AlreadyCarries,
                    ..
                }]
            ));
        }
    }

    #[test]
    fn your_own_dread_will_not_obey() {
        let mut world = hall();
        let compel = learn(&mut world, "compel");
        let at = Point::new(4, 1);
        world.spawn_monster(kind("manifestation"), at);
        assert!(matches!(
            world.apply(Command::Cast {
                rite: compel,
                target: at
            })[..],
            [Event::RiteFailed {
                why: RiteFailure::Immune,
                ..
            }]
        ));
    }

    #[test]
    fn sanctified_ground_keeps_the_dreaming_out() {
        let mut world = hall();
        let sanctify = learn(&mut world, "sanctify");
        world.apply(Command::Cast {
            rite: sanctify,
            target: world.player().pos,
        });
        let eater = world.spawn_monster(kind("lantern_eater"), Point::new(12, 2));
        world.floor.monsters[eater].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        world.player.health = 999;
        let events = wait(&mut world, 20);
        assert!(
            events.iter().all(|e| !matches!(
                e,
                Event::Attack {
                    defender: crate::events::Who::Player,
                    ..
                }
            )),
            "it never reached you"
        );
        let pos = world.floor().monster(eater).unwrap().pos;
        assert!(!world.floor().is_sanctified(pos));
    }

    #[test]
    fn unseen_creatures_lose_you() {
        let mut world = hall();
        let unsee = learn(&mut world, "unsee");
        let at = Point::new(6, 1);
        let id = world.spawn_monster(kind("parishioner"), at);
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        world.apply(Command::Cast {
            rite: unsee,
            target: at,
        });
        assert_eq!(world.floor().monster(id).unwrap().mind, Mind::Unaware);
        wait(&mut world, 3);
        assert_eq!(world.floor().monster(id).unwrap().mind, Mind::Unaware);
    }

    #[test]
    fn a_false_flame_draws_hunters_away() {
        let mut world = hall();
        let flame = learn(&mut world, "false_flame");
        let id = world.spawn_monster(kind("parishioner"), Point::new(4, 1));
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let decoy = Point::new(6, 2);
        let events = world.apply(Command::Cast {
            rite: flame,
            target: decoy,
        });
        assert!(events.contains(&Event::FalseFlameLit { at: decoy }));
        wait(&mut world, 8);
        let pos = world.floor().monster(id).unwrap().pos;
        assert!(pos.chebyshev(decoy) <= 1, "went to the flame: {pos:?}");
        let events = wait(&mut world, 30);
        assert!(events.contains(&Event::FalseFlameOut));
    }

    #[test]
    fn borrowed_eyes_see_past_your_light() {
        let (map, start) = prefab::parse(
            "##############################\n\
             #@...........................#\n\
             ##############################",
        )
        .unwrap();
        let mut world = World::from_map(map, start);
        let eyes = learn(&mut world, "borrowed_eyes");
        let at = Point::new(7, 1);
        world.spawn_monster(kind("parishioner"), at);
        assert!(!world.floor().is_visible(Point::new(13, 1)));
        world.apply(Command::Cast {
            rite: eyes,
            target: at,
        });
        let pos = world.floor().monsters().next().map(|(_, m)| m.pos).unwrap();
        assert!(
            world
                .floor()
                .is_visible(pos + Direction::E + Direction::E + Direction::E)
        );
    }

    #[test]
    fn practice_deepens_and_thrift_cheapens() {
        let mut world = hall();
        let compel = learn(&mut world, "compel");
        let shroud = learn(&mut world, "shroud");
        world.player.skills.add(Skill::Binding, 1000);
        world.player.skills.add(Skill::Veil, 1000);
        assert_eq!(world.rite_potency(compel), 200);
        assert_eq!(world.rite_cost(shroud), 4);
    }
}
