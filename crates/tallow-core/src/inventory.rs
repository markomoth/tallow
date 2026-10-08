//! The acolyte's pack: carrying, weight, equipment, drinking, throwing, firing.

use rand::RngExt;

use crate::boons::{Passive, Trigger};
use crate::combat::{self, BASE_ACCURACY, BASE_DEFENSE, FISTS};
use crate::content::Faction;
use crate::events::Event;
use crate::floor::FloorItem;
use crate::geom::Point;
use crate::item::{
    self, BITTER_DREAD, Burden, Item, ItemClass, ItemId, ItemKindId, Quirk, SideEffect, Slot,
    ThrowStats, TinctureEffect,
};
use crate::monster::MonsterId;
use crate::progress::Source;
use crate::skills::{Skill, Technique};
use crate::world::World;

/// How far a hand can throw.
pub const THROW_RANGE: i32 = 7;
/// Distinct kinds of thing the pack can hold (one letter each).
pub const PACK_SLOTS: usize = 26;

/// What a projectile is and how it flies.
#[derive(Debug, Clone, Copy)]
struct Shot {
    item: ItemKindId,
    accuracy: i32,
    damage: (u32, u32),
    range: i32,
    breaks: bool,
    holy: Option<item::Holy>,
    fire: bool,
    oil: bool,
    noise: Option<u32>,
}

impl World {
    /// Total weight carried, in tenths, tallow included.
    pub fn load(&self) -> u32 {
        let items: u32 = self
            .player
            .inventory
            .iter()
            .map(|it| self.content.item(it.kind).weight * it.count)
            .sum();
        items + self.player.candle.weight() + self.leaving_load()
    }

    pub fn burden(&self) -> Burden {
        let (light, max) = self.load_limits();
        Burden::within(self.load(), light, max)
    }

    /// Carrying limits in tenths: (burdened above, stuck above).
    pub fn load_limits(&self) -> (u32, u32) {
        let extra = if self.has_passive(Passive::PackMule) {
            50
        } else {
            0
        };
        (item::LIGHT_LOAD + extra, item::MAX_LOAD + extra)
    }

    pub fn inventory_item(&self, id: ItemId) -> Option<&Item> {
        self.player.inventory.iter().find(|it| it.id == id)
    }

    /// The acolyte's chance-to-hit base plus the wielded weapon's.
    pub fn player_accuracy(&self) -> i32 {
        let skill = self
            .wielded_family()
            .map_or(0, |f| self.skill_accuracy(Skill::of_family(f)));
        BASE_ACCURACY + self.melee().map_or(0, |(acc, _)| acc) + skill + self.boon_accuracy()
    }

    /// Accuracy with thrown and fired things, before the projectile's own.
    pub fn missile_accuracy(&self) -> i32 {
        let boons = self.passive_sum(|p| match p {
            Passive::MissileAccuracy(n) => Some(n),
            _ => None,
        });
        BASE_ACCURACY + self.skill_accuracy(Skill::Missiles) + self.boon_accuracy() + boons
    }

    fn skill_accuracy(&self, skill: Skill) -> i32 {
        self.content.skill(skill).accuracy_per_rank * self.rank(skill) as i32
    }

    fn boon_accuracy(&self) -> i32 {
        self.passive_sum(|p| match p {
            Passive::Accuracy(n) => Some(n),
            _ => None,
        })
    }

    pub fn player_damage(&self) -> (u32, u32) {
        let (lo, hi) = self.melee().map_or(FISTS, |(_, dmg)| dmg);
        let family = self.wielded_family();
        let mut bonus = self.passive_sum(|p| match p {
            Passive::FamilyDamage(f) if Some(f) == family => Some(1),
            _ => None,
        }) as u32;
        if let Some(Quirk::Candlelit { damage }) = self.wielded_quirk()
            && (self.player.candle.is_lit() || self.player.vigil)
        {
            bonus += damage;
        }
        (lo + bonus, hi + bonus)
    }

