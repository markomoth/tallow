//! Growing stronger: skills train from use, Insight raises your level, and
//! each level offers a draft of boons.

use rand::seq::IndexedRandom;

use crate::boons::{self, Boon, DraftContext, Passive, Reward, Trigger};
use crate::events::Event;
use crate::item::{Family, ItemClass, Slot};
use crate::monster::Mind;
use crate::skills::{Skill, Technique};
use crate::world::World;

/// Health gained with each level.
pub const HEALTH_PER_LEVEL: u32 = 3;

/// Insight for discoveries and victories.
pub const INSIGHT_FIRST_SIGHT: u32 = 8;
pub const INSIGHT_NEW_FLOOR: u32 = 12;
pub const INSIGHT_PER_THREAT: u32 = 6;
pub const INSIGHT_MANIFESTATION: u32 = 20;
pub const INSIGHT_TINCTURE_LEARNED: u32 = 5;

/// How far a lost trail reaches when a boon makes a hunter forget you.
const LOSE_TRAIL_RANGE: i32 = 12;

/// Total Insight needed to reach the level after `level`.
pub const fn insight_for_next(level: u32) -> u32 {
    15 * level * (level + 1)
}

/// Where a blow came from, for skills and boons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Melee(Option<Family>),
    Missile,
    Rite,
    /// Your compelled creature did it. Insight, but no kill boons.
    Thrall,
    /// Creatures fighting each other.
    Other,
    /// Burned to death. Insight if it burned, no kill boons.
    Fire,
}

impl World {
    pub fn rank(&self, skill: Skill) -> u32 {
        self.content
            .skill(skill)
            .rank_for(self.player.skills.xp(skill))
    }

    pub fn techniques(&self, skill: Skill) -> Vec<Technique> {
        self.content.skill(skill).techniques_at(self.rank(skill))
    }

    /// The draft waiting for a choice, if a level up is unresolved.
    pub fn pending_draft(&self) -> Option<&[Boon]> {
        self.drafts.front().map(Vec::as_slice)
    }

    /// Grants Insight directly. For tests and scripted scenes.
    pub fn grant_insight(&mut self, amount: u32) -> Vec<Event> {
        let mut events = Vec::new();
        self.gain_insight(amount, &mut events);
        events
    }

    /// How many level-up drafts are waiting.
    pub fn pending_drafts(&self) -> usize {
        self.drafts.len()
    }

    pub fn has_passive(&self, passive: Passive) -> bool {
        self.player.boons.contains(&Boon::Passive(passive))
    }

    /// Total of a stacking passive, like every `Accuracy(n)` taken.
    pub(crate) fn passive_sum(&self, pick: impl Fn(Passive) -> Option<i32>) -> i32 {
        self.player
            .boons
            .iter()
            .filter_map(|b| match b {
                Boon::Passive(p) => pick(*p),
                Boon::Triggered { .. } => None,
            })
            .sum()
    }

    /// The family of the weapon in hand, if any.
    pub(crate) fn wielded_family(&self) -> Option<Family> {
        let id = self.player.equipment.get(Slot::Melee)?;
        match self.content.item(self.inventory_item(id)?.kind).class {
            ItemClass::Melee { family, .. } => Some(family),
            _ => None,
        }
    }

    /// A technique of the wielded family, if its rank has unlocked it.
    pub(crate) fn melee_technique(&self, pick: impl Fn(Technique) -> bool) -> Option<Technique> {
        let skill = Skill::of_family(self.wielded_family()?);
        self.techniques(skill).into_iter().find(|&t| pick(t))
    }

    pub(crate) fn train(&mut self, skill: Skill, amount: u32, events: &mut Vec<Event>) {
        if amount == 0 {
            return;
        }
        let before = self.rank(skill);
        let known = self.techniques(skill);
        self.player.skills.add(skill, amount);
        let after = self.rank(skill);
        if after > before {
            events.push(Event::SkillRankUp { skill, rank: after });
            for technique in self.techniques(skill) {
                if !known.contains(&technique) {
                    events.push(Event::TechniqueLearned { skill, technique });
                }
            }
        }
    }

