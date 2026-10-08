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
/// How far your light reaches with ink in your eyes.
pub const BLIND_RADIUS: i32 = 2;
/// How far the Vigil Candle lights.
pub const VIGIL_RADIUS: i32 = 7;

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
    /// The turn each recently cast rite is ready again.
    pub rite_ready: std::collections::BTreeMap<crate::rites::RiteId, u64>,
    /// Ink in your eyes until this turn: your light shrinks.
    pub blind_until: Option<u64>,
    /// Leavings carried, in the order taken.
    pub leavings: Vec<crate::leavings::LeavingId>,
    /// A Leaving lets you see in the dark until this turn.
    pub dark_sight_until: Option<u64>,
    /// Carrying the Vigil Candle: endless light, and the way up.
    pub vigil: bool,
    pub(crate) energy: i32,
}

/// What each step of growing adds to a creature (the Manifestation, mostly).
pub const GROWN_HEALTH: u32 = 6;
pub const GROWN_ACCURACY: i32 = 5;
pub const GROWN_DAMAGE: u32 = 1;

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
    pub took_leaving: bool,
    /// Rites learned this run, including any later forgotten.
    pub rites_learned: u32,
}

/// How and when a run ended in death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Death {
    pub cause: Cause,
    pub depth: u8,
    /// Ascent floor (1–4) if it happened on the way up, else 0.
    pub ascent: u8,
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
    /// It stands where no light reaches: you strike worse, it strikes harder.
    pub in_the_dark: bool,
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
    /// Rites learned with no room for them, waiting for you to make room.
    pub(crate) offered_rites: VecDeque<crate::rites::RiteId>,
    pub(crate) boon_rng: GameRng,
    pub(crate) stats: RunStats,
    sighted: HashSet<KindId>,
    witnessed: HashSet<(KindId, Trait)>,
    /// Kinds whose bodies you've studied.
    pub(crate) studied: HashSet<KindId>,
    pub(crate) rite_rng: GameRng,
    pub(crate) death: Option<Death>,
    /// You were warned about fire, deep water or rotten boards at this tile;
    /// stepping in again goes ahead.
    pub(crate) fire_warned: Option<Point>,
    /// The warning from the previous command, valid for this one only.
    pub(crate) fire_ok: Option<Point>,
    /// Every Leaving of the run, wherever it is.
    pub(crate) leavings: Vec<crate::leavings::Leaving>,
    /// When a used Leaving can be used again.
    pub(crate) leaving_ready: HashMap<crate::leavings::LeavingId, u64>,
    /// A Leaving is waking; others don't wake from what it does.
    pub(crate) leaving_busy: bool,
    /// Beelzebub's fight, once you reach the bottom.
    pub(crate) lord: Option<crate::throne::Lord>,
    /// Ascent floors climbed (0 during the descent; past the last, the church).
    pub(crate) ascent: u8,
    /// The turn the Following comes up onto this ascent floor.
    pub(crate) following_at: Option<u64>,
    /// The Vigil Candle has flared on this ascent floor.
    pub(crate) flared: bool,
    /// Turns spent frayed or worse since you were last below it.
    pub(crate) frayed_turns: u32,
    /// A nightmare is on its way and arrives on this turn.
    pub(crate) nightmare_at: Option<u64>,
    /// Manifestations so far this run: each one comes back stronger.
    pub(crate) manifestations: u32,
    pub(crate) victory: Option<crate::throne::Victory>,
    pub(crate) defeated: HashSet<KindId>,
    pub(crate) leavings_taken: Vec<crate::leavings::LeavingId>,
}