    /// What sets the wielded weapon apart, if anything.
    pub fn wielded_quirk(&self) -> Option<Quirk> {
        match self.equipped_def(Slot::Melee)? {
            ItemClass::Melee { quirk, .. } => *quirk,
            _ => None,
        }
    }

    pub fn player_defense(&self) -> i32 {
        let worn = self
            .equipped_def(Slot::Body)
            .map_or(0, |class| match class {
                ItemClass::Vestment { defense } => *defense,
                _ => 0,
            });
        let endurance = self.content.skill(Skill::Endurance).defense_per_rank
            * self.rank(Skill::Endurance) as i32;
        let boons = self.passive_sum(|p| match p {
            Passive::Defense(n) => Some(n),
            _ => None,
        });
        BASE_DEFENSE + worn + endurance + boons
    }

    fn melee(&self) -> Option<(i32, (u32, u32))> {
        match self.equipped_def(Slot::Melee)? {
            ItemClass::Melee {
                accuracy, damage, ..
            } => Some((*accuracy, *damage)),
            _ => None,
        }
    }

    fn equipped_def(&self, slot: Slot) -> Option<&ItemClass> {
        let id = self.player.equipment.get(slot)?;
        let item = self.inventory_item(id)?;
        Some(&self.content.item(item.kind).class)
    }

    /// Where a projectile aimed at `target` would fly, stopping at walls,
    /// range, or the first creature you can see. For the targeting preview.
    pub fn aim(&self, target: Point, range: i32) -> Vec<Point> {
        self.flight(target, range, |p| {
            self.floor.monster_at(p).is_some() && self.floor.is_visible(p)
        })
    }

    /// Range of the equipped ranged weapon, if any.
    pub fn fire_range(&self) -> Option<i32> {
        match self.equipped_def(Slot::Ranged)? {
            ItemClass::Ranged { range, .. } => Some(*range),
            _ => None,
        }
    }

    /// The straight line toward `target`, cut at walls and at `range`, and
    /// ending early on the first tile where `stop` is true.
    fn flight(&self, target: Point, range: i32, stop: impl Fn(Point) -> bool) -> Vec<Point> {
        let mut path = Vec::new();
        for p in line(self.player.pos, target).into_iter().skip(1) {
            if self.map().blocks_sight(p) || p.chebyshev(self.player.pos) > range {
                break;
            }
            path.push(p);
            if stop(p) {
                break;
            }
        }
        path
    }

    pub(crate) fn pick_up(&mut self) -> Vec<Event> {
        let here = self.player.pos;
        let mut found: Vec<FloorItem> = Vec::new();
        self.floor.items.retain(|f| {
            if f.at == here {
                found.push(*f);
                false
            } else {
                true
            }
        });
        let leavings_here = self.floor.leavings.iter().any(|&(at, _)| at == here);
        if found.is_empty() && !leavings_here {
            return vec![Event::NothingToPickUp];
        }
        let mut events = Vec::new();
        self.take_leavings(&mut events);
        for f in found {
            if matches!(self.content.item(f.item.kind).class, ItemClass::Relic)
                && !self.take_relic(&mut events)
            {
                self.floor.items.push(f);
                continue;
            }
            if self.add_to_pack(f.item) {
                events.push(Event::PickedUp {
                    kind: f.item.kind,
                    count: f.item.count,
                });
            } else {
                events.push(Event::PackFull { kind: f.item.kind });
                self.floor.items.push(f);
            }
        }
        self.pass_time(&mut events);
        events
    }

    /// Adds an item, merging stacks. False if the pack has no free slot.
    pub(crate) fn add_to_pack(&mut self, item: Item) -> bool {
        let def = self.content.item(item.kind);
        if def.stacks()
            && let Some(stack) = self
                .player
                .inventory
                .iter_mut()
                .find(|it| it.kind == item.kind)
        {
            stack.count += item.count;
            return true;
        }
        if self.player.inventory.len() >= PACK_SLOTS {
            return false;
        }
        self.player.inventory.push(item);
        true
    }

