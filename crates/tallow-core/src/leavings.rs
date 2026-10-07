//! Leavings (BUILD_GUIDE.md §8): artifacts with a fixed rule for the run,
//! trigger × effect × cost, found in seep rooms among invisible anomalies and
//! dropped by the great ones below. Dangerous, sometimes lethal, but never
//! without warning: a danger tier you can see, a warning on first pickup, and
//! a warning turn before any cost that would kill you. Dropping one stops it.

use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};
use serde::Deserialize;

use crate::content::Faction;
use crate::events::{Cause, Event};
use crate::geom::{Direction, Point};
use crate::map::{Tile, path};
use crate::monster::{Mind, MonsterId};
use crate::world::World;

/// Index into the run's register of Leavings.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, Deserialize,
)]
pub struct LeavingId(pub u32);

/// What wakes a Leaving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Wake {
    /// You use it (from the pack).
    OnUse,
    /// On its own, every so many turns while you carry it.
    WhileCarried { every: u32 },
    /// You kill something.
    OnKill,
    /// Something hurts you.
    OnHurt,
    /// Your dread turns frayed (or worse).
    AtDread,
    /// Your candle goes out.
    EnteringDarkness,
}

/// What a Leaving does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Marvel {
    /// You swap places with the nearest creature you can see.
    SwapNearest,
    /// Every creature in view loses this many actions.
    StopTime {
        actions: u32,
    },
    /// Creatures nearby are dragged to you.
    Pull,
    /// Creatures beside you are thrown back.
    Push,
    /// You see in the dark around you for a while.
    DarkSight {
        turns: u32,
    },
    /// One stack in your pack grows by one.
    Duplicate,
    Mend {
        amount: u32,
    },
    Calm {
        amount: u32,
    },
    Kindle {
        amount: u32,
    },
    /// You are somewhere else on the floor, close by.
    Blink,
    /// Fire bursts up all around you (not where you stand).
    Ignite,
    /// The shape of the whole floor comes to you.
    Reveal,
    /// Holy ground around you for a while.
    Ward {
        turns: u32,
    },
    /// The nearest creature of the Dreaming in view is sent far away.
    Banish,
}

/// What a Leaving takes in return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Price {
    Health {
        amount: u32,
    },
    Dread {
        amount: u32,
    },
    Tallow {
        amount: u32,
    },
    /// No toll when it wakes, but it weighs as much as a vestment.
    Heavy,
    /// Every creature of this faction on the floor knows where you are.
    Attention(Faction),
}

/// How dangerous a Leaving looks. Shown by its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Mild,
    Strange,
    Deadly,
}

/// A named Leaving from `assets/leavings.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct NamedLeaving {
    pub id: String,
    pub name: String,
    pub tell: String,
    pub trigger: Wake,
    pub effect: Marvel,
    pub cost: Price,
    pub weight: u32,
    /// The creature that drops it when it dies.
    #[serde(default)]
    pub from: Option<String>,
    /// Or the floor whose seep room it lies in.
    #[serde(default)]
    pub seep_floor: Option<u8>,
}

/// Words for generated Leavings, and the named ones.
#[derive(Debug, Clone, Deserialize)]
pub struct LeavingParts {
    pub adjectives: Vec<String>,
    pub nouns: Vec<String>,
    pub named: Vec<NamedLeaving>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaving {
    pub name: String,
    /// The named ones have their own tell; generated ones are told by tier.
    pub tell: Option<String>,
    pub wake: Wake,
    pub effect: Marvel,
    pub price: Price,
    pub weight: u32,
    /// Its rule is known once it has woken.
    pub known: bool,
    /// It warned you its next toll would kill you. Involuntary ones then fire
    /// on the next turn unless dropped; ones you use fire on the next use.
    pub armed: bool,
}

/// Weight of a Leaving whose price is its weight.
const HEAVY_WEIGHT: u32 = 80;
/// Turns between uses of a Leaving you wake by hand.
pub const USE_COOLDOWN: u64 = 15;

impl Leaving {
    pub fn tier(&self) -> Tier {
        let involuntary = self.wake != Wake::OnUse;
        match self.price {
            Price::Attention(_) => Tier::Deadly,
            Price::Health { amount } if involuntary && amount >= 3 => Tier::Deadly,
            Price::Health { .. } => Tier::Strange,
            Price::Dread { amount } if amount >= 7 => Tier::Strange,
            Price::Tallow { amount } if amount >= 35 => Tier::Strange,
            _ => Tier::Mild,
        }
    }

