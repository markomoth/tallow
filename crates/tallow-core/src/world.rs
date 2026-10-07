//! The world: the run seed, the current floor, the player, and time.

use std::collections::HashSet;

use crate::actions::Command;
use crate::biome::MAX_DEPTH;
use crate::candle::{self, Candle};
use std::collections::HashMap;

use crate::boons::{Boon, Passive, Trigger};
use crate::combat::{self, PLAYER_HEALTH};
use crate::content::{Content, Faction, KindId, Trait};
use crate::dread::{self, Dread};
use crate::events::{Cause, Event, Who};
use crate::floor::{Floor, FloorItem, Tallow};
use crate::geom::{Direction, Point};
use crate::inventory;
use crate::item::{Burden, Equipment, Family, Item, ItemKindId, TinctureLore};
use crate::map::light::LightSource;
use crate::map::{Map, Tile, generate};
use crate::monster::{Mind, Monster, MonsterId};
use crate::progress::Source;
use crate::rng::{self, GameRng, Stream};
use crate::skills::{Skill, Skills, Technique};
use crate::spawn;
use crate::time::{ACTION_COST, PLAYER_SPEED, TICKS_PER_TURN};
use std::collections::VecDeque;

/// Safety stop for runs across very long halls.
const MAX_RUN_STEPS: u32 = 120;
/// Safety stop for one rest.
const MAX_REST_TURNS: u32 = 200;
/// Energy a step costs while burdened.
const BURDENED_MOVE: i32 = ACTION_COST * 3 / 2;

#[derive(Debug, Clone)]
pub struct Player {
    pub pos: Point,
    pub health: u32,
    pub max_health: u32,
    pub candle: Candle,
    pub dread: Dread,
    pub inventory: Vec<Item>,
    pub equipment: Equipment,
    pub level: u32,
    /// Toward the next level.
    pub insight: u32,
    pub skills: Skills,
    pub boons: Vec<Boon>,
    /// Rites known, in the order learned.
    pub rites: Vec<crate::rites::RiteId>,
    /// Rites lasting on the acolyte (Shroud, Borrowed Eyes).
    pub rite_state: crate::rites::Rites,
    pub(crate) energy: i32,
}

/// Counts that decide which boons can be offered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    pub snuffs: u32,
    pub drinks: u32,
    pub heavy_blows_seen: u32,
    pub casts: u32,
    pub studies: u32,
    /// Taken freed by Exorcise.
    pub exorcised: u32,
}

/// How and when a run ended in death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Death {
    pub cause: Cause,
    pub depth: u8,
    pub turn: u64,
}

/// What Look shows about a visible monster. Every number here is the real one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterInfo {
    pub kind: KindId,
    pub pos: Point,
    pub health: u32,
    pub max_health: u32,
    pub mind: Mind,
    /// Percent chance your blow lands on it.
    pub your_hit_chance: u32,
    /// Percent chance its ordinary blow lands on you, and for how much.
    pub its_hit_chance: u32,
    pub its_damage: (u32, u32),
    /// Tricks you have seen this kind of creature use this run.
    pub known_traits: Vec<Trait>,
    pub winding_up: Option<Point>,
    /// Not really there. Only Look can tell.
    pub phantom: bool,
    /// Actions left under your Compel.
    pub compelled: u32,
    /// Actions left before it can notice you again.
    pub unseeing: u32,
    /// Actions left fleeing your dread.
    pub terrified: u32,
    pub pinned: u32,
    pub carries_dread: bool,
}

#[derive(Debug, Clone)]
pub struct World {
    pub(crate) content: &'static Content,
    seed: u64,
    depth: u8,
    pub(crate) floor: Floor,
    pub(crate) player: Player,
    ticks: u64,
    pub(crate) combat_rng: GameRng,
    pub(crate) ai_rng: GameRng,
    /// The kind dread takes form as.
    pub(crate) manifestation: KindId,
    /// Ids handed out so far; every item gets its own.
    pub(crate) next_item: u32,
    /// This run's tincture strengths and side effects.
    pub(crate) tinctures: HashMap<ItemKindId, TinctureLore>,
    last_burden: Burden,
    /// Level-up drafts waiting for a choice, oldest first.
    pub(crate) drafts: VecDeque<Vec<Boon>>,
    pub(crate) boon_rng: GameRng,
    pub(crate) stats: RunStats,
    sighted: HashSet<KindId>,
    witnessed: HashSet<(KindId, Trait)>,
    /// Kinds whose bodies you've studied.
    pub(crate) studied: HashSet<KindId>,
    pub(crate) rite_rng: GameRng,
    pub(crate) death: Option<Death>,
}

impl World {
    /// A new run. The seed decides every floor and every roll.
    pub fn new(seed: u64) -> Self {
        let content = Content::bundled();
        let mut next_item = 0;
        let floor = Self::generate_floor(content, seed, 1, &mut next_item);
        Self::on_floor(content, seed, floor, next_item)
    }

    /// A run starting on a hand-made map with no monsters, for tests and vaults.
    pub fn from_map(map: Map, start: Point) -> Self {
        assert!(
            map.is_walkable(start),
            "player must start on a walkable tile"
        );
        Self::on_floor(Content::bundled(), 0, Floor::new(map, start), 0)
    }