    /// Takes `count` of an item out of the pack (unequipping it if needed).
    pub(crate) fn take_from_pack(&mut self, id: ItemId, count: u32) -> Option<Item> {
        let index = self.player.inventory.iter().position(|it| it.id == id)?;
        let stack = &mut self.player.inventory[index];
        let taken = count.min(stack.count);
        if taken == stack.count {
            let item = self.player.inventory.remove(index);
            for slot in [Slot::Melee, Slot::Ranged, Slot::Body] {
                if self.player.equipment.get(slot) == Some(id) {
                    self.player.equipment.set(slot, None);
                }
            }
            Some(item)
        } else {
            stack.count -= taken;
            let kind = stack.kind;
            Some(Item {
                id: self.new_item_id(),
                kind,
                count: taken,
            })
        }
    }

    pub(crate) fn new_item_id(&mut self) -> ItemId {
        self.next_item += 1;
        ItemId(self.next_item)
    }

    pub(crate) fn drop_item(&mut self, id: ItemId) -> Vec<Event> {
        let Some(count) = self.inventory_item(id).map(|it| it.count) else {
            return Vec::new();
        };
        let item = self.take_from_pack(id, count).expect("checked above");
        let at = self.player.pos;
        self.floor.items.push(FloorItem::seen(at, item));
        let mut events = vec![Event::Dropped {
            kind: item.kind,
            count: item.count,
        }];
        self.pass_time(&mut events);
        events
    }

    /// Equips, or takes off if already equipped. Fills the item's slot, setting aside what was there.
    pub(crate) fn toggle_equip(&mut self, id: ItemId) -> Vec<Event> {
        let Some(item) = self.inventory_item(id).copied() else {
            return Vec::new();
        };
        let Some(slot) = self.content.item(item.kind).slot() else {
            return vec![Event::CantUse { kind: item.kind }];
        };
        let mut events = Vec::new();
        if self.player.equipment.get(slot) == Some(id) {
            self.player.equipment.set(slot, None);
            events.push(Event::Unequipped { kind: item.kind });
        } else {
            if let Some(old) = self
                .player
                .equipment
                .get(slot)
                .and_then(|o| self.inventory_item(o).copied())
            {
                events.push(Event::Unequipped { kind: old.kind });
            }
            self.player.equipment.set(slot, Some(id));
            events.push(Event::Equipped { kind: item.kind });
        }
        self.pass_time(&mut events);
        events
    }

    /// Drinks a tincture, or equips gear.
    pub(crate) fn use_item(&mut self, id: ItemId) -> Vec<Event> {
        let Some(item) = self.inventory_item(id).copied() else {
            return Vec::new();
        };
        let effect = match self.content.item(item.kind).class {
            ItemClass::Tincture { effect } => effect,
            ItemClass::Text { .. } => return self.read(id),
            ItemClass::Bell { noise } => {
                let mut events = Vec::new();
                let here = self.player.pos;
                self.ring_bell(here, noise, &mut events);
                self.pass_time(&mut events);
                return events;
            }
            _ => return self.toggle_equip(id),
        };
        self.take_from_pack(id, 1);
        let lore = self.tinctures[&item.kind];
        let base = item::tincture_amount(effect, lore.potency);
        let amount =
            if effect == TinctureEffect::Mending && self.has_passive(Passive::StrongMedicine) {
                base * 3 / 2
            } else {
                base
            };
        let mut events = vec![Event::Drank {
            kind: item.kind,
            effect,
            potency: lore.potency,
            side: lore.side,
            amount,
            learned: !lore.known,
        }];
        self.tinctures
            .get_mut(&item.kind)
            .expect("rolled at start")
            .known = true;
        match effect {
            TinctureEffect::Mending => {
                self.player.health = (self.player.health + amount).min(self.player.max_health);
            }
            TinctureEffect::Steadying => self.shift_dread(-(amount as i32) * 100, &mut events),
            TinctureEffect::Seeing => self.floor.reveal(self.player.pos, amount as i32),
        }
        if lore.side == SideEffect::Bitter {
            self.shift_dread(BITTER_DREAD, &mut events);
        }
        self.stats.drinks += 1;
        self.trigger(Trigger::Drink, &mut events);
        if !lore.known {
            self.gain_insight(crate::progress::INSIGHT_TINCTURE_LEARNED, &mut events);
        }
        self.pass_time(&mut events);
        events
    }