    pub fn from_named(def: &NamedLeaving) -> Self {
        Self {
            name: def.name.clone(),
            tell: Some(def.tell.clone()),
            wake: def.trigger,
            effect: def.effect,
            price: def.cost,
            weight: def.weight,
            known: false,
            armed: false,
        }
    }

    /// Rolls a new Leaving's rule and name.
    pub fn roll<R: Rng + ?Sized>(rng: &mut R, parts: &LeavingParts) -> Self {
        let wakes = [
            (Wake::OnUse, 30),
            (
                Wake::WhileCarried {
                    every: rng.random_range(40..=80),
                },
                15,
            ),
            (Wake::OnKill, 20),
            (Wake::OnHurt, 15),
            (Wake::AtDread, 10),
            (Wake::EnteringDarkness, 10),
        ];
        let effects = [
            Marvel::SwapNearest,
            Marvel::StopTime {
                actions: rng.random_range(2..=4),
            },
            Marvel::Pull,
            Marvel::Push,
            Marvel::DarkSight {
                turns: rng.random_range(30..=60),
            },
            Marvel::Duplicate,
            Marvel::Mend {
                amount: rng.random_range(4..=8),
            },
            Marvel::Calm {
                amount: rng.random_range(8..=15),
            },
            Marvel::Kindle {
                amount: rng.random_range(30..=80),
            },
            Marvel::Blink,
            Marvel::Ignite,
            Marvel::Reveal,
            Marvel::Ward {
                turns: rng.random_range(20..=40),
            },
            Marvel::Banish,
        ];
        let prices = [
            (
                Price::Health {
                    amount: rng.random_range(2..=6),
                },
                30,
            ),
            (
                Price::Dread {
                    amount: rng.random_range(4..=10),
                },
                30,
            ),
            (
                Price::Tallow {
                    amount: rng.random_range(20..=50),
                },
                20,
            ),
            (Price::Heavy, 10),
            (
                Price::Attention(*Faction::ALL.choose(rng).expect("factions")),
                10,
            ),
        ];
        let wake = wakes.choose_weighted(rng, |w| w.1).expect("weights").0;
        let effect = *effects.choose(rng).expect("effects");
        let price = prices.choose_weighted(rng, |p| p.1).expect("weights").0;
        let adjective = parts.adjectives.choose(rng).expect("words");
        let noun = parts.nouns.choose(rng).expect("words");
        Self {
            name: format!("{adjective} {noun}"),
            tell: None,
            wake,
            effect,
            price,
            weight: if price == Price::Heavy {
                HEAVY_WEIGHT
            } else {
                rng.random_range(1..=5)
            },
            known: false,
            armed: false,
        }
    }
}

/// An invisible wrongness in a seep room, until something is thrown through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyKind {
    /// Burns what enters it.
    Heat,
    /// Holds what enters it, and crushes a little.
    Snare,
    /// Time stops inside it: turns go by.
    Pocket,
    /// Throws what enters it somewhere else on the floor.
    Swap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anomaly {
    pub at: Point,
    pub kind: AnomalyKind,
    pub revealed: bool,
}

/// Damage a heat well does (never fatal on its own).
const HEAT_DAMAGE: (u32, u32) = (4, 7);
const SNARE_DAMAGE: (u32, u32) = (2, 3);
const SNARE_TURNS: u32 = 3;
const POCKET_TURNS: u32 = 10;

impl World {
    pub fn leaving(&self, id: LeavingId) -> &Leaving {
        &self.leavings[id.0 as usize]
    }

    /// Leavings you carry, in the order you took them.
    pub fn carried_leavings(&self) -> &[LeavingId] {
        &self.player.leavings
    }

    pub(crate) fn register_leaving(&mut self, leaving: Leaving) -> LeavingId {
        self.leavings.push(leaving);
        LeavingId(self.leavings.len() as u32 - 1)
    }