    fn on_floor(content: &'static Content, seed: u64, floor: Floor, next_item: u32) -> Self {
        let mut world = Self {
            content,
            seed,
            depth: 1,
            player: Player {
                pos: floor.arrival(),
                health: PLAYER_HEALTH,
                max_health: PLAYER_HEALTH,
                candle: Candle::default(),
                dread: Dread::default(),
                inventory: Vec::new(),
                equipment: Equipment::default(),
                level: 1,
                insight: 0,
                skills: Skills::default(),
                boons: Vec::new(),
                rites: Vec::new(),
                rite_state: crate::rites::Rites::default(),
                energy: ACTION_COST,
            },
            floor,
            ticks: 0,
            combat_rng: rng::stream(seed, Stream::Combat),
            ai_rng: rng::stream(seed, Stream::Ai),
            manifestation: content
                .kind_by_id("manifestation")
                .expect("monsters.ron defines the manifestation"),
            next_item,
            tinctures: inventory::roll_tinctures(&mut rng::stream(seed, Stream::Loot), content),
            last_burden: Burden::Light,
            drafts: VecDeque::new(),
            boon_rng: rng::stream(seed, Stream::Boons),
            stats: RunStats::default(),
            sighted: HashSet::new(),
            witnessed: HashSet::new(),
            studied: HashSet::new(),
            rite_rng: rng::stream(seed, Stream::Rites),
            death: None,
        };
        // The acolyte comes down with what was at hand.
        for (id, equip) in [
            ("iron_candlestick", true),
            ("cassock", true),
            ("mending_tincture", false),
        ] {
            let kind = content.item_by_id(id).expect("starting kit is defined");
            let item = Item {
                id: world.new_item_id(),
                kind,
                count: 1,
            };
            world.add_to_pack(item);
            if equip {
                let slot = content.item(kind).slot().expect("starting gear has a slot");
                world.player.equipment.set(slot, Some(item.id));
            }
        }
        world.last_burden = world.burden();
        // What you see on arrival is the scene, not news.
        world.update_view();
        world
    }

    fn generate_floor(content: &Content, seed: u64, depth: u8, next_item: &mut u32) -> Floor {
        let mut rng = rng::floor_rng(seed, depth);
        let layout = generate::crypt(&mut rng, depth < MAX_DEPTH);
        let spawns = spawn::populate(&mut rng, content, &layout.map, layout.start, depth);
        let tallow = spawn::place_tallow(&mut rng, &layout.map, layout.start);
        let items = spawn::place_items(&mut rng, content, &layout.map, layout.start, depth);
        let mut floor = Floor::new(layout.map, layout.start);
        for (kind, pos) in spawns {
            floor
                .monsters
                .insert(Monster::new(kind, content.monster(kind), pos));
        }
        floor.tallow = tallow
            .into_iter()
            .map(|(at, amount)| Tallow::new(at, amount))
            .collect();
        floor.items = items
            .into_iter()
            .map(|(at, kind, count)| {
                *next_item += 1;
                FloorItem::new(
                    at,
                    Item {
                        id: crate::item::ItemId(*next_item),
                        kind,
                        count,
                    },
                )
            })
            .collect();
        floor
    }

    /// Puts a monster on the current floor. For tests and scripted scenes.
    pub fn spawn_monster(&mut self, kind: KindId, pos: Point) -> MonsterId {
        assert!(self.map().is_walkable(pos) && self.floor.monster_at(pos).is_none());
        let id = self
            .floor
            .monsters
            .insert(Monster::new(kind, self.content.monster(kind), pos));
        self.update_view();
        id
    }

    /// Drops an item on the current floor. For tests and scripted scenes.
    pub fn place_item(&mut self, at: Point, kind: ItemKindId, count: u32) {
        let item = Item {
            id: self.new_item_id(),
            kind,
            count,
        };
        self.floor.items.push(FloorItem::new(at, item));
        self.update_view();
    }

    /// This run's truth about a tincture kind (whether or not it's been learned).
    pub fn tincture_lore(&self, kind: ItemKindId) -> Option<TinctureLore> {
        self.tinctures.get(&kind).copied()
    }

    /// Drops tallow on the current floor. For tests and scripted scenes.
    pub fn place_tallow(&mut self, at: Point, amount: u32) {
        self.floor.tallow.push(Tallow::new(at, amount));
        self.update_view();
    }

    /// Jumps straight to a deeper floor, as if by stairs but without the rewards.
    /// For testing deep content.
    pub fn dev_skip_to(&mut self, depth: u8) {
        let depth = depth.clamp(1, MAX_DEPTH);
        if depth <= self.depth {
            return;
        }
        self.depth = depth;
        self.floor = Self::generate_floor(self.content, self.seed, depth, &mut self.next_item);
        self.player.pos = self.floor.arrival();
        self.update_view();
    }

    /// Knows every rite. For testing.
    pub fn dev_learn_all_rites(&mut self) {
        self.player.rites = self.content.rite_ids().collect();
    }

    /// Removes every monster from the current floor. For tests and scripted scenes.
    pub fn despawn_all(&mut self) {
        self.floor.monsters.clear();
        self.update_view();
    }

