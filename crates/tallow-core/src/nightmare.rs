//! What happens every turn: the candle burns, dread moves, and the dark
//! answers with whispers, phantoms and finally a Manifestation.

use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::candle::BurnWarning;
use crate::dread::{self, DreadBand};
use crate::events::Event;
use crate::geom::Point;
use crate::map::path;
use crate::monster::{Mind, Monster};
use crate::time::REGEN_TURNS;
use crate::world::World;

/// One in this many turns, the uneasy hear something.
const WHISPER_ODDS: u32 = 45;
/// One in this many turns, the frayed see something.
const PHANTOM_ODDS: u32 = 30;
const MAX_PHANTOMS: usize = 2;
/// Steps from the player where phantoms and Manifestations appear.
const LURK_DISTANCE: (u32, u32) = (6, 14);

impl World {
    pub(crate) fn on_turn(&mut self, events: &mut Vec<Event>) {
        if self.turn().is_multiple_of(REGEN_TURNS) {
            self.player.health = (self.player.health + 1).min(self.player.max_health);
        }

        match self.player.candle.burn() {
            Some(BurnWarning::Low) => events.push(Event::CandleLow),
            Some(BurnWarning::Guttering) => events.push(Event::CandleGuttering),
            Some(BurnWarning::BurnedOut) => events.push(Event::CandleBurnedOut),
            None => {}
        }

        // While your dread walks the floor, it holds you at the top.
        if !self.manifestation_present() {
            let rate = if self.floor.ambient_light(self.player.pos).is_lit() {
                dread::PER_TURN_BY_BRAZIER
            } else if self.player.candle.is_lit() {
                dread::PER_TURN_IN_CANDLELIGHT
            } else {
                dread::PER_TURN_IN_DARKNESS
            };
            self.shift_dread(rate, events);
        }

        let band = self.player.dread.band();
        if band >= DreadBand::Uneasy && self.ai_rng.random_ratio(1, WHISPER_ODDS) {
            events.push(Event::Whisper {
                seed: self.ai_rng.random(),
            });
        }
        if band < DreadBand::Frayed {
            self.clear_phantoms(events);
        } else if band == DreadBand::Frayed
            && self.floor.monsters().filter(|(_, m)| m.phantom).count() < MAX_PHANTOMS
            && self.ai_rng.random_ratio(1, PHANTOM_ODDS)
        {
            self.spawn_phantom();
        }
        if band == DreadBand::Manifest && !self.manifestation_present() {
            self.spawn_manifestation(events);
        }
    }

    pub(crate) fn manifestation_present(&self) -> bool {
        self.floor
            .monsters()
            .any(|(_, m)| m.kind == self.manifestation)
    }

    pub(crate) fn clear_phantoms(&mut self, events: &mut Vec<Event>) {
        let phantoms: Vec<_> = self
            .floor
            .monsters()
            .filter(|(_, m)| m.phantom)
            .map(|(id, _)| id)
            .collect();
        for id in phantoms {
            let monster = self.floor.monsters.remove(id).expect("listed above");
            if self.floor.is_visible(monster.pos) {
                events.push(Event::PhantomFaded {
                    kind: monster.kind,
                    struck: false,
                });
            }
        }
    }

    /// A false creature, shaped like one you've already met, coming for you.
    fn spawn_phantom(&mut self) {
        let mut kinds: Vec<_> = self
            .sighted_kinds()
            .filter(|&k| k != self.manifestation && self.content.monster(k).natural)
            .collect();
        kinds.sort();
        let Some(&kind) = kinds.choose(&mut self.ai_rng) else {
            return;
        };
        let Some(at) = self.lurking_spot() else {
            return;
        };
        let mut phantom = Monster::new(kind, self.content.monster(kind), at);
        phantom.phantom = true;
        phantom.mind = Mind::Hunting {
            last_seen: self.player.pos,
        };
        self.floor.monsters.insert(phantom);
    }

    fn spawn_manifestation(&mut self, events: &mut Vec<Event>) {
        let kind = self.manifestation;
        let Some(at) = self.lurking_spot() else {
            return;
        };
        let mut monster = Monster::new(kind, self.content.monster(kind), at);
        monster.mind = Mind::Hunting {
            last_seen: self.player.pos,
        };
        self.floor.monsters.insert(monster);
        events.push(Event::Manifested);
    }