    pub(crate) fn throw(&mut self, id: ItemId, target: Point) -> Vec<Event> {
        let Some(item) = self.inventory_item(id).copied() else {
            return Vec::new();
        };
        let Some(ThrowStats {
            accuracy,
            damage,
            breaks,
            holy,
            fire,
            oil,
            noise,
        }) = self.content.item(item.kind).thrown
        else {
            return vec![Event::CantThrow { kind: item.kind }];
        };
        if target == self.player.pos || self.aim(target, THROW_RANGE).is_empty() {
            return vec![Event::BadTarget];
        }
        let thrown = self.take_from_pack(id, 1).expect("checked above");
        let shot = Shot {
            item: thrown.kind,
            accuracy: self.missile_accuracy() + accuracy,
            damage,
            range: THROW_RANGE,
            breaks,
            holy,
            fire,
            oil,
            noise,
        };
        let mut events = vec![Event::Thrown { kind: thrown.kind }];
        self.launch(shot, thrown, target, &mut events);
        self.pass_time(&mut events);
        events
    }

    pub(crate) fn fire(&mut self, target: Point) -> Vec<Event> {
        let Some(ItemClass::Ranged {
            damage,
            accuracy,
            range,
            ammo,
        }) = self.equipped_def(Slot::Ranged).cloned()
        else {
            return vec![Event::NoRangedWeapon];
        };
        let ammo_kind = self
            .content
            .item_by_id(&ammo)
            .expect("validated when content loads");
        let Some(stack) = self
            .player
            .inventory
            .iter()
            .find(|it| it.kind == ammo_kind)
            .copied()
        else {
            return vec![Event::NoAmmo { kind: ammo_kind }];
        };
        if target == self.player.pos || self.aim(target, range).is_empty() {
            return vec![Event::BadTarget];
        }
        let round = self.take_from_pack(stack.id, 1).expect("found above");
        let shot = Shot {
            item: ammo_kind,
            accuracy: self.missile_accuracy() + accuracy,
            damage,
            range,
            breaks: false,
            holy: None,
            fire: false,
            oil: false,
            noise: None,
        };
        let mut events = vec![Event::Fired { kind: ammo_kind }];
        self.launch(shot, round, target, &mut events);
        self.pass_time(&mut events);
        events
    }

    /// Flies a projectile toward `target`. Phantoms it passes through come
    /// apart; the first real creature takes a roll to hit.
    fn launch(&mut self, shot: Shot, projectile: Item, target: Point, events: &mut Vec<Event>) {
        let mut path = self.flight(target, shot.range, |p| {
            self.floor
                .monster_at(p)
                .is_some_and(|id| !self.floor.monsters[id].phantom)
        });
        // An anomaly on the way catches the throw and shows itself.
        if let Some(index) = path.iter().position(|&p| self.anomaly_at(p).is_some()) {
            path.truncate(index + 1);
            let caught = self
                .probe(&path, events)
                .expect("an anomaly is on the path");
            self.land(shot, projectile, caught, events);
            return;
        }
        for &p in &path {
            if let Some(id) = self
                .floor
                .monster_at(p)
                .filter(|&id| self.floor.monsters[id].phantom)
            {
                let kind = self.floor.monsters[id].kind;
                self.floor.monsters.remove(id);
                events.push(Event::PhantomFaded { kind, struck: true });
            }
        }
        let Some(&end) = path.last() else {
            // Thrown straight into a wall: it falls at your feet.
            self.land(shot, projectile, self.player.pos, events);
            return;
        };
        if let Some(id) = self.floor.monster_at(end) {
            self.strike(shot, id, events);
        }
        self.land(shot, projectile, end, events);
    }