    pub fn content(&self) -> &'static Content {
        self.content
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }

    pub fn floor(&self) -> &Floor {
        &self.floor
    }

    pub fn map(&self) -> &Map {
        self.floor.map()
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    /// Whole turns of game time. Actions that fail cost nothing.
    pub fn turn(&self) -> u64 {
        self.ticks / TICKS_PER_TURN
    }

    /// Set once the player has died. The world accepts no more commands.
    pub fn death(&self) -> Option<Death> {
        self.death
    }

    /// Look at a monster the player can see.
    pub fn inspect(&self, id: MonsterId) -> Option<MonsterInfo> {
        let m = self.floor.monster(id)?;
        if !self.floor.is_visible(m.pos) {
            return None;
        }
        let def = self.content.monster(m.kind);
        Some(MonsterInfo {
            kind: m.kind,
            pos: m.pos,
            health: m.health,
            max_health: m.max_health,
            mind: m.mind,
            your_hit_chance: combat::hit_chance(self.player_accuracy(), def.defense),
            its_hit_chance: combat::hit_chance(def.accuracy, self.player_defense()),
            its_damage: def.damage,
            known_traits: def
                .traits
                .iter()
                .copied()
                .filter(|t| self.witnessed.contains(&(m.kind, *t)))
                .collect(),
            winding_up: m.winding_up,
            phantom: m.phantom,
            compelled: m.compelled,
            unseeing: m.unseeing,
            terrified: m.terrified,
            pinned: m.pinned,
            carries_dread: m.carries_dread,
        })
    }

    /// Tiles about to be struck by a raised heavy blow.
    pub fn telegraphs(&self) -> impl Iterator<Item = Point> + '_ {
        self.floor.monsters().filter_map(|(_, m)| m.winding_up)
    }

    /// Resolves one player command and returns what happened.
    pub fn apply(&mut self, command: Command) -> Vec<Event> {
        if self.death.is_some() {
            return Vec::new();
        }
        let mut events = self.resolve(command);
        let burden = self.burden();
        if burden != self.last_burden {
            self.last_burden = burden;
            events.push(Event::BurdenChanged { burden });
        }
        events
    }

    fn resolve(&mut self, command: Command) -> Vec<Event> {
        match command {
            Command::Move(dir) => self.player_step(dir),
            Command::Run(dir) => self.run(dir),
            Command::Wait => {
                let mut events = vec![Event::PlayerWaited];
                self.pass_time(&mut events);
                events
            }
            Command::Descend => self.descend(),
            Command::ToggleCandle => self.toggle_candle(),
            Command::Rest => self.rest(),
            Command::PickUp => self.pick_up(),
            Command::Drop(item) => self.drop_item(item),
            Command::Equip(item) => self.toggle_equip(item),
            Command::Use(item) => self.use_item(item),
            Command::Throw { item, target } => self.throw(item, target),
            Command::Fire { target } => self.fire(target),
            Command::ChooseBoon(index) => self.choose_boon(index),
            Command::Study => self.study(),
            Command::Render => self.render(),
            Command::Cast { rite, target } => self.cast(rite, target),
            Command::Ascend => match self.map().tile(self.player.pos) {
                Tile::StairsUp => vec![Event::StairsSealed { depth: self.depth }],
                _ => vec![Event::NoStairsHere],
            },
        }
    }

    /// Steps into `dir`, attacking whatever stands there.
    fn player_step(&mut self, dir: Direction) -> Vec<Event> {
        let target = self.player.pos + dir;
        let mut events = Vec::new();
        let mut cost = ACTION_COST;
        let thrall = self
            .floor
            .monster_at(target)
            .filter(|&id| self.floor.monsters[id].compelled > 0);
        if let Some(id) = thrall {
            // Your thrall steps aside into your place.
            let here = self.player.pos;
            self.floor.monsters[id].pos = here;
            self.player.pos = target;
            events.push(Event::SwappedPlaces {
                kind: self.floor.monsters[id].kind,
            });
            events.push(Event::PlayerMoved { to: target });
        } else if let Some(id) = self.floor.monster_at(target) {
            self.player_attack(id, 0, &mut events);
        } else if let Some((id, bonus)) = self.reach_target(dir) {
            self.player_attack(id, bonus, &mut events);
        } else {
            let tile = self.map().tile(target);
            if !tile.is_walkable() {
                return vec![Event::PlayerBlocked { at: target, tile }];
            }
            match self.burden() {
                Burden::Overloaded => return vec![Event::TooHeavy],
                Burden::Burdened => cost = BURDENED_MOVE,
                Burden::Light => {}
            }
            self.player.pos = target;
            events.push(Event::PlayerMoved { to: target });
            if let Some(i) = self.floor.tallow.iter().position(|t| t.at == target) {
                let found = self.floor.tallow.swap_remove(i);
                let amount = if self.has_passive(Passive::TallowThief) {
                    found.amount * 5 / 4
                } else {
                    found.amount
                };
                self.player.candle.add(amount);
                events.push(Event::TallowFound { amount });
            }
        }
        self.pass_time_costing(cost, &mut events);
        events
    }

    fn toggle_candle(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if self.player.candle.is_lit() {
            self.player.candle.snuff();
            events.push(Event::CandleSnuffed);
            self.stats.snuffs += 1;
            self.trigger(Trigger::Snuff, &mut events);
        } else if self.player.candle.light() {
            events.push(Event::CandleLit);
        } else {
            return vec![Event::CandleSpent];
        }
        self.pass_time(&mut events);
        events
    }

    /// Waits until healed (and, by a brazier, calm), or until something happens.
    fn rest(&mut self) -> Vec<Event> {
        if !self.hostiles_in_view().is_empty() {
            return vec![Event::RunRefused];
        }
        if self.rested() {
            return vec![Event::NothingToRest];
        }
        let mut events = Vec::new();
        let mut turns = 0;
        while turns < MAX_REST_TURNS && !self.rested() {
            let mut step = vec![Event::PlayerWaited];
            self.pass_time(&mut step);
            turns += 1;
            let disturbed = step.iter().any(|e| !is_routine(e));
            events.extend(step);
            if disturbed || !self.hostiles_in_view().is_empty() {
                return events;
            }
        }
        events.push(Event::Rested { turns });
        events
    }

    /// Nothing left to recover here: full health, and calm unless by a brazier.
    fn rested(&self) -> bool {
        let by_brazier = self.floor.ambient_light(self.player.pos).is_lit();
        self.player.health == self.player.max_health
            && (self.player.dread.value() == 0 || !by_brazier)
    }

    /// A creature two tiles away in `dir` that a long-reach weapon can strike,
    /// with the technique's accuracy bonus. The tile between must be open.
    fn reach_target(&self, dir: Direction) -> Option<(MonsterId, i32)> {
        if self.wielded_family() != Some(Family::Reach) {
            return None;
        }
        let Some(Technique::LongReach { accuracy }) =
            self.melee_technique(|t| matches!(t, Technique::LongReach { .. }))
        else {
            return None;
        };
        let between = self.player.pos + dir;
        let far = between + dir;
        let open = self.map().is_walkable(between) && self.floor.monster_at(between).is_none();
        let id = self
            .floor
            .monster_at(far)
            .filter(|&id| self.floor.is_visible(far) && self.floor.monsters[id].compelled == 0)?;
        open.then_some((id, accuracy))
    }

    /// Strikes a monster with the wielded weapon. Also used for ripostes and reach strikes.
    pub(crate) fn player_attack(&mut self, id: MonsterId, bonus: i32, events: &mut Vec<Event>) {
        let monster = &self.floor.monsters[id];
        let kind = monster.kind;
        if monster.phantom {
            self.floor.monsters.remove(id);
            events.push(Event::PhantomFaded { kind, struck: true });
            return;
        }
        let def = self.content.monster(kind);
        let chance = combat::hit_chance(self.player_accuracy() + bonus, def.defense);
        let weapon = self.player_damage();
        let health_before = self.floor.monsters[id].health;
        let family = self.wielded_family();
        let damage = combat::roll_attack(&mut self.combat_rng, chance, weapon);
        events.push(Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(kind),
            damage,
        });

        self.wake(id);
        let Some(damage) = damage else { return };
        if let Some(family) = family {
            self.train(Skill::of_family(family), damage.min(health_before), events);
        }
        self.damage_monster(id, damage, Source::Melee(family), events);
        if family == Some(Family::Bludgeon)
            && let Some(Technique::Stagger { chance }) =
                self.melee_technique(|t| matches!(t, Technique::Stagger { .. }))
            && let Some(monster) = self.floor.monsters.get(id)
            && rand::RngExt::random_range(&mut self.combat_rng, 0..100) < chance
        {
            let kind = monster.kind;
            self.floor.monsters[id].energy -= ACTION_COST;
            events.push(Event::Staggered { kind });
        }
    }

    /// Being attacked makes an unaware monster turn on you.
    pub(crate) fn wake(&mut self, id: MonsterId) {
        let monster = &mut self.floor.monsters[id];
        if monster.mind == Mind::Unaware {
            monster.mind = Mind::Hunting {
                last_seen: self.player.pos,
            };
        }
    }

    pub(crate) fn damage_monster(
        &mut self,
        id: MonsterId,
        damage: u32,
        source: Source,
        events: &mut Vec<Event>,
    ) {
        let monster = &mut self.floor.monsters[id];
        monster.health = monster.health.saturating_sub(damage);
        if monster.health > 0 {
            return;
        }
        let (kind, at) = (monster.kind, monster.pos);
        self.floor.monsters.remove(id);
        if self.floor.is_visible(at) {
            events.push(Event::MonsterDied { kind, at });
        }
        self.leave_corpse(kind, at);
        if self
            .player
            .rite_state
            .borrowed
            .is_some_and(|(b, _, _)| b == id)
        {
            self.player.rite_state.borrowed = None;
            events.push(Event::EyesReturned);
            events.extend(self.update_view());
        }
        if kind == self.manifestation {
            events.push(Event::ManifestationBanished);
            self.ease_dread(events);
        }
        self.credit_kill(kind, source, events);
    }

    pub(crate) fn hurt_player(&mut self, damage: u32, cause: Cause, events: &mut Vec<Event>) {
        let taken = damage.min(self.player.health);
        self.player.health = self.player.health.saturating_sub(damage);
        if self.player.equipment.body.is_some() {
            self.train(Skill::Endurance, taken, events);
        }
        if self.player.health == 0 && self.death.is_none() {
            self.death = Some(Death {
                cause,
                depth: self.depth,
                turn: self.turn(),
            });
            events.push(Event::PlayerDied { cause });
        }
    }

    pub(crate) fn sighted_kinds(&self) -> impl Iterator<Item = KindId> + '_ {
        self.sighted.iter().copied()
    }

    pub(crate) fn witness(&mut self, kind: KindId, t: Trait) {
        self.witnessed.insert((kind, t));
    }

    pub(crate) fn shift_dread(&mut self, hundredths: i32, events: &mut Vec<Event>) {
        if let Some(band) = self.player.dread.shift(hundredths) {
            events.push(Event::DreadChanged { band });
        }
    }

    /// After a Manifestation is dealt with: dread settles and phantoms go.
    fn ease_dread(&mut self, events: &mut Vec<Event>) {
        if let Some(band) = self.player.dread.set(dread::AFTER_MANIFESTATION) {
            events.push(Event::DreadChanged { band });
        }
        self.clear_phantoms(events);
    }

    /// After an action that costs time: the world moves until the player can act again.
    pub(crate) fn pass_time(&mut self, events: &mut Vec<Event>) {
        self.pass_time_costing(ACTION_COST, events);
    }

    fn pass_time_costing(&mut self, cost: i32, events: &mut Vec<Event>) {
        events.extend(self.update_view());
        events.extend(self.note_sightings());
        for monster in self.floor.monsters.values_mut() {
            monster.blow_ready = monster.winding_up.is_some();
        }
        self.player.energy -= cost;
        while self.player.energy < ACTION_COST && self.death.is_none() {
            self.tick(events);
        }
        events.extend(self.update_view());
        events.extend(self.note_sightings());
    }

    pub(crate) fn tick(&mut self, events: &mut Vec<Event>) {
        self.ticks += 1;
        if self.ticks.is_multiple_of(TICKS_PER_TURN) {
            self.on_turn(events);
        }
        let ids: Vec<MonsterId> = self.floor.monsters.keys().collect();
        for id in ids {
            let Some(monster) = self.floor.monsters.get_mut(id) else {
                continue;
            };
            monster.energy += self.content.monster(monster.kind).speed as i32;
            while self.death.is_none()
                && self
                    .floor
                    .monsters
                    .get(id)
                    .is_some_and(|m| m.energy >= ACTION_COST)
            {
                self.floor.monsters[id].energy -= ACTION_COST;
                self.monster_act(id, events);
            }
        }
        self.player.energy += PLAYER_SPEED;
    }

    /// Steps in `dir` until something worth a decision happens: a wall ahead,
    /// a door or stair underfoot or alongside, a new landmark or any creature
    /// in view, anything happening to you, or (in a corridor) a side passage.
    /// Refused outright while something hostile is in view.
    fn run(&mut self, dir: Direction) -> Vec<Event> {
        if !self.hostiles_in_view().is_empty() {
            return vec![Event::RunRefused];
        }
        let sides = |world: &World| {
            [2, -2].map(|turn| world.map().is_walkable(world.player.pos + dir.rotate(turn)))
        };
        let in_corridor = sides(self) == [false, false];
        let mut known_landmarks: Vec<Point> = self.floor.visible_landmarks().collect();
        let mut known_doors = self.doors_beside(dir);

        let mut events = Vec::new();
        for steps in 0..MAX_RUN_STEPS {
            let ahead = self.player.pos + dir;
            if !self.map().is_walkable(ahead) && self.floor.monster_at(ahead).is_none() {
                if steps == 0 {
                    events.extend(self.player_step(dir));
                }
                break;
            }
            let step_events = self.player_step(dir);
            let disturbed = step_events.iter().any(|e| !is_routine(e));
            events.extend(step_events);

            let underfoot = self.map().tile(self.player.pos);
            let new_landmark = self
                .floor
                .visible_landmarks()
                .any(|p| !known_landmarks.contains(&p));
            let doors = self.doors_beside(dir);
            let new_door = doors.iter().any(|d| !known_doors.contains(d));
            let side_opened = in_corridor && sides(self) != [false, false];
            let company = !self.hostiles_in_view().is_empty();
            if disturbed
                || company
                || matches!(underfoot, Tile::Door | Tile::StairsDown | Tile::StairsUp)
                || new_landmark
                || new_door
                || side_opened
            {
                break;
            }
            known_landmarks.extend(self.floor.visible_landmarks());
            known_doors = doors;
        }
        events
    }

    /// Doors next to the player, except the one straight ahead (a run steps onto that one).
    fn doors_beside(&self, heading: Direction) -> Vec<Point> {
        Direction::ALL
            .iter()
            .filter(|&&d| d != heading)
            .map(|&d| self.player.pos + d)
            .filter(|&p| self.map().tile(p) == Tile::Door)
            .collect()
    }

    fn descend(&mut self) -> Vec<Event> {
        if self.map().tile(self.player.pos) != Tile::StairsDown {
            return vec![Event::NoStairsHere];
        }
        let fled = self
            .floor
            .monsters()
            .any(|(_, m)| m.kind == self.manifestation);
        self.depth += 1;
        self.floor = Self::generate_floor(self.content, self.seed, self.depth, &mut self.next_item);
        self.player.pos = self.floor.arrival();
        let mut events = vec![Event::Descended { depth: self.depth }];
        if fled {
            events.push(Event::ManifestationEscaped);
            self.ease_dread(&mut events);
        }
        self.trigger(Trigger::Descend, &mut events);
        self.gain_insight(crate::progress::INSIGHT_NEW_FLOOR, &mut events);
        self.pass_time(&mut events);
        events
    }

    /// Recomputes light and sight. Returns newly spotted landmarks and, when
    /// time has passed, first sightings of creatures.
    pub(crate) fn update_view(&mut self) -> Vec<Event> {
        let candle = self.player.candle.radius().map(|radius| LightSource {
            at: self.player.pos,
            radius,
            color: candle::COLOR,
        });
        let feel = if self.has_passive(Passive::DarkSight) {
            2
        } else {
            1
        };
        let borrowed = self
            .player
            .rite_state
            .borrowed
            .and_then(|(id, _, radius)| Some((self.floor.monsters.get(id)?.pos, radius)));
        self.floor
            .update_view(self.player.pos, candle, feel, borrowed)
    }

    /// Creatures in view that aren't your thralls, nearest first.
    pub fn hostiles_in_view(&self) -> Vec<MonsterId> {
        self.floor
            .visible_monsters(self.player.pos)
            .into_iter()
            .filter(|&id| self.floor.monsters[id].compelled == 0)
            .collect()
    }

    /// First sightings of creature kinds now in view. Reported from actions only,
    /// so a creature already in view on arrival is introduced on your first move.
    fn note_sightings(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        for id in self.floor.visible_monsters(self.player.pos) {
            let monster = &self.floor.monsters[id];
            let kind = monster.kind;
            if monster.phantom || !self.sighted.insert(kind) {
                continue;
            }
            events.push(Event::FirstSighting { kind });
            let def = self.content.monster(kind);
            // Some tricks are plain to see.
            if def.has(|t| *t == Trait::ShunsLight) {
                self.witness(kind, Trait::ShunsLight);
            }
            if def.has(|t| *t == Trait::Relentless) {
                self.witness(kind, Trait::Relentless);
            }
            let shock = match def.faction {
                Faction::Dreaming => dread::FIRST_SIGHT_OF_DREAMING,
                _ => dread::FIRST_SIGHT,
            };
            if kind != self.manifestation {
                self.shift_dread(shock, &mut events);
            }
            self.gain_insight(crate::progress::INSIGHT_FIRST_SIGHT, &mut events);
        }
        events
    }
}

