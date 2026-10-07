//! Corpses are a decision (BUILD_GUIDE.md §4): study one for knowledge,
//! render it for tallow, or leave it to rot into a fly swarm.

use crate::boons::Trigger;
use crate::content::{Faction, KindId};
use crate::events::Event;
use crate::geom::Point;
use crate::item::ItemClass;
use crate::item::ItemId;
use crate::monster::{Mind, Monster};
use crate::world::World;

/// A body begins to swell (and says so) this many turns after death...
pub const SWELL_TURNS: u64 = 70;
/// ...and rots away at this age.
pub const ROT_TURNS: u64 = 100;
/// Bodies this big hatch a fly swarm when they rot; smaller ones just go.
pub const HATCH_HEALTH: u32 = 8;
/// Turns to render a corpse into tallow.
pub const RENDER_TURNS: u32 = 10;
/// Turns to read a text.
pub const READ_TURNS: u32 = 3;
/// Insight for studying a kind of creature for the first time.
pub const INSIGHT_STUDY: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Corpse {
    pub at: Point,
    pub kind: KindId,
    /// The turn it died.
    pub died: u64,
    /// Turns of study and rendering done so far. Interrupted work resumes.
    pub studied: u32,
    pub rendered: u32,
    /// Old bones that were here before you: they don't rot.
    pub ancient: bool,
}

/// How a body is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decay {
    Fresh,
    /// Will rot soon; big ones hatch flies.
    Swelling,
}

impl Corpse {
    pub fn decay(&self, now: u64) -> Decay {
        if now.saturating_sub(self.died) >= SWELL_TURNS {
            Decay::Swelling
        } else {
            Decay::Fresh
        }
    }
}

/// Turns to study a creature: longer for more dangerous ones, 10–20.
pub fn study_turns(threat: u32) -> u32 {
    (10 + 2 * threat).min(20)
}

/// How far the smell of rendering fat carries, in steps. Scavengers come.
pub const RENDER_SMELL: u32 = 10;

/// Tallow a rendered body gives. The Taken were people, and render rich;
/// the Remnant are dry things; swarms are mostly shell.
pub fn render_yield(faction: Faction, max_health: u32) -> u32 {
    match faction {
        Faction::Taken => (max_health * 6).clamp(20, 140),
        Faction::Remnant => (max_health * 3).clamp(10, 80),
        Faction::Swarm => (max_health * 2).clamp(4, 40),
        Faction::Dreaming => wax_yield(max_health),
    }
}