    /// A free tile out of sight, a short walk from the player. Falls back to
    /// any free tile out of sight, then any free tile at all.
    fn lurking_spot(&mut self) -> Option<Point> {
        let dist = path::distances(self.map(), self.player.pos);
        let free = |world: &World, p: Point| {
            world.map().is_walkable(p)
                && p != world.player.pos
                && world.floor.monster_at(p).is_none()
        };
        let all: Vec<(Point, u32)> = self
            .map()
            .points()
            .filter(|&p| free(self, p))
            .filter_map(|p| dist.at(p).map(|d| (p, d)))
            .collect();
        let hidden: Vec<(Point, u32)> = all
            .iter()
            .copied()
            .filter(|&(p, _)| !self.floor.is_visible(p))
            .collect();
        let near: Vec<Point> = hidden
            .iter()
            .filter(|&&(_, d)| (LURK_DISTANCE.0..=LURK_DISTANCE.1).contains(&d))
            .map(|&(p, _)| p)
            .collect();
        let pool: Vec<Point> = if !near.is_empty() {
            near
        } else if !hidden.is_empty() {
            hidden.into_iter().map(|(p, _)| p).collect()
        } else {
            all.into_iter().map(|(p, _)| p).collect()
        };
        pool.choose(&mut self.ai_rng).copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Command;
    use crate::candle::{self, CandleState};
    use crate::content::{Content, KindId, Trait};
    use crate::dread::{self, DreadBand};
    use crate::events::Event;
    use crate::geom::{Direction, Point};
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

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    fn wait(world: &mut World, turns: u32) -> Vec<Event> {
        (0..turns)
            .flat_map(|_| world.apply(Command::Wait))
            .collect()
    }

    fn set_tallow(world: &mut World, amount: u32) {
        let candle = &mut world.player.candle;
        candle.eat(u32::MAX);
        candle.add(amount);
        candle.light();
    }

    #[test]
    fn the_candle_burns_one_tallow_per_turn_unless_snuffed() {
        let mut world = hall();
        wait(&mut world, 10);
        assert_eq!(world.player().candle.tallow(), candle::START_TALLOW - 10);

        let events = world.apply(Command::ToggleCandle);
        assert!(events.contains(&Event::CandleSnuffed));
        assert_eq!(world.turn(), 11, "snuffing takes a turn");
        let before = world.player().candle.tallow();
        wait(&mut world, 10);
        assert_eq!(
            world.player().candle.tallow(),
            before,
            "a snuffed candle saves tallow"
        );
        assert!(
            world
                .apply(Command::ToggleCandle)
                .contains(&Event::CandleLit)
        );
    }

    #[test]
    fn snuffing_darkens_the_world() {
        let mut world = hall();
        assert!(world.floor().is_visible(Point::new(5, 1)));
        world.apply(Command::ToggleCandle);
        assert!(!world.floor().is_visible(Point::new(5, 1)));
        assert!(
            world.floor().is_visible(Point::new(2, 1)),
            "you can still feel what's beside you"
        );
    }

    #[test]
    fn walking_over_tallow_takes_it_and_runs_stop_when_it_comes_into_view() {
        let mut world = hall();
        world.place_tallow(Point::new(20, 1), 120);
        let events = world.apply(Command::Run(Direction::E));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::SpottedTallow { .. }))
        );
        assert!(world.player().pos.x < 20, "stopped when it came into view");

        let before = world.player().candle.tallow();
        while world.player().pos.x < 20 {
            world.apply(Command::Move(Direction::E));
        }
        assert!(world.floor().tallow().is_empty());
        assert!(world.player().candle.tallow() > before + 100);
    }

    #[test]
    fn a_burning_candle_warns_then_gutters_then_goes_out() {
        let mut world = hall();
        set_tallow(&mut world, candle::LOW_AT + 1);
        let events = wait(&mut world, candle::LOW_AT + 2);
        let warnings: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::CandleLow | Event::CandleGuttering | Event::CandleBurnedOut
                )
            })
            .collect();
        assert_eq!(
            warnings,
            vec![
                &Event::CandleLow,
                &Event::CandleGuttering,
                &Event::CandleBurnedOut
            ]
        );
        assert_eq!(world.player().candle.state(), CandleState::Out);
        assert_eq!(world.apply(Command::ToggleCandle), vec![Event::CandleSpent]);
    }

    #[test]
    fn guttering_shrinks_what_you_can_see() {
        let mut world = hall();
        set_tallow(&mut world, candle::GUTTER_AT - 5);
        world.apply(Command::Wait);
        assert!(!world.floor().is_visible(Point::new(1 + candle::RADIUS, 1)));
        assert!(
            world
                .floor()
                .is_visible(Point::new(1 + candle::GUTTER_RADIUS, 1))
        );
    }

    #[test]
    fn darkness_breeds_dread_faster_than_candlelight_and_braziers_ease_it() {
        let mut lit = hall();
        wait(&mut lit, 40);
        let mut dark = hall();
        dark.apply(Command::ToggleCandle);
        wait(&mut dark, 40);
        assert_eq!(lit.player().dread.value(), 2);
        assert!(dark.player().dread.value() >= 9);

        let mut warm = world_from("#######\n#@.&..#\n#######");
        warm.player.dread.set(30);
        wait(&mut warm, 30);
        assert!(warm.player().dread.value() < 30);
    }

    #[test]
    fn first_sight_of_a_horror_is_a_shock() {
        let mut world = hall();
        world.spawn_monster(kind("lantern_eater"), Point::new(4, 2));
        world.apply(Command::Wait);
        assert!(world.player().dread.value() >= (dread::FIRST_SIGHT_OF_DREAMING / 100) as u32);
    }

    #[test]
    fn full_dread_manifests_and_killing_it_eases_dread() {
        let mut world = hall();
        world.player.dread.set(99);
        world.player.candle.snuff();
        let mut events = Vec::new();
        for _ in 0..20 {
            events.extend(world.apply(Command::Wait));
            if world.manifestation_present() {
                break;
            }
        }
        assert!(events.contains(&Event::Manifested));
        assert_eq!(world.player().dread.band(), DreadBand::Manifest);

        // It's held at the top while it lives.
        let (id, _) = world
            .floor()
            .monsters()
            .find(|(_, m)| m.kind == world.manifestation)
            .unwrap();
        world.floor.monsters[id].health = 1;
        world.floor.monsters[id].pos = world.player().pos + Direction::E;
        world.floor.monsters[id].energy = -1000;
        world.combat_rng = crate::rng::stream(1, crate::rng::Stream::Combat);
        let mut events = Vec::new();
        for _ in 0..30 {
            if !world.manifestation_present() {
                break;
            }
            let at = world.floor().monster(id).map(|m| m.pos).unwrap();
            let dir = Direction::ALL
                .into_iter()
                .find(|&d| world.player().pos + d == at)
                .unwrap();
            events.extend(world.apply(Command::Move(dir)));
        }
        assert!(events.contains(&Event::ManifestationBanished));
        assert_eq!(world.player().dread.value(), dread::AFTER_MANIFESTATION);
    }

    #[test]
    fn escaping_the_floor_also_eases_dread() {
        let mut world = world_from("######\n#@>..#\n######");
        world.player.dread.set(100);
        world.apply(Command::Wait);
        assert!(world.manifestation_present() || world.death().is_some());
        world.apply(Command::Move(Direction::E));
        let events = world.apply(Command::Descend);
        assert!(events.contains(&Event::ManifestationEscaped));
        assert_eq!(world.player().dread.value(), dread::AFTER_MANIFESTATION);
    }

    #[test]
    fn phantoms_haunt_the_frayed_but_never_hurt_and_look_sees_through_them() {
        let mut world = hall();
        // Phantoms take the shape of something already seen.
        world.spawn_monster(kind("parishioner"), Point::new(3, 2));
        world.apply(Command::Wait);
        world.despawn_all();

        world.player.dread.set(85);
        let health = world.player().health;
        let mut saw_phantom = false;
        for _ in 0..400 {
            world.player.dread.set(85);
            world.apply(Command::Wait);
            for id in world.floor().visible_monsters(world.player().pos) {
                let info = world.inspect(id).unwrap();
                assert!(info.phantom, "only phantoms here");
                saw_phantom = true;
            }
        }
        assert!(
            saw_phantom,
            "a phantom should appear within 400 frayed turns"
        );
        assert!(world.player().health >= health, "phantoms never hurt");

        world.player.dread.set(20);
        world.apply(Command::Wait);
        assert!(
            world.floor().monsters().all(|(_, m)| !m.phantom),
            "calm clears them"
        );
    }

    #[test]
    fn a_lantern_eaters_bite_snuffs_the_candle() {
        let mut world = hall();
        world.spawn_monster(kind("lantern_eater"), Point::new(2, 1));
        world.player.health = 999;
        world.player.max_health = 999;
        let mut events = Vec::new();
        for _ in 0..60 {
            if events
                .iter()
                .any(|e| matches!(e, Event::CandleEaten { .. }))
            {
                break;
            }
            events.extend(world.apply(Command::Wait));
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::CandleEaten { .. }))
        );
        let (id, _) = world.floor().monsters().next().unwrap();
        world.apply(Command::ToggleCandle);
        let info = world.inspect(id).unwrap();
        assert!(info.known_traits.contains(&Trait::EatsLight));
    }

    #[test]
    fn with_the_candle_out_you_go_unnoticed_unless_something_is_close() {
        let mut dark = hall();
        dark.apply(Command::ToggleCandle);
        dark.spawn_monster(kind("parishioner"), Point::new(6, 1));
        let events = wait(&mut dark, 10);
        assert!(events.iter().all(|e| !matches!(e, Event::Noticed { .. })));

        let mut lit = hall();
        lit.spawn_monster(kind("parishioner"), Point::new(6, 1));
        let events = wait(&mut lit, 10);
        assert!(events.iter().any(|e| matches!(e, Event::Noticed { .. })));
    }

    #[test]
    fn rest_heals_and_respects_company() {
        let mut world = hall();
        assert_eq!(world.apply(Command::Rest), vec![Event::NothingToRest]);
        world.player.health = 10;
        let events = world.apply(Command::Rest);
        assert!(events.iter().any(|e| matches!(e, Event::Rested { .. })));
        assert_eq!(world.player().health, world.player().max_health);

        world.player.health = 10;
        world.spawn_monster(kind("parishioner"), Point::new(4, 1));
        assert_eq!(world.apply(Command::Rest), vec![Event::RunRefused]);
    }

    #[test]
    fn the_uneasy_hear_whispers() {
        let mut world = hall();
        let mut heard = false;
        for _ in 0..400 {
            world.player.dread.set(50);
            if world
                .apply(Command::Wait)
                .iter()
                .any(|e| matches!(e, Event::Whisper { .. }))
            {
                heard = true;
                break;
            }
        }
        assert!(heard);
    }
}