impl World {
    /// A new run. The seed decides every floor and every roll.
    pub fn new(seed: u64) -> Self {
        let content = Content::bundled();
        let mut next_item = 0;
        let floor = Self::generate_floor(content, seed, 1, &mut next_item, &mut Vec::new());
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
                rite_ready: std::collections::BTreeMap::new(),
                blind_until: None,
                leavings: Vec::new(),
                dark_sight_until: None,
                vigil: false,
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
            offered_rites: VecDeque::new(),
            boon_rng: rng::stream(seed, Stream::Boons),
            stats: RunStats::default(),
            sighted: HashSet::new(),
            witnessed: HashSet::new(),
            studied: HashSet::new(),
            rite_rng: rng::stream(seed, Stream::Rites),
            death: None,
            fire_warned: None,
            fire_ok: None,
            leavings: Vec::new(),
            leaving_ready: HashMap::new(),
            leaving_busy: false,
            lord: None,
            ascent: 0,
            following_at: None,
            flared: false,
            frayed_turns: 0,
            nightmare_at: None,
            manifestations: 0,
            victory: None,
            defeated: HashSet::new(),
            leavings_taken: Vec::new(),
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

    fn generate_floor(
        content: &Content,
        seed: u64,
        depth: u8,
        next_item: &mut u32,
        leavings: &mut Vec<crate::leavings::Leaving>,
    ) -> Floor {
        let mut rng = rng::floor_rng(seed, depth);
        if depth == MAX_DEPTH {
            let (map, start, dais, bodies, lord) = crate::throne::throne_room(&mut rng);
            let mut floor = Floor::new(map, start);
            let parishioner = content.kind_by_id("parishioner").expect("defined");
            for at in bodies {
                floor.corpses.push(crate::corpse::Corpse {
                    at,
                    kind: parishioner,
                    died: 0,
                    studied: 0,
                    rendered: 0,
                    ancient: true,
                    spoiled: false,
                });
            }
            let court = content.kind_by_id("beelzebub").expect("defined");
            floor
                .monsters
                .insert(Monster::new(court, content.monster(court), lord));
            let candle = content.item_by_id("vigil_candle").expect("defined");
            *next_item += 1;
            floor.items.push(FloorItem::new(
                dais,
                Item {
                    id: crate::item::ItemId(*next_item),
                    kind: candle,
                    count: 1,
                },
            ));
            return floor;
        }
        let layout = generate::floor(&mut rng, depth, depth < MAX_DEPTH);
        let spawns = spawn::populate(&mut rng, content, &layout.map, layout.start, depth);
        let tallow = spawn::place_tallow(&mut rng, &layout.map, layout.start);
        let items = spawn::place_items(&mut rng, content, &layout.map, layout.start, depth);
        let bosses = spawn::place_boss(&mut rng, content, &layout.map, depth);
        let writing = spawn::place_writing(&mut rng, &layout.map, layout.start, depth);
        let remains = spawn::place_remains(&mut rng, content, &layout.map, layout.start, depth);
        let seep = layout.seep.map(|room| {
            spawn::fill_seep(&mut rng, content, &layout.map, room, layout.start, depth)
        });
        // Nothing worth having lies inside a seep room but its Leaving.
        let outside = |at: Point| match layout.seep {
            Some(room) => spawn::outside_room(&layout.map, room, at),
            None => at,
        };
        let tallow: Vec<(Point, u32)> =
            tallow.into_iter().map(|(at, n)| (outside(at), n)).collect();
        let items: Vec<_> = items
            .into_iter()
            .map(|(at, k, n)| (outside(at), k, n))
            .collect();
        let writing = writing.filter(|&(_, front)| outside(front) == front);
        let mut floor = Floor::new(layout.map, layout.start);
        floor
            .writings
            .extend(writing.map(|(at, _)| crate::floor::Writing {
                at,
                seen: false,
                read: false,
            }));
        if let Some(seep) = seep {
            floor.seep = layout.seep;
            floor.anomalies = seep.anomalies;
            leavings.push(seep.leaving);
            let id = crate::leavings::LeavingId(leavings.len() as u32 - 1);
            floor.leavings.push((seep.leaving_at, id));
            let stone = content.item_by_id("stone").expect("stones exist");
            *next_item += 1;
            floor.items.push(FloorItem::new(
                seep.stones_at,
                Item {
                    id: crate::item::ItemId(*next_item),
                    kind: stone,
                    count: seep.stones,
                },
            ));
        }
        for (kind, at) in remains {
            floor.corpses.push(crate::corpse::Corpse {
                at,
                kind,
                died: 0,
                studied: 0,
                rendered: 0,
                ancient: true,
                spoiled: false,
            });
        }
        let spawns: Vec<(KindId, Point)> = spawns
            .into_iter()
            .filter(|(_, p)| !bosses.iter().any(|(_, b)| b == p))
            .chain(bosses.iter().copied())
            .collect();
        for (kind, pos) in spawns {
            floor
                .monsters
                .insert(Monster::new(kind, content.monster(kind), pos));
        }
        floor.tallow = tallow
            .into_iter()
            .map(|(at, amount)| Tallow::new(at, amount))
            .collect();
        floor
            .items
            .extend(items.into_iter().map(|(at, kind, count)| {
                *next_item += 1;
                FloorItem::new(
                    at,
                    Item {
                        id: crate::item::ItemId(*next_item),
                        kind,
                        count,
                    },
                )
            }));
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
        self.floor = Self::generate_floor(
            self.content,
            self.seed,
            depth,
            &mut self.next_item,
            &mut self.leavings,
        );
        self.player.pos = self.floor.arrival();
        if depth == MAX_DEPTH {
            self.begin_throne();
        }
        self.update_view();
    }

    /// Puts things in the pack. For testing.
    pub fn dev_give(&mut self, id: &str, count: u32) {
        if let Some(kind) = self.content.item_by_id(id) {
            let item = Item {
                id: self.new_item_id(),
                kind,
                count,
            };
            self.add_to_pack(item);
        }
    }

    /// Grows to `level`, taking the first boon of each draft. For testing.
    pub fn dev_level_to(&mut self, level: u32) {
        while self.player.level < level {
            let need = crate::progress::insight_for_next(self.player.level) - self.player.insight;
            self.grant_insight(need);
            while self.pending_draft().is_some() {
                self.choose_boon(0);
            }
        }
    }

    /// Moves you to open floor about `steps` from the stair down. For testing bosses.
    pub fn dev_near_stairs(&mut self, steps: u32) {
        let Some(stairs) = self.map().find(Tile::StairsDown).next() else {
            return;
        };
        let dist = crate::map::path::distances(self.map(), stairs);
        let spot = self
            .map()
            .points()
            .filter(|&p| self.map().tile(p) == Tile::Floor && self.floor.monster_at(p).is_none())
            .filter_map(|p| dist.at(p).map(|d| (d.abs_diff(steps), p)))
            .min();
        if let Some((_, p)) = spot {
            self.player.pos = p;
            self.update_view();
        }
    }

    /// Knows every rite. For testing.
    pub fn dev_learn_all_rites(&mut self) {
        self.player.rites = self.content.rite_ids().collect();
    }

    /// Sets dread directly, to pay for rites. For tests and the dev flags.
    pub fn dev_set_dread(&mut self, value: u32) {
        self.player.dread.set(value);
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
            your_hit_chance: combat::hit_chance(
                self.player_accuracy() - self.dark_penalty(m.pos),
                def.defense,
            ),
            its_hit_chance: combat::hit_chance(self.monster_strength(m).0, self.player_defense()),
            its_damage: self.monster_strength(m).1,
            in_the_dark: self.in_the_dark(m.pos),
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
        self.floor
            .monsters()
            .filter_map(|(id, m)| m.winding_up.map(|t| self.blow_area(id, t)))
            .flatten()
    }

    /// Resolves one player command and returns what happened.
    pub fn apply(&mut self, command: Command) -> Vec<Event> {
        if self.death.is_some() || self.victory.is_some() {
            return Vec::new();
        }
        self.fire_ok = self.fire_warned.take();
        let mut events = self.resolve(command);
        self.fire_ok = None;
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
            Command::MakeRoom(forget) => self.make_room(forget),
            Command::Study => self.study(),
            Command::CloseDoor => self.close_doors(),
            Command::Explore => self.explore(),
            Command::UseLeaving(id) => self.use_leaving(id),
            Command::DropLeaving(id) => self.drop_leaving(id),
            Command::Render => self.render(),
            Command::Cast { rite, target } => self.cast(rite, target),
            Command::Ascend => self.ascend(),
            Command::Flare => self.flare(),
        }
    }