/// The Dreaming leave no body, only a little grave-wax where they came apart.
pub fn wax_yield(max_health: u32) -> u32 {
    (max_health * 2).clamp(8, 60)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Work {
    Study,
    Render,
}

impl World {
    pub fn corpse_here(&self) -> Option<&Corpse> {
        self.floor.corpses.iter().find(|c| c.at == self.player.pos)
    }

    /// Whether studying this kind would teach anything now.
    pub fn study_would_teach(&self, kind: KindId) -> bool {
        !self.studied.contains(&kind) || self.next_teaching(kind).is_some()
    }

    /// The first rite this kind teaches that you don't know yet.
    fn next_teaching(&self, kind: KindId) -> Option<crate::rites::RiteId> {
        self.content
            .monster(kind)
            .teaches
            .iter()
            .filter_map(|id| self.content.rite_by_id(id))
            .find(|&r| !self.knows(r))
    }

    /// Kinds you have studied this run.
    pub fn has_studied(&self, kind: KindId) -> bool {
        self.studied.contains(&kind)
    }

    pub(crate) fn study(&mut self) -> Vec<Event> {
        let Some(corpse) = self.corpse_here().copied() else {
            return vec![Event::NoCorpse];
        };
        if !self.study_would_teach(corpse.kind) {
            return vec![Event::NothingToLearn { kind: corpse.kind }];
        }
        let need = study_turns(self.content.monster(corpse.kind).threat);
        let (mut events, done) = self.work(Work::Study, need);
        if done {
            self.finish_study(corpse.kind, &mut events);
        }
        events
    }

    fn finish_study(&mut self, kind: KindId, events: &mut Vec<Event>) {
        let here = self.player.pos;
        if let Some(c) = self.floor.corpses.iter_mut().find(|c| c.at == here) {
            c.studied = 0;
        }
        let first = self.studied.insert(kind);
        events.push(Event::Studied { kind, first });
        if first {
            let traits = self.content.monster(kind).traits.clone();
            for t in traits {
                self.witness(kind, t);
            }
            self.gain_insight(INSIGHT_STUDY, events);
        }
        if let Some(rite) = self.next_teaching(kind) {
            self.learn_rite(rite, events);
        }
        self.stats.studies += 1;
        self.trigger(Trigger::Study, events);
    }

    pub(crate) fn render(&mut self) -> Vec<Event> {
        let Some(corpse) = self.corpse_here().copied() else {
            return vec![Event::NoCorpse];
        };
        // Starting on a fresh body: the smell carries, and things come to it.
        let mut smell = Vec::new();
        if corpse.rendered == 0 && self.hostiles_in_view().is_empty() {
            let here = self.player.pos;
            if self.make_noise(here, RENDER_SMELL, crate::environment::Noise::Fight) > 0 {
                smell.push(Event::RenderSmell);
            }
        }
        let (events, done) = self.work(Work::Render, RENDER_TURNS);
        let mut events = [smell, events].concat();
        if done {
            let here = self.player.pos;
            self.floor.corpses.retain(|c| c.at != here);
            let def = self.content.monster(corpse.kind);
            let tallow = self.thieving(render_yield(def.faction, def.health));
            events.push(Event::Rendered {
                kind: corpse.kind,
                tallow,
            });
            self.gain_tallow(tallow, &mut events);
        }
        events
    }

    /// Works on the corpse underfoot one turn at a time until `need` turns are
    /// done or something interrupts. Progress is kept on the corpse.
    fn work(&mut self, work: Work, need: u32) -> (Vec<Event>, bool) {
        if !self.hostiles_in_view().is_empty() {
            return (vec![Event::RunRefused], false);
        }
        let here = self.player.pos;
        let mut events = Vec::new();
        loop {
            let Some(corpse) = self.floor.corpses.iter_mut().find(|c| c.at == here) else {
                return (events, false);
            };
            let progress = match work {
                Work::Study => &mut corpse.studied,
                Work::Render => &mut corpse.rendered,
            };
            if *progress >= need {
                return (events, true);
            }
            *progress += 1;
            let kind = corpse.kind;
            let mut step = vec![Event::PlayerWaited];
            self.pass_time(&mut step);
            let disturbed = step.iter().any(|e| !crate::world::is_routine(e));
            events.extend(step);
            if self.death.is_some() {
                return (events, false);
            }
            if disturbed || !self.hostiles_in_view().is_empty() {
                events.push(Event::WorkInterrupted { kind });
                return (events, false);
            }
        }
    }

    /// Reads a text: a few quiet turns, then a rite of its school you don't know.
    pub(crate) fn read(&mut self, id: ItemId) -> Vec<Event> {
        let Some(item) = self.inventory_item(id).copied() else {
            return Vec::new();
        };
        let ItemClass::Text { school } = self.content.item(item.kind).class else {
            return Vec::new();
        };
        if !self.hostiles_in_view().is_empty() {
            return vec![Event::RunRefused];
        }
        let mut events = Vec::new();
        for _ in 0..READ_TURNS {
            let mut step = vec![Event::PlayerWaited];
            self.pass_time(&mut step);
            let disturbed = step.iter().any(|e| !crate::world::is_routine(e));
            events.extend(step);
            if self.death.is_some() {
                return events;
            }
            if disturbed || !self.hostiles_in_view().is_empty() {
                events.push(Event::ReadingInterrupted { kind: item.kind });
                return events;
            }
        }
        self.take_from_pack(id, 1);
        let rite = self.unknown_rite(school);
        events.push(Event::Read {
            kind: item.kind,
            learned: rite.is_some(),
        });
        match rite {
            Some(rite) => self.learn_rite(rite, &mut events),
            None => self.gain_insight(crate::rites::INSIGHT_OLD_TEXT, &mut events),
        }
        events
    }

    /// Leaves a body where a creature died, if its kind leaves one. The
    /// Dreaming leave grave-wax instead.
    pub(crate) fn leave_corpse(&mut self, kind: KindId, at: Point, events: &mut Vec<Event>) {
        let def = self.content.monster(kind);
        if def.faction == Faction::Dreaming && !def.corpse {
            let amount = wax_yield(def.health);
            match self.floor.tallow.iter_mut().find(|t| t.at == at) {
                Some(pile) => pile.amount += amount,
                None => self
                    .floor
                    .tallow
                    .push(crate::floor::Tallow::new(at, amount)),
            }
            if self.floor.is_visible(at) {
                events.push(Event::WaxLeft { kind, amount });
            }
        }
        if def.corpse {
            let died = self.turn();
            self.floor.corpses.retain(|c| c.at != at);
            self.floor.corpses.push(Corpse {
                at,
                kind,
                died,
                studied: 0,
                rendered: 0,
                ancient: false,
            });
        }
    }

    /// Bodies swell, then rot; big ones hatch flies. Called once per turn.
    pub(crate) fn tick_corpses(&mut self, events: &mut Vec<Event>) {
        let now = self.turn();
        let mut rotted = Vec::new();
        for c in self.floor.corpses.iter().filter(|c| !c.ancient) {
            let age = now.saturating_sub(c.died);
            if age == SWELL_TURNS && self.floor.is_visible(c.at) {
                events.push(Event::CorpseSwelling {
                    kind: c.kind,
                    at: c.at,
                });
            }
            if age >= ROT_TURNS {
                rotted.push(*c);
            }
        }
        self.floor
            .corpses
            .retain(|c| c.ancient || now.saturating_sub(c.died) < ROT_TURNS);
        for c in rotted {
            let big = self.content.monster(c.kind).health >= HATCH_HEALTH;
            let free = c.at != self.player.pos && self.floor.monster_at(c.at).is_none();
            let Some(flies) = self.content.kind_by_id("fly_swarm") else {
                continue;
            };
            if big && free {
                let mut swarm = Monster::new(flies, self.content.monster(flies), c.at);
                swarm.mind = Mind::Unaware;
                self.floor.monsters.insert(swarm);
                if self.floor.is_visible(c.at) {
                    events.push(Event::CorpseHatched {
                        kind: c.kind,
                        at: c.at,
                    });
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
    use crate::geom::Direction;
    use crate::map::prefab;

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

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    /// Kills a creature of `id` beside the player and steps onto its body.
    fn body(world: &mut World, id: &str) {
        let east = world.player().pos + Direction::E;
        let m = world.spawn_monster(kind(id), east);
        let mut events = Vec::new();
        world.damage_monster(m, 999, crate::progress::Source::Rite, &mut events);
        world.apply(Command::Move(Direction::E));
    }

    #[test]
    fn the_taken_leave_bodies_and_the_dreaming_do_not() {
        let mut world = hall();
        body(&mut world, "parishioner");
        assert!(world.corpse_here().is_some());
        let mut world = hall();
        body(&mut world, "lantern_eater");
        assert!(world.corpse_here().is_none());
    }

    #[test]
    fn studying_reveals_traits_and_teaches_its_rite() {
        let mut world = hall();
        body(&mut world, "pallbearer");
        let events = world.apply(Command::Study);
        let kind = kind("pallbearer");
        assert!(events.contains(&Event::Studied { kind, first: true }));
        assert!(world.has_studied(kind));
        let kneel = Content::bundled().rite_by_id("kneel").unwrap();
        assert!(world.knows(kneel), "pallbearers teach Kneel");
        assert!(world.turn() >= u64::from(study_turns(4)));
        let again = world.apply(Command::Study);
        assert_eq!(again, vec![Event::NothingToLearn { kind }]);
    }

    #[test]
    fn interrupted_study_keeps_its_progress() {
        let mut world = hall();
        body(&mut world, "parishioner");
        let walker = world.spawn_monster(kind("parishioner"), Point::new(14, 2));
        world.floor.monsters[walker].mind = Mind::Hunting {
            last_seen: world.player().pos,
        };
        let events = world.apply(Command::Study);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::WorkInterrupted { .. })),
            "the parishioner comes into view"
        );
        let done = world.corpse_here().unwrap().studied;
        assert!(done > 0);
    }

    #[test]
    fn rendering_trades_the_body_for_tallow() {
        let mut world = hall();
        body(&mut world, "parishioner");
        let before = world.player().candle.tallow();
        let events = world.apply(Command::Render);
        assert!(
            events.contains(&Event::Rendered {
                kind: kind("parishioner"),
                tallow: 60
            }),
            "the Taken render rich"
        );
        assert!(world.corpse_here().is_none());
        assert_eq!(world.player().candle.tallow(), before + 60 - RENDER_TURNS);
    }

    #[test]
    fn big_bodies_swell_then_hatch_flies() {
        let mut world = hall();
        body(&mut world, "parishioner");
        world.apply(Command::Move(Direction::E));
        let mut events = Vec::new();
        for _ in 0..ROT_TURNS {
            events.extend(world.apply(Command::Wait));
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::CorpseSwelling { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::CorpseHatched { .. }))
        );
        assert!(
            world
                .floor()
                .monsters()
                .any(|(_, m)| m.kind == kind("fly_swarm"))
        );
    }

    #[test]
    fn small_bodies_just_rot() {
        let mut world = hall();
        body(&mut world, "gnawer");
        world.apply(Command::Move(Direction::E));
        for _ in 0..ROT_TURNS + 1 {
            world.apply(Command::Wait);
        }
        assert!(world.floor().corpses().is_empty());
        assert_eq!(world.floor().monsters().count(), 0);
    }

    #[test]
    fn reading_a_page_teaches_a_rite_of_its_school() {
        let mut world = hall();
        let note = Content::bundled().item_by_id("heretic_note").unwrap();
        world.place_item(world.player().pos, note, 1);
        world.apply(Command::PickUp);
        let id = world
            .player()
            .inventory
            .iter()
            .find(|it| it.kind == note)
            .unwrap()
            .id;
        let events = world.apply(Command::Use(id));
        assert!(events.contains(&Event::Read {
            kind: note,
            learned: true
        }));
        let rite = world.known_rites()[0];
        assert_eq!(
            Content::bundled().rite(rite).school,
            crate::rites::School::Binding
        );
        assert!(world.inventory_item(id).is_none(), "the page is used up");
    }
}