    pub(crate) fn gain_insight(&mut self, amount: u32, events: &mut Vec<Event>) {
        self.player.insight += amount;
        while self.player.insight >= insight_for_next(self.player.level) {
            self.player.level += 1;
            self.player.max_health += HEALTH_PER_LEVEL;
            self.player.health += HEALTH_PER_LEVEL;
            events.push(Event::LevelUp {
                level: self.player.level,
            });
            let ranks = |skill: Skill| self.rank(skill);
            let ctx = DraftContext {
                skills: &self.player.skills,
                ranks: &ranks,
                owned: &self.player.boons,
                snuffs: self.stats.snuffs,
                drinks: self.stats.drinks,
                heavy_blows_seen: self.stats.heavy_blows_seen,
                casts: self.stats.casts,
                studies: self.stats.studies,
                knows_rites: !self.player.rites.is_empty(),
            };
            let mut rng = self.boon_rng.clone();
            let draft = boons::draft(&mut rng, &ctx);
            self.boon_rng = rng;
            self.drafts.push_back(draft);
        }
    }

    pub(crate) fn choose_boon(&mut self, index: usize) -> Vec<Event> {
        let Some(boon) = self.drafts.front().and_then(|d| d.get(index)).copied() else {
            return Vec::new();
        };
        self.drafts.pop_front();
        if let Boon::Passive(Passive::MaxHealth(extra)) = boon {
            self.player.max_health += extra;
            self.player.health += extra;
        }
        self.player.boons.push(boon);
        let mut events = vec![Event::BoonTaken { boon }];
        // Picking a boon is a pause, not an action; but the view may change (dark sight).
        events.extend(self.update_view());
        events
    }

    /// Pays out every boon listening for `trigger`.
    pub(crate) fn trigger(&mut self, trigger: Trigger, events: &mut Vec<Event>) {
        let rewards: Vec<Reward> = self
            .player
            .boons
            .iter()
            .filter_map(|b| match *b {
                Boon::Triggered { when, then } if when == trigger => Some(then),
                _ => None,
            })
            .collect();
        for reward in rewards {
            match reward {
                Reward::Heal(n) => {
                    self.player.health = (self.player.health + n).min(self.player.max_health);
                }
                Reward::Dread(n) => self.shift_dread(n as i32 * 100, events),
                Reward::Tallow(n) => self.gain_tallow(n, events),
                Reward::LoseTrail => self.lose_nearest_trail(),
            }
        }
    }

    fn lose_nearest_trail(&mut self) {
        let here = self.player.pos;
        let hunters: Vec<_> = self
            .floor
            .monsters()
            .filter(|(_, m)| {
                matches!(m.mind, Mind::Hunting { .. }) && m.pos.chebyshev(here) <= LOSE_TRAIL_RANGE
            })
            .filter(|(_, m)| m.kind != self.manifestation)
            .map(|(id, m)| (m.pos.distance_squared(here), id))
            .collect();
        let nearest = hunters.iter().map(|&(d, _)| d).min();
        let closest: Vec<_> = hunters
            .iter()
            .filter(|&&(d, _)| Some(d) == nearest)
            .map(|&(_, id)| id)
            .collect();
        if let Some(&id) = closest.choose(&mut self.ai_rng) {
            self.floor.monsters[id].mind = Mind::Unaware;
        }
    }

