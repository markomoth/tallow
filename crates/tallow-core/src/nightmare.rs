//! What happens every turn: the candle burns, dread moves, and the dark
//! answers with whispers, phantoms and finally a Manifestation.

use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::boons::Passive;
use crate::candle::BurnWarning;
use crate::dread::{self, DreadBand};
use crate::events::Event;
use crate::geom::Point;
use crate::map::path;
use crate::monster::{Mind, Monster};
use crate::time::REGEN_TURNS;

/// Regeneration interval with the quick-mending boon.
const QUICK_REGEN_TURNS: u64 = 8;
use crate::world::World;

/// One in this many turns, the uneasy hear something.
const WHISPER_ODDS: u32 = 45;
/// One in this many turns, the frayed see something.
const PHANTOM_ODDS: u32 = 30;
const MAX_PHANTOMS: usize = 2;
/// Every this many turns your candle burns on a floor, the light draws one
/// more creature onto it, up to `MAX_DRAWN`. Darkness draws nothing.
pub const LIGHT_DRAW_TURNS: u32 = 120;
const MAX_DRAWN: u32 = 3;
/// A drawn creature arrives at least this many steps away, out of sight.
const DRAWN_DISTANCE: u32 = 15;
/// Steps from the player where phantoms and Manifestations appear.
const LURK_DISTANCE: (u32, u32) = (6, 14);
/// Every this many turns frayed or worse, a nightmare comes for you...
pub const NIGHTMARE_TURNS: u32 = 50;
/// ...announced this many turns ahead...
pub const NIGHTMARE_WARNING: u64 = 5;
/// ...and no more than this many at once.
const MAX_NIGHTMARES: usize = 2;
/// A phantom that reaches you hurts this much, or else adds this much dread.
const PHANTOM_BITE: (u32, u32) = (1, 2);
const PHANTOM_DREAD: i32 = 500;

impl World {
    /// A lit candle is seen from far off. Burn it long enough on one floor and
    /// something new comes looking, and says so.
    fn tick_light_drawing(&mut self, events: &mut Vec<Event>) {
        let showing = self.player.candle.is_lit() && !self.shrouded() && !self.player.vigil;
        if !showing
            || self.stage() != crate::throne::Stage::Descent
            || self.depth() >= crate::MAX_DEPTH
        {
            return;
        }
        self.floor.lit_turns += 1;
        if !self.floor.lit_turns.is_multiple_of(LIGHT_DRAW_TURNS) || self.floor.drawn >= MAX_DRAWN {
            return;
        }
        let here = self.player.pos;
        let dist = path::distances(self.map(), here);
        let spots: Vec<Point> = self
            .map()
            .points()
            .filter(|&p| {
                self.map().tile(p) == crate::map::Tile::Floor
                    && !self.floor.in_sight(p)
                    && self.floor.monster_at(p).is_none()
                    && dist.at(p).is_some_and(|d| d >= DRAWN_DISTANCE)
            })
            .collect();
        let kinds = crate::spawn::eligible(self.content, self.depth());
        let content = self.content;
        let (Some(&at), Ok(&kind)) = (
            spots.choose(&mut self.ai_rng),
            kinds.choose_weighted(&mut self.ai_rng, |&k| content.monster(k).weight),
        ) else {
            return;
        };
        let id = self
            .floor
            .monsters
            .insert(Monster::new(kind, content.monster(kind), at));
        self.floor.monsters[id].mind = Mind::Hunting { last_seen: here };
        self.floor.drawn += 1;
        events.push(Event::LightDrawn);
    }