    fn strike(&mut self, shot: Shot, id: MonsterId, events: &mut Vec<Event>) {
        let kind = self.floor.monsters[id].kind;
        let def = self.content.monster(kind);
        let at = self.floor.monsters[id].pos;
        let chance = combat::hit_chance(shot.accuracy - self.dark_penalty(at), def.defense);
        let hit = self.combat_rng.random_range(0..100) < chance;
        let damage = match (hit, shot.holy) {
            (false, _) => None,
            (true, Some(holy)) => Some(match def.faction {
                Faction::Dreaming => combat::roll_damage(&mut self.combat_rng, holy.dreaming),
                Faction::Taken => combat::roll_damage(&mut self.combat_rng, holy.taken),
                Faction::Swarm | Faction::Remnant => 0,
            }),
            (true, None) => {
                let roll = combat::roll_damage(&mut self.combat_rng, shot.damage);
                Some(
                    if combat::resists(def.faction, combat::Weapon::Missile, self.in_the_dark(at)) {
                        combat::halve(roll)
                    } else {
                        roll
                    },
                )
            }
        };
        events.push(Event::ProjectileHit {
            item: shot.item,
            target: kind,
            damage,
        });
        self.wake(id);
        let Some(damage) = damage else { return };
        let health_before = self.floor.monsters[id].health;
        self.train(Skill::Missiles, damage.min(health_before), events);
        self.trigger(Trigger::MissileHit, events);
        self.damage_monster(id, damage, Source::Missile, events);
        if damage > 0
            && let Some(Technique::Pin { chance, actions }) = self
                .techniques(Skill::Missiles)
                .into_iter()
                .find(|t| matches!(t, Technique::Pin { .. }))
            && self.floor.monsters.contains_key(id)
            && self.combat_rng.random_range(0..100) < chance
        {
            self.floor.monsters[id].pinned = actions;
            events.push(Event::Pinned { kind });
        }
    }

    fn land(&mut self, shot: Shot, projectile: Item, at: Point, events: &mut Vec<Event>) {
        if shot.holy.is_some() {
            self.consecrate(at, events);
        }
        if shot.oil {
            self.spill_oil(at);
            events.push(Event::OilSpilled { at });
        }
        if shot.fire {
            let cross = [
                crate::geom::Direction::N,
                crate::geom::Direction::S,
                crate::geom::Direction::E,
                crate::geom::Direction::W,
            ];
            // The flame splashes away from the hand that threw it.
            let here = self.player.pos;
            let mut caught = false;
            for p in std::iter::once(at).chain(cross.iter().map(|&d| at + d)) {
                if p != here {
                    caught |= self.ignite(p, true);
                }
            }
            if caught {
                events.push(Event::Ignited { at });
            }
            self.floor.refresh_lights();
        }
        if let Some(noise) = shot.noise {
            self.ring_bell(at, noise, events);
        }
        if shot.breaks {
            events.push(Event::Shattered {
                kind: shot.item,
                at,
            });
        } else {
            self.floor.items.push(FloorItem::seen(at, projectile));
        }
    }
}

/// Grid points on a straight line from `a` to `b`, both included (Bresenham).
pub fn line(a: Point, b: Point) -> Vec<Point> {
    let (dx, dy) = ((b.x - a.x).abs(), -(b.y - a.y).abs());
    let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut err = dx + dy;
    let mut p = a;
    let mut points = vec![p];
    while p != b {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            p.x += sx;
        }
        if e2 <= dx {
            err += dx;
            p.y += sy;
        }
        points.push(p);
    }
    points
}