    /// Insight and boons for a kill.
    pub(crate) fn credit_kill(
        &mut self,
        kind: crate::content::KindId,
        source: Source,
        events: &mut Vec<Event>,
    ) {
        let insight = if kind == self.manifestation {
            INSIGHT_MANIFESTATION
        } else {
            self.content.monster(kind).threat * INSIGHT_PER_THREAT
        };
        match source {
            Source::Other => return,
            Source::Thrall | Source::Fire => return self.gain_insight(insight, events),
            _ => {}
        }
        self.trigger(Trigger::Kill, events);
        self.wake_leavings(crate::leavings::Wake::OnKill, events);
        if let Source::Melee(Some(family)) = source {
            self.trigger(Trigger::KillWith(family), events);
        }
        self.gain_insight(insight, events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Command;
    use crate::boons::Reward;
    use crate::content::{Content, KindId};
    use crate::geom::{Direction, Point};
    use crate::item::{ItemId, ItemKindId};
    use crate::map::prefab;
    use crate::monster::MonsterId;

    fn hall() -> World {
        let (map, start) = prefab::parse(
            "##############################\n\
             #@...........................#\n\
             #............................#\n\
             ##############################",
        )
        .unwrap();
        World::from_map(map, start)
    }

    fn monster(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn item(id: &str) -> ItemKindId {
        Content::bundled().item_by_id(id).unwrap()
    }

    fn give(world: &mut World, kind: ItemKindId, count: u32) -> ItemId {
        world.place_item(world.player().pos, kind, count);
        world.apply(Command::PickUp);
        world
            .player()
            .inventory
            .iter()
            .find(|it| it.kind == kind)
            .unwrap()
            .id
    }

    fn wield(world: &mut World, id: &str) {
        let weapon = give(world, item(id), 1);
        world.apply(Command::Equip(weapon));
    }

    /// Attacks the monster east of the player until it's gone.
    fn fight_east(world: &mut World, id: MonsterId) -> Vec<Event> {
        let mut events = Vec::new();
        world.player.health = 999;
        world.player.max_health = 999;
        while world.floor().monster(id).is_some() && events.len() < 2000 {
            events.extend(world.apply(Command::Move(Direction::E)));
        }
        events
    }

    #[test]
    fn blades_rank_up_from_killing_with_a_sickle() {
        let mut world = hall();
        wield(&mut world, "sickle");
        let before = world.player_accuracy();
        let mut events = Vec::new();
        for _ in 0..3 {
            let east = world.player().pos + Direction::E;
            let id = world.spawn_monster(monster("parishioner"), east);
            events.extend(fight_east(&mut world, id));
        }
        assert!(events.contains(&Event::SkillRankUp {
            skill: Skill::Blades,
            rank: 1
        }));
        assert!(world.rank(Skill::Blades) >= 1);
        assert_eq!(
            world.player_accuracy(),
            before + 2 * world.rank(Skill::Blades) as i32
        );
    }

    #[test]
    fn experience_never_exceeds_what_a_creature_had_to_lose() {
        let mut world = hall();
        wield(&mut world, "censer");
        let east = world.player().pos + Direction::E;
        let id = world.spawn_monster(monster("gnawer"), east);
        fight_east(&mut world, id);
        assert!(
            world.player().skills.xp(Skill::Bludgeons) <= 4,
            "a gnawer has 4 health"
        );
    }

    #[test]
    fn phantoms_teach_nothing() {
        let mut world = hall();
        let east = world.player().pos + Direction::E;
        let id = world.spawn_monster(monster("parishioner"), east);
        world.floor.monsters[id].phantom = true;
        world.apply(Command::Move(Direction::E));
        assert_eq!(world.player().skills.xp(Skill::Bludgeons), 0);
    }

    #[test]
    fn endurance_trains_only_inside_a_vestment() {
        let mut world = hall();
        let mut events = Vec::new();
        world.hurt_player(
            3,
            crate::events::Cause::Attack(monster("gnawer")),
            &mut events,
        );
        assert_eq!(world.player().skills.xp(Skill::Endurance), 3);
        let cassock = world.player().equipment.body.unwrap();
        world.apply(Command::Equip(cassock));
        world.hurt_player(
            3,
            crate::events::Cause::Attack(monster("gnawer")),
            &mut events,
        );
        assert_eq!(
            world.player().skills.xp(Skill::Endurance),
            3,
            "no vestment, no training"
        );
    }

    #[test]
    fn missiles_train_from_throws() {
        let mut world = hall();
        let knives = give(&mut world, item("throwing_knife"), 30);
        let target = Point::new(4, 1);
        world.spawn_monster(monster("pallbearer"), target);
        for _ in 0..10 {
            world.apply(Command::Throw {
                item: knives,
                target,
            });
        }
        assert!(world.player().skills.xp(Skill::Missiles) > 0);
    }

    #[test]
    fn insight_brings_a_level_and_a_draft_of_three() {
        let mut world = hall();
        let mut events = Vec::new();
        world.gain_insight(insight_for_next(1), &mut events);
        assert!(events.contains(&Event::LevelUp { level: 2 }));
        assert_eq!(
            world.player().max_health,
            crate::combat::PLAYER_HEALTH + HEALTH_PER_LEVEL
        );
        let draft = world.pending_draft().unwrap().to_vec();
        assert_eq!(draft.len(), 3);

        let turn = world.turn();
        let events = world.apply(Command::ChooseBoon(1));
        assert!(events.contains(&Event::BoonTaken { boon: draft[1] }));
        assert!(world.pending_draft().is_none());
        assert_eq!(world.player().boons, vec![draft[1]]);
        assert_eq!(world.turn(), turn, "choosing is free");
    }

    #[test]
    fn several_levels_queue_several_drafts() {
        let mut world = hall();
        let mut events = Vec::new();
        world.gain_insight(insight_for_next(3), &mut events);
        assert_eq!(world.drafts.len(), 3);
    }

    #[test]
    fn rank_three_unlocks_a_technique() {
        let mut world = hall();
        let mut events = Vec::new();
        let needed = Content::bundled().skill(Skill::Blades).xp_for(3);
        world.train(Skill::Blades, needed, &mut events);
        assert!(events.contains(&Event::TechniqueLearned {
            skill: Skill::Blades,
            technique: Technique::Riposte { chance: 50 }
        }));
    }

    #[test]
    fn a_master_blade_ripostes_every_miss() {
        let mut world = hall();
        wield(&mut world, "sickle");
        let needed = Content::bundled().skill(Skill::Blades).xp_for(6);
        world.player.skills.add(Skill::Blades, needed);
        let east = world.player().pos + Direction::E;
        world.spawn_monster(monster("pallbearer"), east);
        world.player.health = 999;
        world.player.max_health = 999;
        let mut misses = 0;
        let mut ripostes = 0;
        for _ in 0..60 {
            for e in world.apply(Command::Wait) {
                match e {
                    Event::Attack {
                        attacker: crate::events::Who::Monster(_),
                        damage: None,
                        ..
                    } => misses += 1,
                    Event::Riposte { .. } => ripostes += 1,
                    _ => {}
                }
            }
        }
        assert!(misses > 0);
        assert_eq!(ripostes, misses);
    }

    #[test]
    fn a_reach_weapon_strikes_two_tiles_away() {
        let mut world = hall();
        wield(&mut world, "boathook");
        let needed = Content::bundled().skill(Skill::Reach).xp_for(3);
        world.player.skills.add(Skill::Reach, needed);
        let here = world.player().pos;
        world.spawn_monster(monster("pallbearer"), here + Direction::E + Direction::E);
        let events = world.apply(Command::Move(Direction::E));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Attack {
                attacker: crate::events::Who::Player,
                ..
            }
        )));
        assert_eq!(world.player().pos, here, "struck instead of stepping");
    }