    /// Puts a Leaving on the current floor. For tests and scripted scenes.
    pub fn place_leaving(&mut self, at: Point, leaving: Leaving) -> LeavingId {
        let id = self.register_leaving(leaving);
        self.floor.leavings.push((at, id));
        self.update_view();
        id
    }

    /// Weight of carried Leavings, in tenths.
    pub(crate) fn leaving_load(&self) -> u32 {
        self.player
            .leavings
            .iter()
            .map(|&id| self.leaving(id).weight)
            .sum()
    }

    /// Picks up any Leavings underfoot. The first one ever comes with a warning.
    pub(crate) fn take_leavings(&mut self, events: &mut Vec<Event>) {
        let here = self.player.pos;
        let found: Vec<LeavingId> = self
            .floor
            .leavings
            .iter()
            .filter(|&&(at, _)| at == here)
            .map(|&(_, id)| id)
            .collect();
        self.floor.leavings.retain(|&(at, _)| at != here);
        for id in found {
            let first = !self.stats.took_leaving;
            self.stats.took_leaving = true;
            self.player.leavings.push(id);
            if !self.leavings_taken.contains(&id) {
                self.leavings_taken.push(id);
            }
            events.push(Event::LeavingTaken { id, first });
        }
    }

    pub(crate) fn drop_leaving(&mut self, id: LeavingId) -> Vec<Event> {
        let Some(index) = self.player.leavings.iter().position(|&l| l == id) else {
            return Vec::new();
        };
        self.player.leavings.remove(index);
        self.leavings[id.0 as usize].armed = false;
        self.floor.leavings.push((self.player.pos, id));
        let mut events = vec![Event::LeavingDropped { id }];
        self.pass_time(&mut events);
        events
    }

    pub(crate) fn use_leaving(&mut self, id: LeavingId) -> Vec<Event> {
        if !self.player.leavings.contains(&id) {
            return Vec::new();
        }
        if self.leaving(id).wake != Wake::OnUse {
            return vec![Event::LeavingWontWake { id }];
        }
        if self
            .leaving_ready
            .get(&id)
            .is_some_and(|&t| self.turn() < t)
        {
            return vec![Event::LeavingResting { id }];
        }
        let mut events = Vec::new();
        self.leaving_ready.insert(id, self.turn() + USE_COOLDOWN);
        self.fire_leaving(id, &mut events);
        self.pass_time(&mut events);
        events
    }

    /// Wakes every carried Leaving listening for `wake`.
    pub(crate) fn wake_leavings(&mut self, wake: Wake, events: &mut Vec<Event>) {
        if self.leaving_busy {
            return;
        }
        let ids: Vec<LeavingId> = self
            .player
            .leavings
            .iter()
            .copied()
            .filter(|&id| {
                let w = self.leaving(id).wake;
                std::mem::discriminant(&w) == std::mem::discriminant(&wake)
            })
            .collect();
        for id in ids {
            self.fire_leaving(id, events);
        }
    }

    /// Once per turn: carried Leavings that wake on their own, and armed ones.
    pub(crate) fn tick_leavings(&mut self, events: &mut Vec<Event>) {
        let now = self.turn();
        let ids: Vec<LeavingId> = self.player.leavings.clone();
        for id in ids {
            let l = self.leaving(id);
            let due = matches!(l.wake, Wake::WhileCarried { every } if now.is_multiple_of(u64::from(every)));
            let armed = l.armed && l.wake != Wake::OnUse;
            if due || armed {
                self.fire_leaving(id, events);
            }
        }
        if self
            .player
            .dark_sight_until
            .is_some_and(|until| now >= until)
        {
            self.player.dark_sight_until = None;
            events.push(Event::DarkSightFaded);
        }
    }