    /// Steps into `dir`, attacking whatever stands there.
    pub(crate) fn player_step(&mut self, dir: Direction) -> Vec<Event> {
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
        } else if let Some(id) = self.floor.monster_at(target).filter(|&id| {
            let m = &self.floor.monsters[id];
            !m.phantom && self.content.monster(m.kind).has(|t| *t == Trait::Undying)
        }) {
            return self.push_through(id);
        } else if let Some(id) = self.floor.monster_at(target) {
            self.player_attack(id, 0, &mut events);
        } else if let Some((id, bonus)) = self.reach_target(dir) {
            self.player_attack(id, bonus, &mut events);
        } else {
            if let Some(events) = self.use_tile(target) {
                return events;
            }
            let tile = self.map().tile(target);
            if !tile.is_walkable() {
                return vec![Event::PlayerBlocked { at: target, tile }];
            }
            let here_tile = self.map().tile(self.player.pos);
            let warning = if self.floor.is_burning(target) {
                Some(Event::FireAhead { at: target })
            } else if self.anomaly_at(target).is_some_and(|a| a.revealed) {
                Some(Event::AnomalyAhead { at: target })
            } else if tile == Tile::DeepWater && here_tile != Tile::DeepWater {
                Some(Event::DeepWaterAhead { at: target })
            } else if tile == Tile::RottenFloor {
                Some(Event::RottenAhead { at: target })
            } else {
                None
            };
            if let Some(warning) = warning
                && self.fire_ok != Some(target)
            {
                self.fire_warned = Some(target);
                return vec![warning];
            }
            match self.burden() {
                Burden::Overloaded => return vec![Event::TooHeavy],
                Burden::Burdened => cost = BURDENED_MOVE,
                Burden::Light => {}
            }
            cost = cost.max(match tile {
                Tile::ShallowWater => ACTION_COST * 3 / 2,
                Tile::DeepWater => ACTION_COST * 2,
                _ => ACTION_COST,
            });
            self.player.pos = target;
            events.push(Event::PlayerMoved { to: target });
            if tile == Tile::RottenFloor {
                self.fall_through(target, &mut events);
                return events;
            }
            if tile == Tile::DeepWater && self.player.candle.is_lit() && !self.player.vigil {
                self.player.candle.snuff();
                events.push(Event::CandleDrowned);
                self.wake_leavings(crate::leavings::Wake::EnteringDarkness, &mut events);
            }
            if self.anomaly_at(target).is_some() {
                self.trip_anomaly(target, &mut events);
            }
            if self.slips(target) {
                events.push(Event::Slipped { who: Who::Player });
                cost += ACTION_COST;
            }
            if let Some(i) = self.floor.tallow.iter().position(|t| t.at == target) {
                let found = self.floor.tallow.swap_remove(i);
                let amount = self.thieving(found.amount);
                events.push(Event::TallowFound { amount });
                self.gain_tallow(amount, &mut events);
            }
        }
        self.pass_time_costing(cost, &mut events);
        events
    }

    fn toggle_candle(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let in_water = self.map().tile(self.player.pos) == Tile::DeepWater;
        if !self.player.candle.is_lit() && in_water {
            return vec![Event::CantLightInWater];
        }
        if self.player.candle.is_lit() {
            self.player.candle.snuff();
            events.push(Event::CandleSnuffed);
            self.stats.snuffs += 1;
            self.trigger(Trigger::Snuff, &mut events);
            self.wake_leavings(crate::leavings::Wake::EnteringDarkness, &mut events);
        } else if self.player.candle.light() {
            events.push(Event::CandleLit);
        } else {
            return vec![Event::CandleSpent];
        }
        self.pass_time(&mut events);
        events
    }

    /// Waits until healed, or until something happens.
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

    /// Nothing left to recover: full health.
    fn rested(&self) -> bool {
        self.player.health == self.player.max_health
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
        let at = monster.pos;
        let accuracy = self.player_accuracy() + bonus - self.dark_penalty(at);
        let chance = combat::hit_chance(accuracy, def.defense);
        if self.in_the_dark(at) {
            self.shift_dread(crate::dread::DARK_BLOW, events);
        }
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
        self.fight_noise();
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

    /// No light reaches this tile: not your candle, not a brazier, not fire.
    pub fn in_the_dark(&self, p: Point) -> bool {
        !self.floor.light(p).is_lit()
    }

    /// What striking at something on `p` costs your accuracy.
    pub fn dark_penalty(&self, p: Point) -> i32 {
        match (self.in_the_dark(p), self.has_passive(Passive::DarkSight)) {
            (false, _) => 0,
            (true, false) => combat::DARK_ACCURACY,
            (true, true) => combat::DARK_ACCURACY / 2,
        }
    }

    /// A creature's accuracy and damage where it stands: the dark makes it
    /// worse, and so does growing.
    pub fn monster_strength(&self, m: &crate::monster::Monster) -> (i32, (u32, u32)) {
        let def = self.content.monster(m.kind);
        let (mut accuracy, (mut lo, mut hi)) = (def.accuracy, def.damage);
        accuracy += m.grown as i32 * GROWN_ACCURACY;
        lo += m.grown * GROWN_DAMAGE;
        hi += m.grown * GROWN_DAMAGE;
        if self.in_the_dark(m.pos) {
            accuracy += combat::DARK_FURY_ACCURACY;
            lo += combat::DARK_FURY_DAMAGE;
            hi += combat::DARK_FURY_DAMAGE;
        }
        (accuracy, (lo, hi))
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
        if self.absorb_blow(id, damage, events) {
            return;
        }
        let monster = &mut self.floor.monsters[id];
        monster.health = monster.health.saturating_sub(damage);
        let health = monster.health;
        let chorus = self.chorus_share(id, health);
        if health > 0 {
            return;
        }
        let monster = &self.floor.monsters[id];
        let (kind, at) = (monster.kind, monster.pos);
        for other in chorus {
            if let Some(m) = self.floor.monsters.remove(other)
                && self.floor.is_visible(m.pos)
            {
                events.push(Event::MonsterDied { kind, at: m.pos });
            }
        }
        self.floor.monsters.remove(id);
        if self.floor.is_visible(at) {
            events.push(Event::MonsterDied { kind, at });
        }
        self.leave_corpse(kind, at, events);
        self.on_monster_death(kind, at, events);
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
        if taken > 0 && cause != Cause::Leaving && self.player.health > 0 {
            self.wake_leavings(crate::leavings::Wake::OnHurt, events);
        }
        if self.player.health == 0 && self.death.is_none() {
            self.death = Some(Death {
                cause,
                depth: self.depth,
                ascent: self.ascent,
                turn: self.turn(),
            });
            events.push(Event::PlayerDied { cause });
        }
    }

    /// Kinds of creature you have seen this run.
    pub fn sighted_kinds(&self) -> impl Iterator<Item = KindId> + '_ {
        self.sighted.iter().copied()
    }

    /// Kinds whose bodies you've studied this run.
    pub fn studied_kinds(&self) -> impl Iterator<Item = KindId> + '_ {
        self.studied.iter().copied()
    }

    /// The great ones you have put down this run.
    pub fn defeated(&self) -> impl Iterator<Item = KindId> + '_ {
        self.defeated.iter().copied()
    }

    /// Every Leaving you have held this run.
    pub fn leavings_taken(&self) -> &[crate::leavings::LeavingId] {
        &self.leavings_taken
    }

    pub(crate) fn witness(&mut self, kind: KindId, t: Trait) {
        self.witnessed.insert((kind, t));
    }

    pub(crate) fn shift_dread(&mut self, hundredths: i32, events: &mut Vec<Event>) {
        if let Some(band) = self.player.dread.shift(hundredths) {
            events.push(Event::DreadChanged { band });
            if hundredths > 0 && band >= dread::DreadBand::Frayed {
                self.wake_leavings(crate::leavings::Wake::AtDread, events);
            }
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
                || self.floor.has_oil(self.player.pos)
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
            .filter(|&p| self.map().tile(p).is_door())
            .collect()
    }

    fn descend(&mut self) -> Vec<Event> {
        if self.map().tile(self.player.pos) != Tile::StairsDown {
            return vec![Event::NoStairsHere];
        }
        if self.ascent > 0 {
            return vec![Event::NoGoingBack];
        }
        let mut events = vec![Event::Descended {
            depth: self.depth + 1,
        }];
        self.enter_next_floor(false, &mut events);
        self.pass_time(&mut events);
        events
    }

    /// Leaves for the floor below, arriving by the stair or, after a fall, at `landing`.
    fn enter_next_floor(&mut self, fell: bool, events: &mut Vec<Event>) {
        let fled = self.manifestation_present();
        self.depth += 1;
        self.floor = Self::generate_floor(
            self.content,
            self.seed,
            self.depth,
            &mut self.next_item,
            &mut self.leavings,
        );
        let now = self.turn();
        for c in &mut self.floor.corpses {
            c.died = now;
        }
        if self.depth == MAX_DEPTH {
            self.begin_throne();
        }
        self.player.pos = self.floor.arrival();
        if fell {
            // You land somewhere on open floor, not by the stair.
            let spots: Vec<Point> = self
                .map()
                .points()
                .filter(|&p| {
                    self.map().tile(p) == Tile::Floor && self.floor.monster_at(p).is_none()
                })
                .collect();
            if let Some(&p) = rand::seq::IndexedRandom::choose(spots.as_slice(), &mut self.ai_rng) {
                self.player.pos = p;
            }
        }
        self.player.rite_state.borrowed = None;
        if fled {
            events.push(Event::ManifestationEscaped);
            self.ease_dread(events);
        }
        self.trigger(Trigger::Descend, events);
        self.gain_insight(crate::progress::INSIGHT_NEW_FLOOR, events);
    }

    /// The boards give way: down one floor, with a bruising landing (never fatal).
    fn fall_through(&mut self, at: Point, events: &mut Vec<Event>) {
        self.floor.set_tile(at, Tile::Pit);
        let damage = combat::roll_damage(&mut self.combat_rng, (2, 4))
            .min(self.player.health.saturating_sub(1));
        events.push(Event::Fell {
            depth: self.depth + 1,
            damage,
        });
        self.player.health -= damage;
        self.enter_next_floor(true, events);
        self.pass_time(events);
    }

    /// Recomputes light and sight. Returns newly spotted landmarks and, when
    /// time has passed, first sightings of creatures.
    pub(crate) fn update_view(&mut self) -> Vec<Event> {
        let blind = self.player.blind_until.is_some();
        let vigil = self.player.vigil && self.player.candle.is_lit();
        let candle = if vigil {
            Some(VIGIL_RADIUS)
        } else {
            self.player.candle.radius()
        };
        let candle = candle.map(|radius| LightSource {
            at: self.player.pos,
            radius: if blind {
                radius.min(BLIND_RADIUS)
            } else {
                radius
            },
            color: candle::COLOR,
        });
        let feel = if self.player.dark_sight_until.is_some() {
            6
        } else if self.has_passive(Passive::DarkSight) {
            2
        } else {
            1
        };
        let borrowed = self
            .player
            .rite_state
            .borrowed
            .and_then(|(id, _, radius)| Some((self.floor.monsters.get(id)?.pos, radius)));
        let mut events = self
            .floor
            .update_view(self.player.pos, candle, feel, borrowed);
        if !self.player.candle.is_lit() && !blind {
            let glowing: Vec<Point> = self
                .floor
                .monsters
                .values()
                .filter(|m| !m.phantom && self.content.monster(m.kind).faction == Faction::Dreaming)
                .map(|m| m.pos)
                .collect();
            events.extend(self.floor.see_glows(&glowing));
        }
        events
    }

    /// Tallow found or rendered, with Tallow Thief's quarter more.
    pub(crate) fn thieving(&self, amount: u32) -> u32 {
        if self.has_passive(Passive::TallowThief) {
            amount * 5 / 4
        } else {
            amount
        }
    }

    /// Adds tallow to your candle. What you can't carry without being stuck
    /// spills at your feet, so tallow alone can never pin you in place.
    pub(crate) fn gain_tallow(&mut self, amount: u32, events: &mut Vec<Event>) {
        self.player.candle.add(amount);
        let (_, max) = self.load_limits();
        let load = self.load();
        if load <= max {
            return;
        }
        // The least tallow that brings the load back within limits (lumps go first,
        // being heavier), but never more than was just gained.
        let tallow = self.player.candle.tallow();
        let excess = load - max;
        let lighter = |spill: u32| {
            crate::candle::weight_of(tallow) - crate::candle::weight_of(tallow - spill) >= excess
        };
        let most = amount.min(tallow);
        let (mut lo, mut hi) = (0, most);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if lighter(mid) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let spill = lo;
        if spill == 0 {
            return;
        }
        self.player.candle.shed(spill);
        let here = self.player.pos;
        match self.floor.tallow.iter_mut().find(|t| t.at == here) {
            Some(pile) => pile.amount += spill,
            None => self.floor.tallow.push(Tallow::seen(here, spill)),
        }
        events.push(Event::TallowSpilled { amount: spill });
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

        // Out of candle range in a dark hall: the run starts, then stops when it's
        // seen, or (a lit candle carrying farther than you can see) when it notices you.
        let mut world = world_from(
            "##########################\n#@.......................#\n##########################",
        );
        world.spawn_monster(kind("parishioner"), Point::new(20, 1));
        let events = world.apply(Command::Run(Direction::E));
        assert!(world.player().pos.x < 19);
        assert!(
            !world
                .floor()
                .visible_monsters(world.player().pos)
                .is_empty()
                || events.iter().any(|e| matches!(e, Event::Noticed { .. }))
        );
    }

    #[test]
    fn the_dark_makes_you_worse_and_them_worse_still() {
        let mut world = world_from("##########\n#@.......#\n##########");
        let id = world.spawn_monster(kind("parishioner"), Point::new(2, 1));
        let lit = world.inspect(id).unwrap();
        assert!(!lit.in_the_dark);
        world.apply(Command::ToggleCandle);
        let dark = world.inspect(id).expect("felt beside you");
        assert!(dark.in_the_dark);
        assert_eq!(
            dark.your_hit_chance,
            lit.your_hit_chance - combat::DARK_ACCURACY as u32
        );
        assert_eq!(
            dark.its_hit_chance,
            lit.its_hit_chance + combat::DARK_FURY_ACCURACY as u32
        );
        assert_eq!(
            dark.its_damage.1,
            lit.its_damage.1 + combat::DARK_FURY_DAMAGE
        );
        let before = world.player().dread.value();
        world.apply(Command::Move(Direction::E));
        assert!(
            world.player().dread.value() >= before + 2,
            "blows in the dark feed dread"
        );
    }

    #[test]
    fn the_dreaming_leave_grave_wax() {
        let mut world = world_from("##########\n#@.......#\n##########");
        let at = Point::new(4, 1);
        world.leave_corpse(kind("inkling"), at, &mut Vec::new());
        let wax = world.floor().tallow_at(at).expect("wax left");
        assert_eq!(wax.amount, crate::corpse::wax_yield(7));
        assert!(world.floor().corpse_at(at).is_none(), "no body");
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
        // Walk around rotten boards rather than fall through them.
        let mut solid = world.map().clone();
        for p in world.map().find(Tile::RottenFloor) {
            solid.set(p, Tile::Wall);
        }
        let dist = crate::map::path::distances(&solid, stairs);
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