/// Events that don't interrupt a run or a rest.
pub(crate) fn is_routine(event: &Event) -> bool {
    matches!(
        event,
        Event::PlayerMoved { .. }
            | Event::SwappedPlaces { .. }
            | Event::PlayerWaited
            | Event::Spotted { .. }
            | Event::SpottedTallow { .. }
            | Event::TallowFound { .. }
            | Event::Whisper { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::prefab;

    fn world_from(text: &str) -> World {
        let (map, start) = prefab::parse(text).unwrap();
        World::from_map(map, start)
    }

    fn small_world() -> World {
        world_from("#####\n#@..#\n#..+#\n#####")
    }

    fn kind(id: &str) -> KindId {
        Content::bundled().kind_by_id(id).unwrap()
    }

    #[test]
    fn moving_onto_floor_advances_the_turn() {
        let mut world = small_world();
        let events = world.apply(Command::Move(Direction::E));
        assert_eq!(world.player().pos, Point::new(2, 1));
        assert_eq!(world.turn(), 1);
        assert_eq!(
            events,
            vec![Event::PlayerMoved {
                to: Point::new(2, 1)
            }]
        );
    }

    #[test]
    fn walls_block_and_cost_no_time() {
        let mut world = small_world();
        let events = world.apply(Command::Move(Direction::N));
        assert_eq!(world.player().pos, Point::new(1, 1));
        assert_eq!(world.turn(), 0);
        assert_eq!(
            events,
            vec![Event::PlayerBlocked {
                at: Point::new(1, 0),
                tile: Tile::Wall
            }]
        );
    }

    #[test]
    fn diagonal_moves_and_doors_are_walkable() {
        let mut world = small_world();
        world.apply(Command::Move(Direction::SE));
        world.apply(Command::Move(Direction::E));
        assert_eq!(world.player().pos, Point::new(3, 2));
        assert_eq!(world.map().tile(world.player().pos), Tile::Door);
    }

    #[test]
    fn waiting_advances_the_turn() {
        let mut world = small_world();
        assert_eq!(world.apply(Command::Wait), vec![Event::PlayerWaited]);
        assert_eq!(world.turn(), 1);
    }

    #[test]
    fn running_stops_at_the_wall() {
        let mut world = world_from("##########\n#@.......#\n##########");
        world.apply(Command::Run(Direction::E));
        assert_eq!(world.player().pos, Point::new(8, 1));
        assert_eq!(world.turn(), 7);
    }

    #[test]
    fn running_into_a_wall_is_just_a_bump() {
        let mut world = world_from("####\n#@.#\n####");
        let events = world.apply(Command::Run(Direction::N));
        assert!(matches!(events[..], [Event::PlayerBlocked { .. }]));
        assert_eq!(world.turn(), 0);
    }

    #[test]
    fn running_stops_on_a_door() {
        let mut world = world_from("##########\n#@..+....#\n##########");
        world.apply(Command::Run(Direction::E));
        assert_eq!(world.player().pos, Point::new(4, 1));
    }

    #[test]
    fn running_stops_at_a_side_passage() {
        let mut world = world_from(
            "##########\n\
             #@.......#\n\
             ####.#####\n\
             ####.#####",
        );
        world.apply(Command::Run(Direction::E));
        assert_eq!(world.player().pos, Point::new(4, 1));
    }

    #[test]
    fn running_through_a_room_ignores_its_shape_but_stops_beside_doors() {
        let mut world = world_from(
            "###########\n\
             #@........#\n\
             #.........#\n\
             ######+####",
        );
        world.apply(Command::Run(Direction::E));
        assert_eq!(
            world.player().pos,
            Point::new(9, 1),
            "open rooms don't stop a run"
        );

        let mut world = world_from(
            "###########\n\
             #.........#\n\
             #@........#\n\
             ######+####",
        );
        world.apply(Command::Run(Direction::E));
        assert_eq!(
            world.player().pos,
            Point::new(5, 2),
            "a door alongside stops it"
        );
    }

    #[test]
    fn running_stops_when_a_new_landmark_comes_into_view() {
        // The brazier sits in a side room that only becomes visible past the gap.
        let mut world = world_from(
            "#################\n\
             #@..............#\n\
             #######.#########\n\
             ####.......######\n\
             ####...&...######\n\
             #################",
        );
        let events = world.apply(Command::Run(Direction::E));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Spotted {
                tile: Tile::Brazier,
                ..
            }
        )));
        assert!(world.player().pos.x < 15, "stopped before the far wall");
    }

    #[test]
    fn running_is_refused_with_company_and_stops_when_company_appears() {
        let mut world = world_from("############\n#@.........#\n############");
        world.spawn_monster(kind("parishioner"), Point::new(4, 1));
        assert_eq!(
            world.apply(Command::Run(Direction::E)),
            vec![Event::RunRefused]
        );
        assert_eq!(world.turn(), 0);

        // Out of candle range in a dark hall: the run starts, then stops when it's seen.
        let mut world = world_from(
            "##########################\n#@.......................#\n##########################",
        );
        world.spawn_monster(kind("parishioner"), Point::new(20, 1));
        world.apply(Command::Run(Direction::E));
        assert!(world.player().pos.x < 19);
        assert!(
            !world
                .floor()
                .visible_monsters(world.player().pos)
                .is_empty()
        );
    }

    #[test]
    fn stairs_up_are_sealed_and_cost_nothing() {
        let mut world = world_from("#####\n#<@>#\n#####");
        world.apply(Command::Move(Direction::W));
        let turn = world.turn();
        assert_eq!(
            world.apply(Command::Ascend),
            vec![Event::StairsSealed { depth: 1 }]
        );
        assert_eq!(world.turn(), turn);
    }

    #[test]
    fn descending_needs_stairs() {
        let mut world = small_world();
        assert_eq!(world.apply(Command::Descend), vec![Event::NoStairsHere]);
        assert_eq!(world.depth(), 1);
    }

    /// Clears the floor of monsters, walks to the down-stair and takes it.
    fn descend_once(world: &mut World) {
        world.floor.monsters.clear();
        let stairs = world
            .map()
            .find(Tile::StairsDown)
            .next()
            .expect("stairs exist");
        let dist = crate::map::path::distances(world.map(), stairs);
        while world.player().pos != stairs {
            let here = dist.at(world.player().pos).expect("stairs reachable");
            let dir = Direction::ALL
                .into_iter()
                .find(|&d| dist.at(world.player().pos + d).is_some_and(|n| n < here))
                .expect("a step closer exists");
            world.apply(Command::Move(dir));
        }
        let events = world.apply(Command::Descend);
        assert!(matches!(events[0], Event::Descended { .. }));
    }

    #[test]
    fn a_seed_reaches_the_bottom_and_the_bottom_has_no_stairs() {
        let mut world = World::new(1234);
        while world.depth() < MAX_DEPTH {
            descend_once(&mut world);
            assert_eq!(world.player().pos, world.floor().arrival());
            assert_eq!(world.map().tile(world.player().pos), Tile::StairsUp);
            assert!(
                world.floor().monsters().count() > 0,
                "floor {} is empty",
                world.depth()
            );
        }
        assert_eq!(world.map().find(Tile::StairsDown).count(), 0);
    }

    #[test]
    fn same_seed_same_dungeon() {
        let a = World::new(99);
        let b = World::new(99);
        assert_eq!(a.map(), b.map());
        assert_eq!(a.player().pos, b.player().pos);
        assert_ne!(World::new(100).map(), a.map());
    }

    #[test]
    fn the_candle_lights_the_way() {
        let world = World::new(5);
        assert!(world.floor().light(world.player().pos).is_lit());
        assert!(world.floor().is_visible(world.player().pos));
    }

    #[test]
    fn bumping_a_monster_attacks_it_until_it_dies() {
        let mut world = world_from("######\n#@...#\n######");
        let id = world.spawn_monster(kind("gnawer"), Point::new(2, 1));
        let mut died = false;
        for _ in 0..50 {
            let events = world.apply(Command::Move(Direction::E));
            assert!(
                events.iter().any(|e| matches!(
                    e,
                    Event::Attack {
                        attacker: Who::Player,
                        ..
                    }
                )) || world.floor().monster(id).is_none()
            );
            if events
                .iter()
                .any(|e| matches!(e, Event::MonsterDied { .. }))
            {
                died = true;
                break;
            }
        }
        assert!(died, "a lone gnawer should die within 50 swings");
        assert_eq!(
            world.player().pos,
            Point::new(1, 1),
            "attacking doesn't move you"
        );
    }

    #[test]
    fn monsters_notice_hunt_and_hit_you() {
        let mut world = world_from("##########\n#@.......#\n##########");
        world.spawn_monster(kind("parishioner"), Point::new(5, 1));
        let mut events = Vec::new();
        for _ in 0..30 {
            events.extend(world.apply(Command::Wait));
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Noticed { seen: true, .. }))
        );
        assert!(
            events.iter().any(|e| matches!(e, Event::Bark { .. })),
            "parishioners plead"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Attack {
                attacker: Who::Monster(_),
                defender: Who::Player,
                ..
            }
        )));
    }

    #[test]
    fn the_player_can_die_and_the_world_then_stops() {
        let mut world = world_from("#####\n#@..#\n#####");
        world.spawn_monster(kind("parishioner"), Point::new(2, 1));
        world.player.health = 1;
        let mut events = Vec::new();
        for _ in 0..200 {
            events.extend(world.apply(Command::Wait));
            if world.death().is_some() {
                break;
            }
        }
        let death = world
            .death()
            .expect("a parishioner kills a 1-health acolyte eventually");
        assert_eq!(death.cause, Cause::Attack(kind("parishioner")));
        assert!(events.iter().any(|e| matches!(e, Event::PlayerDied { .. })));
        assert!(world.apply(Command::Wait).is_empty());
    }

    #[test]
    fn heavy_blows_are_telegraphed_and_dodgeable() {
        let mut world = world_from("#######\n#@....#\n#.....#\n#######");
        let pallbearer = world.spawn_monster(kind("pallbearer"), Point::new(2, 1));
        world.floor.monsters[pallbearer].mind = Mind::Hunting {
            last_seen: Point::new(1, 1),
        };

        // Wait until it raises the blow.
        let mut events = Vec::new();
        while world.telegraphs().next().is_none() {
            events = world.apply(Command::Wait);
            assert!(world.death().is_none());
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::WindUp { target, .. } if *target == Point::new(1, 1)))
        );
        assert_eq!(
            world.inspect(pallbearer).unwrap().winding_up,
            Some(Point::new(1, 1))
        );
        assert!(
            world
                .inspect(pallbearer)
                .unwrap()
                .known_traits
                .iter()
                .any(|t| matches!(t, Trait::HeavyBlow { .. }))
        );

        // Step out from under it: the blow hits the empty floor.
        let health = world.player().health;
        let mut events = world.apply(Command::Move(Direction::S));
        while world.telegraphs().next().is_some() {
            events.extend(world.apply(Command::Move(Direction::E)));
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::HeavyBlow { damage: None, .. }))
        );
        assert!(
            world.player().health + 3 >= health,
            "only ordinary blows could have landed"
        );
    }

    #[test]
    fn a_heavy_blow_always_waits_for_your_next_action() {
        let mut world = world_from("#######\n#@....#\n#######");
        let pallbearer = world.spawn_monster(kind("pallbearer"), Point::new(2, 1));
        world.floor.monsters[pallbearer].mind = Mind::Hunting {
            last_seen: Point::new(1, 1),
        };
        // Absurdly fast pallbearer: it would get many actions per player turn.
        while world.telegraphs().next().is_none() {
            world.apply(Command::Wait);
        }
        world.floor.monsters[pallbearer].energy = 10_000;
        world.player.energy = 0;
        let mut events = Vec::new();
        world.tick(&mut events);
        assert!(
            events.iter().all(|e| !matches!(e, Event::HeavyBlow { .. })),
            "no blow before you act"
        );
        assert!(world.telegraphs().next().is_some());
    }

    #[test]
    fn first_sightings_describe_once_even_if_already_in_view() {
        let mut world = world_from("########\n#@.....#\n########");
        world.spawn_monster(kind("parishioner"), Point::new(4, 1));
        world.spawn_monster(kind("parishioner"), Point::new(5, 1));
        let events = world.apply(Command::Wait);
        let sightings = events
            .iter()
            .filter(|e| matches!(e, Event::FirstSighting { .. }))
            .count();
        assert_eq!(sightings, 1, "one introduction per kind");
        let again = world.apply(Command::Wait);
        assert!(
            again
                .iter()
                .all(|e| !matches!(e, Event::FirstSighting { .. }))
        );
    }

    #[test]
    fn a_lone_gnawer_flees() {
        let mut world = world_from("############\n#@.........#\n############");
        world.spawn_monster(kind("gnawer"), Point::new(3, 1));
        let mut events = Vec::new();
        for _ in 0..6 {
            events.extend(world.apply(Command::Wait));
        }
        assert!(events.iter().any(|e| matches!(e, Event::Fled { .. })));
    }

    #[test]
    fn lantern_eaters_will_not_cross_brazier_light() {
        let mut world = world_from(
            "##########################\n\
             #@......&................#\n\
             ##########################",
        );
        let eater = world.spawn_monster(kind("lantern_eater"), Point::new(22, 1));
        world.floor.monsters[eater].mind = Mind::Hunting {
            last_seen: Point::new(1, 1),
        };
        for _ in 0..40 {
            world.apply(Command::Wait);
        }
        let pos = world.floor().monster(eater).unwrap().pos;
        assert!(
            !world.floor().ambient_light(pos).is_lit(),
            "it stays in the dark"
        );
        assert!(pos.x > 8, "it never got past the brazier");
    }

    #[test]
    fn health_regenerates_slowly() {
        use crate::time::REGEN_TURNS;
        let mut world = small_world();
        world.player.health = 10;
        for _ in 0..REGEN_TURNS {
            world.apply(Command::Wait);
        }
        assert_eq!(world.player().health, 11);
    }

    #[test]
    fn look_shows_real_hit_chances_only_for_visible_monsters() {
        let mut world = world_from(
            "##############################\n#@...........................#\n##############################",
        );
        let near = world.spawn_monster(kind("gnawer"), Point::new(3, 1));
        let far = world.spawn_monster(kind("gnawer"), Point::new(25, 1));
        let info = world.inspect(near).unwrap();
        assert_eq!(
            info.your_hit_chance,
            combat::hit_chance(world.player_accuracy(), 15)
        );
        assert_eq!(
            info.its_hit_chance,
            combat::hit_chance(60, world.player_defense())
        );
        assert_eq!(
            info.your_hit_chance, 70,
            "candlestick +5 on base 80, against defense 15"
        );
        assert_eq!(
            info.its_hit_chance, 50,
            "cassock +3 on base 7, against accuracy 60"
        );
        assert!(
            world.inspect(far).is_none(),
            "unseen monsters can't be inspected"
        );
    }

    #[test]
    fn same_seed_same_fight() {
        let play = |seed| {
            let mut world = World::new(seed);
            let mut log = Vec::new();
            for i in 0..200 {
                let dir = Direction::ALL[i % 8];
                log.extend(world.apply(Command::Move(dir)));
            }
            (log, world.player().health)
        };
        assert_eq!(play(77), play(77));
    }
}