    /// A Leaving wakes: its effect, then its price. A price that would kill
    /// you warns first and waits a turn (or, for one you use, another use).
    pub(crate) fn fire_leaving(&mut self, id: LeavingId, events: &mut Vec<Event>) {
        if self.leaving_busy || self.death.is_some() {
            return;
        }
        let l = self.leaving(id).clone();
        if let Price::Health { amount } = l.price
            && amount >= self.player.health
            && !l.armed
        {
            self.leavings[id.0 as usize].armed = true;
            events.push(Event::LeavingThreatens { id });
            return;
        }
        if !self.marvel_has_target(l.effect) {
            return;
        }
        self.leaving_busy = true;
        let known = l.known;
        let entry = &mut self.leavings[id.0 as usize];
        entry.known = true;
        entry.armed = false;
        events.push(Event::LeavingWoke {
            id,
            learned: !known,
        });
        self.do_marvel(l.effect, events);
        match l.price {
            Price::Health { amount } => self.hurt_player(amount, Cause::Leaving, events),
            // Dread is power now: the price drinks what you have gathered.
            Price::Dread { amount } => self.shift_dread(-(amount as i32) * 100, events),
            Price::Tallow { amount } => {
                let amount = amount.min(self.player.candle.tallow());
                self.player.candle.eat(amount);
            }
            Price::Heavy => {}
            Price::Attention(faction) => {
                let here = self.player.pos;
                for m in self.floor.monsters.values_mut() {
                    if !m.phantom
                        && m.compelled == 0
                        && self.content.monster(m.kind).faction == faction
                    {
                        m.mind = Mind::Hunting { last_seen: here };
                    }
                }
            }
        }
        self.leaving_busy = false;
        events.extend(self.update_view());
    }

    /// Effects that need something to act on stay quiet without it.
    fn marvel_has_target(&self, effect: Marvel) -> bool {
        let seen = || {
            self.hostiles_in_view()
                .into_iter()
                .any(|id| !self.floor.monsters[id].phantom)
        };
        match effect {
            Marvel::SwapNearest | Marvel::StopTime { .. } | Marvel::Pull | Marvel::Push => seen(),
            Marvel::Banish => self.nearest_dreaming().is_some(),
            Marvel::Duplicate => self
                .player
                .inventory
                .iter()
                .any(|it| self.content.item(it.kind).stacks()),
            _ => true,
        }
    }

    fn nearest_dreaming(&self) -> Option<MonsterId> {
        self.hostiles_in_view().into_iter().find(|&id| {
            let m = &self.floor.monsters[id];
            !m.phantom
                && self.content.monster(m.kind).faction == Faction::Dreaming
                && !self.content.monster(m.kind).boss
        })
    }

    fn do_marvel(&mut self, effect: Marvel, events: &mut Vec<Event>) {
        let here = self.player.pos;
        let in_view: Vec<MonsterId> = self
            .hostiles_in_view()
            .into_iter()
            .filter(|&id| !self.floor.monsters[id].phantom)
            .collect();
        match effect {
            Marvel::SwapNearest => {
                if let Some(&id) = in_view.first() {
                    let there = self.floor.monsters[id].pos;
                    self.floor.monsters[id].pos = here;
                    self.player.pos = there;
                }
            }
            Marvel::StopTime { actions } => {
                for id in in_view {
                    self.floor.monsters[id].energy -= crate::time::ACTION_COST * actions as i32;
                }
            }
            Marvel::Pull => {
                for id in in_view {
                    let from = self.floor.monsters[id].pos;
                    let line = crate::inventory::line(from, here);
                    let mut to = from;
                    for &p in &line[1..line.len().saturating_sub(1)] {
                        if !self.map().is_walkable(p) || self.floor.monster_at(p).is_some() {
                            break;
                        }
                        to = p;
                    }
                    self.floor.monsters[id].pos = to;
                }
            }
            Marvel::Push => {
                for id in in_view {
                    let from = self.floor.monsters[id].pos;
                    if from.chebyshev(here) > 3 {
                        continue;
                    }
                    let (dx, dy) = ((from.x - here.x).signum(), (from.y - here.y).signum());
                    let mut to = from;
                    for _ in 0..3 {
                        let next = Point::new(to.x + dx, to.y + dy);
                        if !self.map().is_walkable(next) || self.floor.monster_at(next).is_some() {
                            break;
                        }
                        to = next;
                    }
                    self.floor.monsters[id].pos = to;
                    self.floor.monsters[id].energy -= crate::time::ACTION_COST;
                }
            }
            Marvel::DarkSight { turns } => {
                self.player.dark_sight_until = Some(self.turn() + u64::from(turns));
            }
            Marvel::Duplicate => {
                let stacks: Vec<usize> = (0..self.player.inventory.len())
                    .filter(|&i| self.content.item(self.player.inventory[i].kind).stacks())
                    .collect();
                if let Some(&i) = stacks.choose(&mut self.ai_rng) {
                    self.player.inventory[i].count += 1;
                    events.push(Event::Duplicated {
                        kind: self.player.inventory[i].kind,
                    });
                }
            }
            Marvel::Mend { amount } => {
                self.player.health = (self.player.health + amount).min(self.player.max_health);
            }
            Marvel::Calm { amount } => self.shift_dread(-(amount as i32) * 100, events),
            Marvel::Kindle { amount } => self.gain_tallow(amount, events),
            Marvel::Blink => {
                let dist = path::distances(self.map(), here);
                let spots: Vec<Point> = self
                    .map()
                    .points()
                    .filter(|&p| {
                        self.map().tile(p) == Tile::Floor
                            && self.floor.monster_at(p).is_none()
                            && self.anomaly_at(p).is_none()
                            && dist.at(p).is_some_and(|d| (5..=10).contains(&d))
                    })
                    .collect();
                if let Some(&p) = spots.choose(&mut self.ai_rng) {
                    self.player.pos = p;
                }
            }
            Marvel::Ignite => {
                for d in Direction::ALL {
                    self.ignite(here + d, true);
                }
                self.floor.refresh_lights();
            }
            Marvel::Reveal => self.floor.reveal(here, 99),
            Marvel::Ward { turns } => {
                let until = self.turn() + u64::from(turns);
                for p in self.map().points() {
                    if p.distance_squared(here) <= 5 && self.map().is_walkable(p) {
                        self.floor.sanctify(p, until);
                    }
                }
            }
            Marvel::Banish => {
                if let Some(id) = self.nearest_dreaming() {
                    self.send_far(id);
                }
            }
        }
    }