/// Rolls this run's strength and side effect for every tincture kind.
pub(crate) fn roll_tinctures<R: rand::Rng + ?Sized>(
    rng: &mut R,
    content: &crate::Content,
) -> std::collections::HashMap<ItemKindId, item::TinctureLore> {
    use item::{Potency, TinctureLore};
    content
        .item_kinds()
        .filter(|(_, def)| matches!(def.class, ItemClass::Tincture { .. }))
        .map(|(kind, _)| {
            let potency = [Potency::Weak, Potency::Common, Potency::Strong][rng.random_range(0..3)];
            let side = if rng.random_bool(0.3) {
                SideEffect::Bitter
            } else {
                SideEffect::None
            };
            (
                kind,
                TinctureLore {
                    potency,
                    side,
                    known: false,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_continuous_and_end_where_asked() {
        let a = Point::new(0, 0);
        for b in [
            Point::new(5, 2),
            Point::new(-3, 7),
            Point::new(4, -4),
            Point::new(0, 6),
        ] {
            let pts = line(a, b);
            assert_eq!(pts.first(), Some(&a));
            assert_eq!(pts.last(), Some(&b));
            for w in pts.windows(2) {
                assert_eq!(w[0].chebyshev(w[1]), 1);
            }
        }
    }
}

#[cfg(test)]
mod world_tests {
    use crate::actions::Command;
    use crate::content::{Content, KindId};
    use crate::events::Event;
    use crate::geom::{Direction, Point};
    use crate::item::{Burden, ItemId, ItemKindId, Slot};
    use crate::map::prefab;
    use crate::world::World;

    fn world_from(text: &str) -> World {
        let (map, start) = prefab::parse(text).unwrap();
        World::from_map(map, start)
    }

    fn hall() -> World {
        world_from(
            "##############################\n\
             #@...........................#\n\
             #............................#\n\
             ##############################",
        )
    }

    fn item(id: &str) -> ItemKindId {
        Content::bundled().item_by_id(id).unwrap()
    }

    fn monster(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn pack_id(world: &World, kind: ItemKindId) -> ItemId {
        world
            .player()
            .inventory
            .iter()
            .find(|it| it.kind == kind)
            .unwrap()
            .id
    }

    /// Puts an item underfoot and picks it up.
    fn give(world: &mut World, kind: ItemKindId, count: u32) -> ItemId {
        world.place_item(world.player().pos, kind, count);
        world.apply(Command::PickUp);
        pack_id(world, kind)
    }

    #[test]
    fn the_acolyte_starts_with_candlestick_cassock_and_a_tincture() {
        let world = hall();
        let p = world.player();
        assert_eq!(p.inventory.len(), 3);
        assert!(p.equipment.melee.is_some() && p.equipment.body.is_some());
        assert_eq!(world.player_accuracy(), 85);
        assert_eq!(world.player_defense(), 10);
        assert_eq!(world.load(), 30 + 20 + 3 + 30);
        assert_eq!(world.burden(), Burden::Light);
    }

    #[test]
    fn items_are_spotted_picked_up_and_stacked() {
        let mut world = hall();
        world.place_item(Point::new(12, 1), item("stone"), 5);
        let events = world.apply(Command::Run(Direction::E));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::SpottedItem { .. }))
        );
        assert!(world.player().pos.x < 12, "runs stop for new items");

        assert_eq!(world.apply(Command::PickUp), vec![Event::NothingToPickUp]);
        give(&mut world, item("stone"), 3);
        give(&mut world, item("stone"), 2);
        let stones = world
            .player()
            .inventory
            .iter()
            .filter(|it| it.kind == item("stone"))
            .count();
        assert_eq!(stones, 1, "one stack");
        assert_eq!(
            world
                .inventory_item(pack_id(&world, item("stone")))
                .unwrap()
                .count,
            5
        );
    }

    #[test]
    fn weight_slows_then_stops_you() {
        let mut world = hall();
        // 400 in the candle, 500 in lumps: 4.0 + 10.0.
        world.player.candle.add(600);
        let mut events = Vec::new();
        world.place_item(world.player().pos, item("choir_mail"), 1);
        events.extend(world.apply(Command::PickUp));
        assert!(events.contains(&Event::BurdenChanged {
            burden: Burden::Burdened
        }));

        let turn = world.turn();
        world.apply(Command::Move(Direction::E));
        world.apply(Command::Move(Direction::E));
        assert_eq!(
            world.turn() - turn,
            3,
            "two burdened steps take three turns"
        );

        for _ in 0..2 {
            world.place_item(world.player().pos, item("choir_mail"), 1);
            world.apply(Command::PickUp);
        }
        assert_eq!(world.burden(), Burden::Overloaded);
        let here = world.player().pos;
        assert_eq!(
            world.apply(Command::Move(Direction::E)),
            vec![Event::TooHeavy]
        );
        assert_eq!(world.player().pos, here);
    }

    #[test]
    fn tallow_weighs_something() {
        let mut world = hall();
        let before = world.load();
        world.place_tallow(Point::new(2, 1), 500);
        world.apply(Command::Move(Direction::E));
        assert!(world.load() >= before + 49);
    }

    #[test]
    fn equipping_swaps_weapons_and_changes_accuracy() {
        let mut world = hall();
        let sickle = give(&mut world, item("sickle"), 1);
        let events = world.apply(Command::Equip(sickle));
        assert!(events.contains(&Event::Equipped {
            kind: item("sickle")
        }));
        assert!(events.contains(&Event::Unequipped {
            kind: item("iron_candlestick")
        }));
        assert_eq!(world.player().equipment.get(Slot::Melee), Some(sickle));
        assert_eq!(world.player_accuracy(), 92);

        world.apply(Command::Equip(sickle));
        assert_eq!(
            world.player().equipment.melee,
            None,
            "equip again takes it off"
        );
        assert_eq!(world.player_damage(), crate::combat::FISTS);
    }

    #[test]
    fn dropping_gear_takes_it_off() {
        let mut world = hall();
        let cassock = world.player().equipment.body.unwrap();
        world.apply(Command::Drop(cassock));
        assert_eq!(world.player().equipment.body, None);
        assert_eq!(world.player_defense(), crate::combat::BASE_DEFENSE);
        assert_eq!(world.floor().items_at(world.player().pos).count(), 1);
    }

    #[test]
    fn tinctures_are_learned_by_drinking() {
        let mut world = hall();
        let kind = item("mending_tincture");
        assert!(!world.tincture_lore(kind).unwrap().known);
        world.player.health = 5;
        let first = pack_id(&world, kind);
        let events = world.apply(Command::Use(first));
        let Some(Event::Drank {
            amount, learned, ..
        }) = events.first().copied()
        else {
            panic!("expected a drink, got {events:?}");
        };
        assert!(learned);
        assert!(world.tincture_lore(kind).unwrap().known);
        assert_eq!(
            world.player().health,
            (5 + amount).min(world.player().max_health)
        );
        assert!(
            world.player().inventory.iter().all(|it| it.kind != kind),
            "used up"
        );

        let second = give(&mut world, kind, 1);
        let events = world.apply(Command::Use(second));
        assert!(matches!(
            events.first(),
            Some(Event::Drank { learned: false, .. })
        ));
    }

    #[test]
    fn a_seeing_tincture_shows_the_floor() {
        let mut world = hall();
        let far = Point::new(27, 2);
        assert!(!world.floor().is_explored(far));
        let flask = give(&mut world, item("seeing_tincture"), 1);
        world
            .tinctures
            .get_mut(&item("seeing_tincture"))
            .unwrap()
            .potency = crate::item::Potency::Strong;
        world.apply(Command::Use(flask));
        assert!(world.floor().is_explored(far));
    }

    #[test]
    fn thrown_knives_fly_hit_and_land() {
        let mut world = hall();
        let knives = give(&mut world, item("throwing_knife"), 20);
        world.spawn_monster(monster("parishioner"), Point::new(5, 1));
        let target = Point::new(5, 1);
        assert_eq!(
            *world.aim(target, super::THROW_RANGE).last().unwrap(),
            target
        );

        let mut hit = false;
        for _ in 0..20 {
            let events = world.apply(Command::Throw {
                item: knives,
                target,
            });
            assert!(events.contains(&Event::Thrown {
                kind: item("throwing_knife")
            }));
            if events.iter().any(|e| {
                matches!(
                    e,
                    Event::ProjectileHit {
                        damage: Some(_),
                        ..
                    }
                )
            }) {
                hit = true;
                break;
            }
            if world.inventory_item(knives).is_none() || world.death().is_some() {
                break;
            }
        }
        assert!(hit, "a 95% throw lands within a few tries");
        assert!(
            world.floor().items_at(target).count() > 0
                || world.floor().monster_at(target).is_none()
        );
    }

    #[test]
    fn projectiles_stop_at_walls() {
        let world = world_from("#########\n#@..#...#\n#########");
        let path = world.aim(Point::new(7, 1), 10);
        assert_eq!(path.last(), Some(&Point::new(3, 1)));
    }

    #[test]
    fn holy_water_burns_the_dreaming_and_shatters() {
        let mut world = hall();
        let flasks = give(&mut world, item("holy_water"), 10);
        world.spawn_monster(monster("lantern_eater"), Point::new(4, 1));
        let mut burned = false;
        for _ in 0..10 {
            let events = world.apply(Command::Throw {
                item: flasks,
                target: Point::new(4, 1),
            });
            assert!(events.iter().any(|e| matches!(e, Event::Shattered { .. })));
            if events
                .iter()
                .any(|e| matches!(e, Event::ProjectileHit { damage: Some(d), .. } if *d >= 5))
            {
                burned = true;
                break;
            }
        }
        assert!(burned);
        assert_eq!(world.floor().items().len(), 0, "flasks don't survive");

        let mut world = hall();
        let flasks = give(&mut world, item("holy_water"), 10);
        world.spawn_monster(monster("gnawer"), Point::new(4, 1));
        for _ in 0..5 {
            let events = world.apply(Command::Throw {
                item: flasks,
                target: Point::new(4, 1),
            });
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, Event::ProjectileHit { damage: Some(d), .. } if *d > 0))
            );
        }
    }

    #[test]
    fn slings_need_stones() {
        let mut world = hall();
        let target = Point::new(6, 1);
        assert_eq!(
            world.apply(Command::Fire { target }),
            vec![Event::NoRangedWeapon]
        );
        let sling = give(&mut world, item("sling"), 1);
        world.apply(Command::Equip(sling));
        assert_eq!(world.fire_range(), Some(8));
        assert_eq!(
            world.apply(Command::Fire { target }),
            vec![Event::NoAmmo {
                kind: item("stone")
            }]
        );
        give(&mut world, item("stone"), 2);
        let events = world.apply(Command::Fire { target });
        assert!(events.contains(&Event::Fired {
            kind: item("stone")
        }));
        assert_eq!(
            world.floor().items_at(target).count(),
            1,
            "the stone lands where it was aimed"
        );
    }

    #[test]
    fn throwing_through_a_phantom_dispels_it() {
        let mut world = hall();
        let stones = give(&mut world, item("stone"), 3);
        let id = world.spawn_monster(monster("gnawer"), Point::new(4, 1));
        world.floor.monsters[id].phantom = true;
        let events = world.apply(Command::Throw {
            item: stones,
            target: Point::new(8, 1),
        });
        assert!(events.contains(&Event::PhantomFaded {
            kind: monster("gnawer"),
            struck: true
        }));
        assert_eq!(
            world.floor().items_at(Point::new(8, 1)).count(),
            1,
            "it flew on"
        );
    }

    #[test]
    fn floors_come_with_loot() {
        let world = World::new(21);
        assert!((3..=5).contains(&world.floor().items().len()));
        let again = World::new(21);
        assert_eq!(world.floor().items(), again.floor().items());
    }
}