    #[test]
    fn a_pinned_creature_stays_put() {
        let mut world = hall();
        let at = Point::new(8, 1);
        let id = world.spawn_monster(monster("parishioner"), at);
        world.floor.monsters[id].pinned = 5;
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        for _ in 0..3 {
            world.apply(Command::Wait);
        }
        assert_eq!(world.floor().monster(id).unwrap().pos, at);
    }

    #[test]
    fn bracing_softens_heavy_blows() {
        let mut world = hall();
        let needed = Content::bundled().skill(Skill::Endurance).xp_for(6);
        world.player.skills.add(Skill::Endurance, needed);
        world.player.health = 999;
        world.player.max_health = 999;
        let east = world.player().pos + Direction::E;
        let id = world.spawn_monster(monster("pallbearer"), east);
        world.floor.monsters[id].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let mut landed = Vec::new();
        for _ in 0..80 {
            for e in world.apply(Command::Wait) {
                if let Event::HeavyBlow {
                    damage: Some(d), ..
                } = e
                {
                    landed.push(d);
                }
            }
        }
        assert!(!landed.is_empty());
        assert!(
            landed.iter().all(|&d| d <= 12 - 12 * 40 / 100),
            "braced blows: {landed:?}"
        );
    }

    #[test]
    fn triggered_boons_pay_out() {
        let mut world = hall();
        world.player.boons.push(Boon::Triggered {
            when: Trigger::Kill,
            then: Reward::Heal(1),
        });
        world.player.health = 10;
        let mut events = Vec::new();
        world.trigger(Trigger::Kill, &mut events);
        assert_eq!(world.player().health, 11);
        world.trigger(Trigger::Descend, &mut events);
        assert_eq!(world.player().health, 11, "only the matching trigger pays");
    }

    #[test]
    fn passives_change_the_rules() {
        let mut world = hall();
        world
            .player
            .boons
            .push(Boon::Passive(Passive::CandleThrift));
        let start = world.player().candle.tallow();
        for _ in 0..40 {
            world.apply(Command::Wait);
        }
        assert_eq!(
            start - world.player().candle.tallow(),
            30,
            "a quarter slower"
        );

        let (light, _) = world.load_limits();
        world.player.boons.push(Boon::Passive(Passive::PackMule));
        assert_eq!(world.load_limits().0, light + 50);

        world.apply(Command::ToggleCandle);
        assert!(
            !world
                .floor()
                .is_visible(world.player().pos + Direction::E + Direction::E)
        );
        world.player.boons.push(Boon::Passive(Passive::DarkSight));
        world.apply(Command::Wait);
        assert!(
            world
                .floor()
                .is_visible(world.player().pos + Direction::E + Direction::E)
        );
    }

    #[test]
    fn kills_and_new_floors_bring_insight() {
        let mut world = World::new(5);
        let before = world.player().insight;
        let mut events = Vec::new();
        world.credit_kill(monster("pallbearer"), Source::Melee(None), &mut events);
        assert_eq!(world.player().insight, before + 4 * INSIGHT_PER_THREAT);
    }
}