    /// Sends a creature far from you on this floor; it forgets you.
    pub(crate) fn send_far(&mut self, id: MonsterId) {
        let dist = path::distances(self.map(), self.player.pos);
        let mut far: Vec<(u32, Point)> = self
            .map()
            .points()
            .filter(|&p| {
                self.map().tile(p) == Tile::Floor
                    && self.floor.monster_at(p).is_none()
                    && !self.floor.is_visible(p)
            })
            .filter_map(|p| dist.at(p).map(|d| (d, p)))
            .collect();
        far.sort();
        let start = far.len() * 2 / 3;
        if let Some(&(_, to)) = far[start..].choose(&mut self.ai_rng) {
            let m = &mut self.floor.monsters[id];
            m.pos = to;
            m.mind = Mind::Unaware;
            m.winding_up = None;
            m.foe = None;
        }
    }

    pub fn anomaly_at(&self, p: Point) -> Option<&Anomaly> {
        self.floor.anomalies.iter().find(|a| a.at == p)
    }

    /// Something thrown flew through an anomaly: it shows itself, and the
    /// throw ends there (or somewhere else, for a swap point). Returns where
    /// the projectile ends up, if an anomaly caught it.
    pub(crate) fn probe(&mut self, path: &[Point], events: &mut Vec<Event>) -> Option<Point> {
        let index = path.iter().position(|&p| self.anomaly_at(p).is_some())?;
        let at = path[index];
        let anomaly = self
            .floor
            .anomalies
            .iter_mut()
            .find(|a| a.at == at)
            .expect("found above");
        anomaly.revealed = true;
        let kind = anomaly.kind;
        events.push(Event::AnomalyRevealed { kind, at });
        Some(match kind {
            AnomalyKind::Swap => self.random_seep_floor().unwrap_or(at),
            _ => at,
        })
    }

    fn random_seep_floor(&mut self) -> Option<Point> {
        let spots: Vec<Point> = self
            .map()
            .points()
            .filter(|&p| {
                self.floor.is_seep(p)
                    && self.map().tile(p) == Tile::Floor
                    && self.anomaly_at(p).is_none()
            })
            .collect();
        spots.choose(&mut self.ai_rng).copied()
    }