    pub(crate) fn on_turn(&mut self, events: &mut Vec<Event>) {
        let regen = if self.has_passive(Passive::QuickMending) {
            QUICK_REGEN_TURNS
        } else {
            REGEN_TURNS
        };
        if self.turn().is_multiple_of(regen) {
            self.player.health = (self.player.health + 1).min(self.player.max_health);
        }
        if self
            .player
            .blind_until
            .is_some_and(|until| self.turn() >= until)
        {
            self.player.blind_until = None;
            events.push(Event::SightReturns);
        }
        self.tick_rites(events);
        self.tick_leavings(events);
        self.tick_throne(events);
        self.tick_corpses(events);
        self.tick_fire(events);
        self.tick_seals(events);

        // A thrifty candle skips every fourth turn of burning.
        // The Vigil Candle needs no tallow.
        let thrifty = (self.has_passive(Passive::CandleThrift) && self.turn().is_multiple_of(4))
            || self.player.vigil;
        match if thrifty {
            None
        } else {
            self.player.candle.burn()
        } {
            Some(BurnWarning::Low) => events.push(Event::CandleLow),
            Some(BurnWarning::Guttering) => events.push(Event::CandleGuttering),
            Some(BurnWarning::BurnedOut) => {
                events.push(Event::CandleBurnedOut);
                self.wake_leavings(crate::leavings::Wake::EnteringDarkness, events);
            }
            None => {}
        }

        self.tick_light_drawing(events);

        // While your dread walks the floor, it holds you at the top.
        if !self.manifestation_present() {
            let lit =
                self.player.candle.is_lit() || self.floor.ambient_light(self.player.pos).is_lit();
            if !lit {
                let rate = if self.has_passive(Passive::DarkFed) {
                    dread::PER_TURN_IN_DARKNESS_FED
                } else {
                    dread::PER_TURN_IN_DARKNESS
                };
                self.shift_dread(rate, events);
            }
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
        self.tick_nightmares(band, events);
    }

    pub(crate) fn manifestation_present(&self) -> bool {
        self.floor
            .monsters()
            .any(|(_, m)| m.kind == self.manifestation && !m.phantom)
    }

    /// A Nightmare has been announced and is on its way. When it arrives is
    /// not said: only that it is near.
    pub fn nightmare_coming(&self) -> bool {
        self.nightmare_at.is_some()
    }

    /// Nightmares on this floor.
    pub fn nightmares(&self) -> Vec<crate::monster::MonsterId> {
        self.floor
            .monsters()
            .filter(|(_, m)| {
                !m.phantom
                    && self
                        .content
                        .monster(m.kind)
                        .has(|t| *t == crate::content::Trait::Nightmare)
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Frayed, real things come for you; calm, they come apart.
    fn tick_nightmares(&mut self, band: DreadBand, events: &mut Vec<Event>) {
        if band == DreadBand::Calm {
            for id in self.nightmares() {
                let m = self.floor.monsters.remove(id).expect("listed above");
                events.push(Event::NightmareFades { kind: m.kind });
            }
        }
        if band < DreadBand::Frayed || self.stage() == crate::throne::Stage::Church {
            self.frayed_turns = 0;
            if self.nightmare_at.take().is_some() {
                events.push(Event::NightmareTurnedAway);
            }
            return;
        }
        // While your Manifestation walks, it is enough.
        if self.manifestation_present() {
            return;
        }
        if self.nightmare_at.is_some_and(|at| self.turn() >= at) {
            self.nightmare_at = None;
            self.spawn_nightmare(events);
        }
        self.frayed_turns += 1;
        if self.frayed_turns.is_multiple_of(NIGHTMARE_TURNS)
            && self.nightmare_at.is_none()
            && self.nightmares().len() < MAX_NIGHTMARES
        {
            self.nightmare_at = Some(self.turn() + NIGHTMARE_WARNING);
            events.push(Event::NightmareComing);
        }
    }

    /// The nightmare for this depth, out of sight and already hunting you.
    fn spawn_nightmare(&mut self, events: &mut Vec<Event>) {
        let depth = self.depth();
        let content = self.content;
        let mut kinds: Vec<_> = content
            .kinds()
            .filter(|(_, d)| d.has(|t| *t == crate::content::Trait::Nightmare))
            .collect();
        kinds.sort_by_key(|&(k, _)| k);
        let here: Vec<_> = kinds
            .iter()
            .filter(|(_, d)| d.depth.0 <= depth && depth <= d.depth.1)
            .map(|&(k, _)| k)
            .collect();
        let Some(&kind) = here.first().or_else(|| kinds.last().map(|(k, _)| k)) else {
            return;
        };
        let Some(at) = self.lurking_spot() else {
            return;
        };
        let mut monster = Monster::new(kind, content.monster(kind), at);
        monster.mind = Mind::Hunting {
            last_seen: self.player.pos,
        };
        self.floor.monsters.insert(monster);
        events.push(Event::NightmareArrives { kind });
    }

    /// A phantom reached you: it comes apart, but fear still bites.
    pub(crate) fn phantom_bite(&mut self, kind: crate::content::KindId, events: &mut Vec<Event>) {
        let damage = if self.ai_rng.random_bool(0.5) {
            let roll = crate::combat::roll_damage(&mut self.ai_rng, PHANTOM_BITE);
            // Fear never takes your last breath.
            Some(roll.min(self.player.health.saturating_sub(1))).filter(|&d| d > 0)
        } else {
            None
        };
        events.push(Event::PhantomBit { kind, damage });
        match damage {
            Some(damage) => {
                self.hurt_player(damage, crate::events::Cause::Attack(kind), events);
            }
            None => self.shift_dread(PHANTOM_DREAD, events),
        }
    }

    /// A false copy of `kind` beside `at`, hunting you.
    pub(crate) fn spawn_phantom_at(&mut self, kind: crate::content::KindId, at: Point) {
        let mut phantom = Monster::new(kind, self.content.monster(kind), at);
        phantom.phantom = true;
        phantom.mind = Mind::Hunting {
            last_seen: self.player.pos,
        };
        self.floor.monsters.insert(phantom);
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
        if let Some(at) = self.lurking_spot() {
            self.spawn_phantom_at(kind, at);
        }
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
        // Each time it comes back, it has learned more of you.
        let grown = self.manifestations;
        monster.grown = grown;
        monster.health += grown * crate::world::GROWN_HEALTH;
        monster.max_health = monster.health;
        self.manifestations += 1;
        self.floor.monsters.insert(monster);
        events.push(Event::Manifested { grown });
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
    use super::{LIGHT_DRAW_TURNS, NIGHTMARE_TURNS, NIGHTMARE_WARNING};
    use crate::actions::Command;
    use crate::candle::{self, CandleState};
    use crate::content::{Content, KindId, Trait};
    use crate::dread::{self, DreadBand};
    use crate::events::Event;
    use crate::geom::{Direction, Point};
    use crate::map::prefab;
    use crate::monster::Mind;
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
    fn only_darkness_breeds_dread() {
        let mut lit = hall();
        wait(&mut lit, 40);
        let mut dark = hall();
        dark.apply(Command::ToggleCandle);
        wait(&mut dark, 40);
        assert_eq!(lit.player().dread.value(), 0);
        assert!(dark.player().dread.value() >= 9);

        // Brazier light is light: no dread, but none taken away either.
        let mut warm = world_from("#######\n#@.&..#\n#######");
        warm.apply(Command::ToggleCandle);
        warm.player.dread.set(30);
        wait(&mut warm, 30);
        assert_eq!(warm.player().dread.value(), 30);
    }

    #[test]
    fn a_long_burning_candle_draws_creatures_and_says_so() {
        let mut world = World::new(3);
        world.despawn_all();
        let mut drawn = 0;
        for _ in 0..LIGHT_DRAW_TURNS {
            drawn += world
                .apply(Command::Wait)
                .iter()
                .filter(|e| **e == Event::LightDrawn)
                .count();
        }
        assert_eq!(drawn, 1);
        let (_, m) = world.floor().monsters().next().expect("something came");
        assert!(matches!(m.mind, Mind::Hunting { .. }));
        assert!(!world.floor().in_sight(m.pos), "it arrives out of sight");

        // Snuffed, the dark draws nothing.
        let mut world = World::new(3);
        world.despawn_all();
        world.apply(Command::ToggleCandle);
        for _ in 0..LIGHT_DRAW_TURNS * 2 {
            assert!(!world.apply(Command::Wait).contains(&Event::LightDrawn));
        }
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
        assert!(events.contains(&Event::Manifested { grown: 0 }));
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
    fn phantoms_haunt_the_frayed_bite_but_never_kill_and_look_sees_through_them() {
        let mut world = hall();
        // Phantoms take the shape of something already seen.
        world.spawn_monster(kind("parishioner"), Point::new(3, 2));
        world.apply(Command::Wait);
        world.despawn_all();

        world.player.health = 2;
        let mut saw_phantom = false;
        let mut bites = 0;
        for _ in 0..400 {
            world.player.dread.set(85);
            let events = world.apply(Command::Wait);
            // Only the phantoms: real nightmares are tested on their own.
            world.floor.monsters.retain(|_, m| m.phantom);
            bites += events
                .iter()
                .filter(|e| matches!(e, Event::PhantomBit { .. }))
                .count();
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
        assert!(bites > 0, "phantoms that reach you bite");
        assert!(world.death().is_none(), "fear never takes your last breath");

        world.player.dread.set(20);
        world.apply(Command::Wait);
        assert!(
            world.floor().monsters().all(|(_, m)| !m.phantom),
            "calm clears them"
        );
    }

    #[test]
    fn nightmares_come_for_the_frayed_and_come_apart_when_calm() {
        let mut world = hall();
        world.player.health = 999;
        world.player.max_health = 999;
        let mut events = Vec::new();
        for _ in 0..NIGHTMARE_TURNS + NIGHTMARE_WARNING as u32 + 1 {
            world.player.dread.set(75);
            events.extend(world.apply(Command::Wait));
        }
        let coming = events
            .iter()
            .position(|e| *e == Event::NightmareComing)
            .expect("announced");
        let arrived = events
            .iter()
            .position(|e| matches!(e, Event::NightmareArrives { .. }))
            .expect("arrived");
        assert!(coming < arrived, "always announced first");
        assert_eq!(world.nightmares().len(), 1);
        let id = world.nightmares()[0];
        assert_eq!(world.floor().monster(id).unwrap().kind, kind("mare"));

        world.player.dread.set(30);
        let events = world.apply(Command::Wait);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::NightmareFades { .. }))
        );
        assert!(world.nightmares().is_empty());
    }

    #[test]
    fn calming_down_turns_a_nightmare_away() {
        let mut world = hall();
        let mut events = Vec::new();
        for _ in 0..NIGHTMARE_TURNS {
            world.player.dread.set(75);
            events.extend(world.apply(Command::Wait));
        }
        assert!(events.contains(&Event::NightmareComing));
        assert!(world.nightmare_coming(), "the sidebar can say so");
        world.player.dread.set(60);
        assert!(
            world
                .apply(Command::Wait)
                .contains(&Event::NightmareTurnedAway)
        );
        assert!(!world.nightmare_coming());
        for _ in 0..NIGHTMARE_WARNING + 2 {
            world.player.dread.set(60);
            world.apply(Command::Wait);
        }
        assert!(world.nightmares().is_empty());
    }

    #[test]
    fn the_manifestation_snuffs_doubles_and_grows() {
        let mut world = hall();
        world.player.health = 999;
        world.player.max_health = 999;
        world.manifestations = 2;
        world.player.dread.set(100);
        let events = world.apply(Command::Wait);
        assert!(events.contains(&Event::Manifested { grown: 2 }));
        let (id, m) = world
            .floor()
            .monsters()
            .find(|(_, m)| m.kind == world.manifestation)
            .unwrap();
        assert_eq!(m.max_health, 18 + 2 * crate::world::GROWN_HEALTH);
        world.floor.monsters[id].pos = Point::new(4, 1);
        world.apply(Command::ToggleCandle);
        world.apply(Command::ToggleCandle);
        assert!(world.player().candle.is_lit());
        let mut events = Vec::new();
        for _ in 0..12 {
            events.extend(world.apply(Command::Wait));
        }
        assert!(events.contains(&Event::CandleSnuffedBy {
            kind: world.manifestation
        }));
        assert!(events.contains(&Event::Doubled {
            kind: world.manifestation
        }));
        let info = world.inspect(id).unwrap();
        let (lo, _) = Content::bundled().monster(world.manifestation).damage;
        assert!(info.its_damage.0 >= lo + 2, "grown hits harder");
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