    /// You walked into an anomaly.
    pub(crate) fn trip_anomaly(&mut self, at: Point, events: &mut Vec<Event>) {
        let Some(anomaly) = self.floor.anomalies.iter_mut().find(|a| a.at == at) else {
            return;
        };
        anomaly.revealed = true;
        let kind = anomaly.kind;
        let spare = |world: &World, dmg: u32| dmg.min(world.player.health.saturating_sub(1));
        match kind {
            AnomalyKind::Heat => {
                let damage = crate::combat::roll_damage(&mut self.combat_rng, HEAT_DAMAGE);
                let damage = spare(self, damage);
                events.push(Event::AnomalyStruck { kind, damage });
                self.player.health -= damage;
            }
            AnomalyKind::Snare => {
                let damage = crate::combat::roll_damage(&mut self.combat_rng, SNARE_DAMAGE);
                let damage = spare(self, damage);
                events.push(Event::AnomalyStruck { kind, damage });
                self.player.health -= damage;
                for _ in 0..SNARE_TURNS {
                    self.pass_time(events);
                }
            }
            AnomalyKind::Pocket => {
                events.push(Event::AnomalyStruck { kind, damage: 0 });
                for _ in 0..POCKET_TURNS {
                    self.pass_time(events);
                }
            }
            AnomalyKind::Swap => {
                events.push(Event::AnomalyStruck { kind, damage: 0 });
                let spots: Vec<Point> = self
                    .map()
                    .points()
                    .filter(|&p| {
                        self.map().tile(p) == Tile::Floor
                            && self.floor.monster_at(p).is_none()
                            && !self.floor.is_seep(p)
                    })
                    .collect();
                if let Some(&p) = spots.choose(&mut self.ai_rng) {
                    self.player.pos = p;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Command;
    use crate::content::Content;
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

    fn leaving(wake: Wake, effect: Marvel, price: Price) -> Leaving {
        Leaving {
            name: "test bone".into(),
            tell: None,
            wake,
            effect,
            price,
            weight: 2,
            known: false,
            armed: false,
        }
    }

    fn carry(world: &mut World, l: Leaving) -> LeavingId {
        let id = world.place_leaving(world.player().pos, l);
        world.apply(Command::PickUp);
        assert!(world.carried_leavings().contains(&id));
        id
    }

    #[test]
    fn generated_leavings_vary_and_tiers_follow_their_price() {
        let parts = &Content::bundled().leavings;
        let mut rng = crate::rng::floor_rng(1, 1);
        let rolled: Vec<Leaving> = (0..60).map(|_| Leaving::roll(&mut rng, parts)).collect();
        let tiers: std::collections::HashSet<Tier> = rolled.iter().map(Leaving::tier).collect();
        assert_eq!(tiers.len(), 3, "all three tiers turn up");
        let deadly = leaving(
            Wake::WhileCarried { every: 50 },
            Marvel::Reveal,
            Price::Health { amount: 5 },
        );
        assert_eq!(deadly.tier(), Tier::Deadly);
        let mild = leaving(Wake::OnUse, Marvel::Reveal, Price::Dread { amount: 4 });
        assert_eq!(mild.tier(), Tier::Mild);
    }

    #[test]
    fn the_first_leaving_comes_with_a_warning() {
        let mut world = hall();
        let l = leaving(
            Wake::OnUse,
            Marvel::Kindle { amount: 50 },
            Price::Dread { amount: 5 },
        );
        let id = world.place_leaving(world.player().pos, l);
        let events = world.apply(Command::PickUp);
        assert!(events.contains(&Event::LeavingTaken { id, first: true }));
    }

    #[test]
    fn using_one_does_its_thing_and_takes_its_price() {
        let mut world = hall();
        let id = carry(
            &mut world,
            leaving(
                Wake::OnUse,
                Marvel::Kindle { amount: 50 },
                Price::Dread { amount: 5 },
            ),
        );
        let tallow = world.player().candle.tallow();
        world.player.dread.set(20);
        let events = world.apply(Command::UseLeaving(id));
        assert!(events.contains(&Event::LeavingWoke { id, learned: true }));
        assert_eq!(world.player().candle.tallow(), tallow + 50 - 1);
        assert_eq!(world.player().dread.value(), 15, "it drinks your dread");
        assert!(world.leaving(id).known);
        assert_eq!(
            world.apply(Command::UseLeaving(id)),
            vec![Event::LeavingResting { id }]
        );
    }

    #[test]
    fn a_lethal_price_warns_a_turn_ahead_and_dropping_stops_it() {
        let mut world = hall();
        let id = carry(
            &mut world,
            leaving(Wake::OnHurt, Marvel::Reveal, Price::Health { amount: 6 }),
        );
        world.player.health = 5;
        let mut events = Vec::new();
        world.wake_leavings(Wake::OnHurt, &mut events);
        assert_eq!(events, vec![Event::LeavingThreatens { id }]);
        assert_eq!(world.player().health, 5, "nothing taken yet");
        world.apply(Command::DropLeaving(id));
        let events = world.apply(Command::Wait);
        assert!(world.death().is_none(), "{events:?}");

        // Kept, it fires on the next turn.
        let mut world = hall();
        let id = carry(
            &mut world,
            leaving(Wake::OnHurt, Marvel::Reveal, Price::Health { amount: 6 }),
        );
        world.player.health = 5;
        let mut events = Vec::new();
        world.wake_leavings(Wake::OnHurt, &mut events);
        world.apply(Command::Wait);
        assert_eq!(world.death().map(|d| d.cause), Some(Cause::Leaving));
        let _ = id;
    }

    #[test]
    fn a_thrown_stone_reveals_an_anomaly_and_stops_there() {
        let mut world = hall();
        world.floor.anomalies.push(Anomaly {
            at: Point::new(4, 1),
            kind: AnomalyKind::Heat,
            revealed: false,
        });
        let stone = Content::bundled().item_by_id("stone").unwrap();
        world.place_item(world.player().pos, stone, 1);
        world.apply(Command::PickUp);
        let id = world
            .player()
            .inventory
            .iter()
            .find(|i| i.kind == stone)
            .unwrap()
            .id;
        let events = world.apply(Command::Throw {
            item: id,
            target: Point::new(6, 1),
        });
        assert!(events.contains(&Event::AnomalyRevealed {
            kind: AnomalyKind::Heat,
            at: Point::new(4, 1)
        }));
        assert!(world.anomaly_at(Point::new(4, 1)).unwrap().revealed);
        assert_eq!(world.floor().items_at(Point::new(4, 1)).count(), 1);
    }

    #[test]
    fn walking_into_a_heat_well_burns_but_never_kills() {
        let mut world = hall();
        world.floor.anomalies.push(Anomaly {
            at: Point::new(2, 1),
            kind: AnomalyKind::Heat,
            revealed: false,
        });
        world.player.health = 3;
        let events = world.apply(Command::Move(Direction::E));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AnomalyStruck { .. }))
        );
        assert!(world.player().health >= 1 && world.death().is_none());
    }

    #[test]
    fn a_revealed_anomaly_needs_a_second_step() {
        let mut world = hall();
        world.floor.anomalies.push(Anomaly {
            at: Point::new(2, 1),
            kind: AnomalyKind::Pocket,
            revealed: true,
        });
        assert!(matches!(
            world.apply(Command::Move(Direction::E))[..],
            [Event::AnomalyAhead { .. }]
        ));
    }

    #[test]
    fn bosses_drop_their_named_leaving() {
        let mut world = hall();
        let sexton = Content::bundled().kind_by_id("sexton").unwrap();
        let id = world.spawn_monster(sexton, Point::new(5, 2));
        let mut events = Vec::new();
        world.damage_monster(id, 999, crate::progress::Source::Rite, &mut events);
        let (at, lid) = world.floor().leavings()[0];
        assert_eq!(at, Point::new(5, 2));
        assert_eq!(world.leaving(lid).name, "the Sexton's spade");
    }

    #[test]
    fn seep_floors_come_with_anomalies_a_leaving_and_stones() {
        let content = Content::bundled();
        let mut found = 0;
        for seed in 0..40 {
            let mut world = World::new(seed);
            world.dev_skip_to(11);
            assert!(
                !world.floor().anomalies().is_empty(),
                "floor 11 always seeps"
            );
            assert_eq!(world.floor().leavings().len(), 1);
            let stone = content.item_by_id("stone").unwrap();
            assert!(world.floor().items().iter().any(|f| f.item.kind == stone));
            found += 1;
        }
        assert_eq!(found, 40);
    }
}
