//! The bots: one brain, steered by a [`Style`] that says what it may use.
//! Each one-trick style leans on a single answer (the starting weapon, the
//! light, the dark, rites, missiles, gear); `mixed` uses all of them where
//! they fit. A balanced game wins far more often with `mixed`.
//!
//! Every style shares the basics a player can't avoid: pick a boon, step off
//! marked tiles, hit what's beside it, rest when hurt, flare the Vigil
//! Candle, and walk toward wherever the run goes next. Like the old bot it
//! always knows where the stairs are.

use tallow_core::map::path;
use tallow_core::{
    Burden, Command, Direction, Event, Faction, ItemClass, ItemId, ItemKindId, MAX_DEPTH, Map,
    Monster, MonsterId, Phase, Point, RiteEffect, RiteId, RiteTarget, Stage, Tile, Trait, World,
};

/// What the candle does.
#[derive(Debug, Clone, Copy)]
pub enum CandleWay {
    /// Always burning while there's tallow.
    Lit,
    /// Always out.
    Dark,
    /// Dark to travel (below `snuff_below` tallow, nothing near), lit to fight
    /// or once dread passes `fear_light`.
    Managed { snuff_below: u32, fear_light: u32 },
}

/// When it goes after tallow and bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greed {
    /// Never renders; takes only the tallow underfoot.
    Never,
    /// Renders and fetches tallow while short.
    Short,
    /// Every stub, every body, every time.
    Always,
}

/// What it throws and shoots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missiles {
    Never,
    /// Stones at fliers it can't reach, as the old bot did.
    Fliers,
    /// Anything it can see, with everything it carries, before closing in.
    Always,
    /// Fliers, the Dreaming (holy water), and what it would rather not meet
    /// in melee.
    Smart,
}

/// How it learns and casts rites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rites {
    Never,
    /// Every rite that can be cast at anything, the moment it can.
    Spam,
    /// The right rite for the moment.
    Smart,
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub name: &'static str,
    pub about: &'static str,
    pub candle: CandleWay,
    pub greed: Greed,
    /// Picks up and wears better weapons and vestments.
    pub gear: bool,
    /// Swaps weapons to the one a creature's armour doesn't blunt.
    pub swap: bool,
    pub missiles: Missiles,
    pub rites: Rites,
    /// Drinks mending and steadying tinctures when they'd help.
    pub tinctures: bool,
    /// Uses guard and shove where they help.
    pub tactics: bool,
    /// A rite it never keeps or casts.
    pub shuns: Option<&'static str>,
}

const MANAGED: CandleWay = CandleWay::Managed {
    snuff_below: 150,
    fear_light: 60,
};

pub const STYLES: &[Style] = &[
    Style {
        name: "mixed",
        about: "everything where it fits: light and dark, gear, missiles, rites, tinctures",
        candle: CandleWay::Managed {
            snuff_below: 260,
            fear_light: 65,
        },
        greed: Greed::Short,
        gear: true,
        swap: true,
        missiles: Missiles::Smart,
        rites: Rites::Smart,
        tinctures: true,
        tactics: true,
        shuns: None,
    },
    Style {
        name: "classic",
        about: "the old bot: starting weapon, light managed, stones at fliers",
        candle: MANAGED,
        greed: Greed::Short,
        gear: false,
        swap: false,
        missiles: Missiles::Fliers,
        rites: Rites::Never,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
    Style {
        name: "starter",
        about: "the iron candlestick and nothing else",
        candle: MANAGED,
        greed: Greed::Short,
        gear: false,
        swap: false,
        missiles: Missiles::Never,
        rites: Rites::Never,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
    Style {
        name: "light",
        about: "never in the dark; every stub and every body rendered",
        candle: CandleWay::Lit,
        greed: Greed::Always,
        gear: false,
        swap: false,
        missiles: Missiles::Never,
        rites: Rites::Never,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
    Style {
        name: "dark",
        about: "candle always out, never renders",
        candle: CandleWay::Dark,
        greed: Greed::Never,
        gear: false,
        swap: false,
        missiles: Missiles::Never,
        rites: Rites::Never,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
    Style {
        name: "rites",
        about: "learns every rite it can, casts whatever it can, walks dark for dread",
        candle: CandleWay::Managed {
            snuff_below: u32::MAX,
            fear_light: 95,
        },
        greed: Greed::Short,
        gear: false,
        swap: false,
        missiles: Missiles::Never,
        rites: Rites::Spam,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
    Style {
        name: "missiles",
        about: "slings, crossbows, knives and flasks at everything; melee only when empty",
        candle: MANAGED,
        greed: Greed::Short,
        gear: false,
        swap: false,
        missiles: Missiles::Always,
        rites: Rites::Never,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
    Style {
        name: "armoury",
        about: "the best weapon and vestment it finds",
        candle: MANAGED,
        greed: Greed::Short,
        gear: true,
        swap: false,
        missiles: Missiles::Never,
        rites: Rites::Never,
        tinctures: false,
        tactics: false,
        shuns: None,
    },
];

const MIXED: Style = STYLES[0];

/// `mixed` with one answer taken away each: how much the mix needs it.
pub const ABLATIONS: &[Style] = &[
    MIXED,
    Style {
        name: "-rites",
        about: "mixed, without rites",
        rites: Rites::Never,
        ..MIXED
    },
    Style {
        name: "-banish",
        about: "mixed, never keeping Banish",
        shuns: Some("banish"),
        ..MIXED
    },
    Style {
        name: "-missiles",
        about: "mixed, without throwing or shooting",
        missiles: Missiles::Never,
        ..MIXED
    },
    Style {
        name: "-gear",
        about: "mixed, with the starting weapon and cassock only",
        gear: false,
        swap: false,
        ..MIXED
    },
    Style {
        name: "-tinctures",
        about: "mixed, without tinctures",
        tinctures: false,
        ..MIXED
    },
    Style {
        name: "-dark",
        about: "mixed, candle always lit",
        candle: CandleWay::Lit,
        ..MIXED
    },
    Style {
        name: "-light",
        about: "mixed, candle always out",
        candle: CandleWay::Dark,
        ..MIXED
    },
];

pub fn style(name: &str) -> Option<&'static Style> {
    STYLES.iter().chain(ABLATIONS).find(|s| s.name == name)
}

/// Creatures within this many tiles, in line of sight, count as near.
const DANGER_NEAR: i32 = 7;
/// Below this much tallow it counts as short.
const SHORT_BELOW: u32 = 350;
/// Stones, knives or bolts it likes to carry.
const MISSILES_WANTED: u32 = 8;
/// How far it walks for something it wants.
const FETCH: u32 = 25;

pub struct Bot {
    pub style: &'static Style,
    place: (u8, Stage),
    /// Auto-explore said there's nothing left on this floor.
    explored: bool,
    /// Where it stood lately, to notice pacing back and forth.
    recent: std::collections::VecDeque<Point>,
}

impl Bot {
    pub fn new(style: &'static Style) -> Self {
        Bot {
            style,
            place: (0, Stage::Descent),
            explored: false,
            recent: std::collections::VecDeque::new(),
        }
    }

    /// What came of the last command.
    pub fn saw(&mut self, events: &[Event]) {
        if events.iter().any(|e| matches!(e, Event::Explored)) {
            self.explored = true;
        }
    }

    /// What it's thinking, for traces.
    pub fn debug(&self, world: &World) -> String {
        let brain = Brain::new(self, world);
        format!(
            "goal {:?} explored {} near {} pacing {} stairs {:?} reach {:?}",
            brain.goal(),
            self.explored,
            brain.near.len(),
            brain.pacing,
            brain.stairs(),
            brain.stairs().map(|s| brain.reachable(s))
        )
    }

    pub fn choose(&mut self, world: &World) -> Command {
        let place = (world.depth(), world.stage());
        if place != self.place {
            self.place = place;
            self.explored = false;
        }
        self.recent.push_back(world.player().pos);
        if self.recent.len() > 16 {
            self.recent.pop_front();
        }
        Brain::new(self, world).choose()
    }
}

/// One decision's worth of looking around.
struct Brain<'a> {
    style: &'static Style,
    explored: bool,
    /// Pacing between the same few tiles: stop chasing and exploring.
    pacing: bool,
    world: &'a World,
    here: Point,
    telegraphed: Vec<Point>,
    /// Real creatures that mean it harm, nearest first, with their distance.
    hostiles: Vec<(MonsterId, i32)>,
    /// Hostiles in sight and near.
    near: Vec<(MonsterId, i32)>,
}

impl<'a> Brain<'a> {
    fn new(bot: &Bot, world: &'a World) -> Self {
        let here = world.player().pos;
        let floor = world.floor();
        let following = world.content().kind_by_id("the_following");
        let mut hostiles: Vec<(MonsterId, i32)> = floor
            .monsters()
            .filter(|(_, m)| !m.phantom && m.compelled == 0 && Some(m.kind) != following)
            .map(|(id, m)| (id, m.pos.chebyshev(here)))
            .collect();
        hostiles.sort_by_key(|&(_, d)| d);
        let near = hostiles
            .iter()
            .copied()
            .filter(|&(id, d)| d <= DANGER_NEAR && floor.in_sight(floor.monster(id).unwrap().pos))
            .collect();
        Brain {
            style: bot.style,
            explored: bot.explored,
            pacing: world.stage() == Stage::Descent
                && world.depth() < MAX_DEPTH
                && bot.recent.len() >= 16
                && {
                    let mut seen: Vec<Point> = bot.recent.iter().copied().collect();
                    seen.sort_by_key(|p| (p.x, p.y));
                    seen.dedup();
                    seen.len() <= 3
                },
            world,
            here,
            telegraphed: world.telegraphs().collect(),
            hostiles,
            near,
        }
    }

    fn monster(&self, id: MonsterId) -> &'a Monster {
        self.world.floor().monster(id).expect("listed")
    }

    fn choose(&self) -> Command {
        let world = self.world;
        if world.pending_draft().is_some() {
            return Command::ChooseBoon(0);
        }
        if let Some(new) = world.pending_rite() {
            return Command::MakeRoom(self.make_room(new));
        }
        if let Some(c) = self.dodge() {
            return c;
        }
        if let Some(c) = self.flare() {
            return c;
        }
        if let Some(c) = self.drink() {
            return c;
        }
        if let Some(c) = self.candle() {
            return c;
        }
        if let Some(c) = self.cast() {
            return c;
        }
        if let Some(c) = self.shoot() {
            return c;
        }
        if let Some(c) = self.melee() {
            return c;
        }
        if let Some(c) = self.housekeeping() {
            return c;
        }
        if let Some(c) = self.body() {
            return c;
        }
        let player = world.player();
        if player.health * 5 < player.max_health * 2
            && world.hostiles_in_view().is_empty()
            && player.dread.value() < 90
        {
            return Command::Rest;
        }
        if self.style.tinctures
            && player.health * 10 < player.max_health * 7
            && world.hostiles_in_view().is_empty()
            && player.dread.value() < 60
        {
            return Command::Rest;
        }
        if let Some(c) = self.engage() {
            return c;
        }
        self.travel()
    }

    // ── The basics ──────────────────────────────────────────────────────

    /// A tile it would step onto.
    fn free(&self, p: Point) -> bool {
        let world = self.world;
        let floor = world.floor();
        world.map().is_walkable(p)
            && floor.monster_at(p).is_none()
            && !floor.is_burning(p)
            && !matches!(world.map().tile(p), Tile::RottenFloor | Tile::DeepWater)
            && world.anomaly_at(p).is_none_or(|a| !a.revealed)
    }

    /// Step off a marked tile, staying close to whatever raised the blow so
    /// the next swing can answer it.
    fn dodge(&self) -> Option<Command> {
        if !self.telegraphed.contains(&self.here) {
            return None;
        }
        let threat = self
            .world
            .floor()
            .monsters()
            .filter(|(_, m)| m.winding_up.is_some())
            .map(|(_, m)| m.pos)
            .min_by_key(|p| p.distance_squared(self.here));
        Direction::ALL
            .into_iter()
            .filter(|&d| self.free(self.here + d) && !self.telegraphed.contains(&(self.here + d)))
            .min_by_key(|&d| threat.map_or(0, |t| (self.here + d).chebyshev(t)))
            .map(Command::Move)
    }

    /// Drive the Following back when it closes in.
    fn flare(&self) -> Option<Command> {
        let world = self.world;
        if !world.can_flare() {
            return None;
        }
        let following = world.content().kind_by_id("the_following")?;
        world
            .floor()
            .monsters()
            .any(|(_, m)| m.kind == following && m.pos.chebyshev(self.here) <= 2)
            .then_some(Command::Flare)
    }

    fn drink(&self) -> Option<Command> {
        if !self.style.tinctures {
            return None;
        }
        let world = self.world;
        let player = world.player();
        let tincture = |effect: tallow_core::item::TinctureEffect| {
            player.inventory.iter().find(|it| {
                matches!(world.content().item(it.kind).class,
                    ItemClass::Tincture { effect: e } if e == effect)
            })
        };
        let fighting = !self.near.is_empty();
        if fighting
            && player.health * 3 < player.max_health
            && let Some(it) = tincture(tallow_core::item::TinctureEffect::Mending)
        {
            return Some(Command::Use(it.id));
        }
        if player.dread.value() >= 90
            && let Some(it) = tincture(tallow_core::item::TinctureEffect::Steadying)
        {
            return Some(Command::Use(it.id));
        }
        None
    }

    fn candle(&self) -> Option<Command> {
        let world = self.world;
        let player = world.player();
        let tallow = player.candle.tallow();
        let lit = player.candle.is_lit();
        if player.vigil || tallow == 0 {
            return None;
        }
        let beside = self.beside().is_some();
        let want_lit = match self.style.candle {
            CandleWay::Lit => true,
            CandleWay::Dark => false,
            CandleWay::Managed {
                snuff_below,
                fear_light,
            } => {
                // A flier beside it and nothing to throw: the dark brings it down.
                let grounding = self.flier_beside().is_some() && self.missile().is_none();
                let writing = self.writing_beside().is_some();
                !grounding
                    && !writing
                    && (!self.near.is_empty()
                        || beside
                        || tallow >= snuff_below
                        || player.dread.value() >= fear_light)
            }
        };
        // Tracing writing needs the dark, whatever the style.
        let want_lit = want_lit && !(self.learns() && self.writing_beside().is_some());
        (want_lit != lit).then_some(Command::ToggleCandle)
    }

    /// The weakest hostile beside it (phantoms too: striking pops them).
    fn beside(&self) -> Option<(Direction, MonsterId)> {
        let floor = self.world.floor();
        let following = self.world.content().kind_by_id("the_following");
        Direction::ALL
            .into_iter()
            .filter_map(|d| floor.monster_at(self.here + d).map(|id| (d, id)))
            .filter(|&(_, id)| {
                floor
                    .monster(id)
                    .is_some_and(|m| m.compelled == 0 && Some(m.kind) != following)
            })
            .min_by_key(|&(_, id)| floor.monster(id).map_or(0, |m| m.health))
    }

    fn flier_beside(&self) -> Option<&'a Monster> {
        let (_, id) = self.beside()?;
        let m = self.world.floor().monster(id)?;
        self.flies(m).then_some(m)
    }

    fn flies(&self, m: &Monster) -> bool {
        self.world
            .content()
            .monster(m.kind)
            .has(|t| matches!(t, Trait::Flies { .. }))
    }

    fn learns(&self) -> bool {
        self.style.rites != Rites::Never
    }

    fn is_boss(&self, m: &Monster) -> bool {
        self.world.content().monster(m.kind).boss
    }

    fn faction(&self, m: &Monster) -> Faction {
        self.world.content().monster(m.kind).faction
    }

    // ── Melee ───────────────────────────────────────────────────────────

    fn melee(&self) -> Option<Command> {
        let (dir, id) = self.beside()?;
        let world = self.world;
        let m = self.monster(id);
        // Swap to a weapon its armour doesn't blunt.
        if self.style.swap
            && !m.phantom
            && world.resists_wielded(m)
            && let Some(better) = self.best_weapon_against(Some(self.faction(m)))
            && Some(better) != world.player().equipment.melee
        {
            return Some(Command::Equip(better));
        }
        // A flier aloft: stones if it has any.
        if self.style.missiles != Missiles::Never
            && self.flies(m)
            && world.aloft(m)
            && let Some(item) = self.missile()
        {
            return Some(Command::Throw {
                item,
                target: m.pos,
            });
        }
        if self.style.tactics
            && m.winding_up.is_none()
            && self.surrounded() >= 3
            && world.player().health * 2 < world.player().max_health
        {
            return Some(Command::Guard);
        }
        Some(Command::Move(dir))
    }

    fn surrounded(&self) -> usize {
        self.hostiles.iter().filter(|&&(_, d)| d <= 1).count()
    }

    /// Average blow with a weapon against a faction, counting hit chance.
    fn weapon_score(&self, kind: ItemKindId, against: Option<Faction>) -> u32 {
        let ItemClass::Melee {
            family,
            damage,
            accuracy,
            quirk,
        } = &self.world.content().item(kind).class
        else {
            return 0;
        };
        // In tenths of a point; it fights lit, and a wound bleeds about
        // half its course before the fight ends.
        let mut avg = (damage.0 + damage.1) * 5;
        match quirk {
            Some(tallow_core::item::Quirk::Candlelit { damage }) => avg += damage * 10,
            Some(tallow_core::item::Quirk::Bleeds { turns }) => avg += turns * 5,
            _ => {}
        }
        let halved = matches!(
            (against, family),
            (Some(Faction::Swarm), tallow_core::item::Family::Bludgeon)
                | (Some(Faction::Remnant), tallow_core::item::Family::Blade)
        );
        if halved {
            avg /= 2;
        }
        avg * (80 + accuracy).clamp(5, 95) as u32
    }

    /// The factions a swapping style keeps a weapon for.
    fn armours(&self) -> &'static [Option<Faction>] {
        if self.style.swap {
            &[None, Some(Faction::Swarm), Some(Faction::Remnant)]
        } else {
            &[None]
        }
    }

    /// The carried weapons worth keeping: the best against each armour.
    fn weapons_kept(&self) -> Vec<ItemId> {
        self.armours()
            .iter()
            .filter_map(|&f| self.best_weapon_against(f))
            .collect()
    }

    /// A weapon kind better than anything carried, against some armour.
    fn beats_carried(&self, kind: ItemKindId) -> bool {
        let world = self.world;
        self.armours().iter().any(|&f| {
            let best = self
                .best_weapon_against(f)
                .and_then(|id| world.inventory_item(id))
                .map_or(0, |it| self.weapon_score(it.kind, f));
            self.weapon_score(kind, f) > best
        })
    }

    fn best_weapon_against(&self, against: Option<Faction>) -> Option<ItemId> {
        let world = self.world;
        world
            .player()
            .inventory
            .iter()
            .filter(|it| matches!(world.content().item(it.kind).class, ItemClass::Melee { .. }))
            .max_by_key(|it| self.weapon_score(it.kind, against))
            .map(|it| it.id)
    }

    /// Walk at visible creatures to fight them, while healthy and before a
    /// floor is done; the missile style keeps its distance.
    fn engage(&self) -> Option<Command> {
        let world = self.world;
        let player = world.player();
        if world.stage() != Stage::Descent
            || self.pacing
            || player.health * 2 < player.max_health
            || self.style.missiles == Missiles::Always && self.missile_ready()
        {
            return None;
        }
        let &(id, d) = self.near.first()?;
        let m = self.monster(id);
        if self.is_boss(m) && world.depth() != MAX_DEPTH || d > 6 {
            return None;
        }
        self.step_toward(m.pos)
    }

    // ── Missiles ────────────────────────────────────────────────────────

    /// A stone, knife or bolt to throw by hand.
    fn missile(&self) -> Option<ItemId> {
        let world = self.world;
        world
            .player()
            .inventory
            .iter()
            .filter(|it| is_missile(world, it.kind))
            .max_by_key(|it| {
                world
                    .content()
                    .item(it.kind)
                    .thrown
                    .map_or(0, |t| t.damage.1)
            })
            .map(|it| it.id)
    }

    fn missile_ready(&self) -> bool {
        self.missile().is_some() || self.can_fire()
    }

    fn can_fire(&self) -> bool {
        let world = self.world;
        let Some(ranged) = world.player().equipment.ranged else {
            return false;
        };
        let Some(item) = world.inventory_item(ranged) else {
            return false;
        };
        let ItemClass::Ranged { ammo, .. } = &world.content().item(item.kind).class else {
            return false;
        };
        let ammo = world.content().item_by_id(ammo);
        world
            .player()
            .inventory
            .iter()
            .any(|it| Some(it.kind) == ammo)
    }

    /// The flight toward `target` ends on it.
    fn clear_shot(&self, target: Point, range: i32) -> bool {
        self.world.aim(target, range).last() == Some(&target)
    }

    fn shoot(&self) -> Option<Command> {
        let style = self.style.missiles;
        if matches!(style, Missiles::Never | Missiles::Fliers) {
            return None;
        }
        let world = self.world;
        for &(id, d) in &self.near {
            let m = self.monster(id);
            if !world.floor().is_visible(m.pos) {
                continue;
            }
            let faction = self.faction(m);
            let worth = match style {
                Missiles::Always => true,
                _ => {
                    // Smart: the Dreaming, fliers, whatever winds up heavy
                    // blows, and bosses; not swarm vermin worth a swing.
                    faction == Faction::Dreaming
                        || self.flies(m)
                        || self.is_boss(m)
                        || world.content().monster(m.kind).heavy_blow().is_some()
                }
            };
            if !worth || d <= 1 && style == Missiles::Smart {
                continue;
            }
            if let Some(c) = self.throw_at(m, d) {
                return Some(c);
            }
        }
        None
    }

    fn throw_at(&self, m: &Monster, d: i32) -> Option<Command> {
        let world = self.world;
        let faction = self.faction(m);
        let inv = &world.player().inventory;
        let find = |want: &dyn Fn(&tallow_core::item::ThrowStats) -> bool| {
            inv.iter()
                .find(|it| {
                    world
                        .content()
                        .item(it.kind)
                        .thrown
                        .is_some_and(|t| want(&t))
                })
                .map(|it| it.id)
        };
        let hand = self.clear_shot(m.pos, tallow_core::inventory::THROW_RANGE);
        // Holy water on the Dreaming and the Taken.
        if hand
            && matches!(faction, Faction::Dreaming | Faction::Taken)
            && let Some(item) = find(&|t| t.holy.is_some())
        {
            return Some(Command::Throw {
                item,
                target: m.pos,
            });
        }
        // Fire well away from itself.
        if hand
            && d >= 3
            && faction != Faction::Dreaming
            && let Some(item) = find(&|t| t.fire)
        {
            return Some(Command::Throw {
                item,
                target: m.pos,
            });
        }
        if self.can_fire()
            && let Some(range) = world.fire_range()
            && self.clear_shot(m.pos, range)
        {
            return Some(Command::Fire { target: m.pos });
        }
        if hand && let Some(item) = self.missile() {
            return Some(Command::Throw {
                item,
                target: m.pos,
            });
        }
        None
    }

    // ── Rites ───────────────────────────────────────────────────────────

    fn cast(&self) -> Option<Command> {
        if self.style.rites == Rites::Never || self.near.is_empty() {
            return None;
        }
        let world = self.world;
        for &rite in world.known_rites() {
            if self.shunned(rite) {
                continue;
            }
            if let Some(target) = self.rite_target(rite) {
                return Some(Command::Cast { rite, target });
            }
        }
        None
    }

    /// Where to cast `rite` now, if anywhere.
    fn rite_target(&self, rite: RiteId) -> Option<Point> {
        let world = self.world;
        let def = world.content().rite(rite);
        let spam = self.style.rites == Rites::Spam;
        let ok = |p: Point| world.rite_check(rite, p).is_ok();
        let player = world.player();
        let hurt = player.health * 2 < player.max_health;
        let hunting = self
            .near
            .iter()
            .filter(|&&(id, _)| matches!(self.monster(id).mind, tallow_core::Mind::Hunting { .. }))
            .count();
        match def.effect.target() {
            RiteTarget::Myself => {
                if !ok(self.here) {
                    return None;
                }
                let wanted = spam
                    || match def.effect {
                        RiteEffect::Sanctify { .. } => self.near.iter().any(|&(id, d)| {
                            d <= 4 && self.faction(self.monster(id)) == Faction::Dreaming
                        }),
                        RiteEffect::Shroud { .. } => hurt && hunting > 0,
                        _ => false,
                    };
                wanted.then_some(self.here)
            }
            RiteTarget::Tile => {
                if !spam && !(hurt && hunting >= 2) {
                    return None;
                }
                // As far from itself as it reaches.
                self.world
                    .map()
                    .points()
                    .filter(|&p| p.chebyshev(self.here) >= 4 && ok(p))
                    .max_by_key(|p| p.chebyshev(self.here))
            }
            RiteTarget::Door => {
                if !spam {
                    return None;
                }
                self.world
                    .map()
                    .points()
                    .filter(|&p| p.chebyshev(self.here) <= def.range)
                    .find(|&p| ok(p))
            }
            RiteTarget::Creature => self.near.iter().find_map(|&(id, d)| {
                let m = self.monster(id);
                if !ok(m.pos) {
                    return None;
                }
                let wanted = spam || self.smart_on(def.effect, m, d, hurt, hunting);
                wanted.then_some(m.pos)
            }),
        }
    }

    fn smart_on(
        &self,
        effect: RiteEffect,
        m: &Monster,
        d: i32,
        hurt: bool,
        hunting: usize,
    ) -> bool {
        let world = self.world;
        let big = m.health >= 12 || self.is_boss(m);
        let awake = matches!(m.mind, tallow_core::Mind::Hunting { .. });
        match effect {
            RiteEffect::Banish => awake,
            RiteEffect::Compel { .. } => awake && hunting >= 2 && !self.is_boss(m),
            RiteEffect::Leech { .. } => hurt || m.health <= 5,
            RiteEffect::Kneel { .. } => awake && big && d >= 2,
            RiteEffect::Exorcise { .. } => big || hurt,
            RiteEffect::Transference { .. } => world.player().dread.value() >= 80,
            RiteEffect::Unsee { .. } => hurt && awake,
            RiteEffect::Turncoat => {
                awake
                    && self
                        .near
                        .iter()
                        .filter(|&&(o, _)| self.monster(o).kind == m.kind)
                        .count()
                        >= 3
            }
            RiteEffect::Beckon { .. } => false,
            RiteEffect::Exchange => false,
            _ => false,
        }
    }

    fn shunned(&self, rite: RiteId) -> bool {
        self.style.shuns == Some(self.world.content().rite(rite).id.as_str())
    }

    /// With every slot full, forget the least useful rite for a better one.
    fn make_room(&self, new: RiteId) -> Option<RiteId> {
        let world = self.world;
        let rank = |r: RiteId| {
            if self.shunned(r) {
                return 99;
            }
            const ORDER: [&str; 16] = [
                "banish",
                "compel",
                "leech",
                "kneel",
                "exorcise",
                "sanctify",
                "transference",
                "unsee",
                "turncoat",
                "shroud",
                "false_flame",
                "exchange",
                "beckon",
                "seal",
                "hush",
                "borrowed_eyes",
            ];
            let id = &world.content().rite(r).id;
            ORDER.iter().position(|o| o == id).unwrap_or(ORDER.len())
        };
        let worst = world
            .known_rites()
            .iter()
            .copied()
            .max_by_key(|&r| rank(r))?;
        (rank(new) < rank(worst)).then_some(worst)
    }

    // ── Bodies, writing, texts, gear ────────────────────────────────────

    fn short(&self) -> bool {
        let world = self.world;
        world.stage() == Stage::Descent
            && match self.style.greed {
                Greed::Never => false,
                Greed::Short => world.player().candle.tallow() < SHORT_BELOW,
                Greed::Always => world.burden() == Burden::Light,
            }
    }

    /// Study or render the body underfoot.
    fn body(&self) -> Option<Command> {
        let world = self.world;
        let corpse = world.corpse_here()?;
        if !world.hostiles_in_view().is_empty() {
            return None;
        }
        if self.learns()
            && corpse.rendered == 0
            && !corpse.spoiled
            && world.study_would_teach(corpse.kind)
            && teaches_rite(world, corpse.kind)
            && !refused(world, Command::Study)
        {
            return Some(Command::Study);
        }
        if self.short() && !corpse.spoiled && !refused(world, Command::Render) {
            return Some(Command::Render);
        }
        None
    }

    /// Read texts, wear better gear, drop what it has outgrown.
    fn housekeeping(&self) -> Option<Command> {
        let world = self.world;
        if !world.hostiles_in_view().is_empty() {
            return None;
        }
        let player = world.player();
        let content = world.content();
        if self.learns()
            && let Some(text) = player
                .inventory
                .iter()
                .find(|it| matches!(content.item(it.kind).class, ItemClass::Text { .. }))
        {
            return Some(Command::Use(text.id));
        }
        if self.style.gear {
            if let Some(best) = self.best_weapon_against(None)
                && Some(best) != player.equipment.melee
            {
                return Some(Command::Equip(best));
            }
            let best_vest = player
                .inventory
                .iter()
                .filter_map(|it| match content.item(it.kind).class {
                    ItemClass::Vestment { defense } => Some((defense, it.id)),
                    _ => None,
                })
                .max();
            if let Some((_, best)) = best_vest
                && Some(best) != player.equipment.body
            {
                return Some(Command::Equip(best));
            }
            // Drop outgrown weapons and vestments to stay light.
            let keep = self.weapons_kept();
            if let Some(old) = player.inventory.iter().find(|it| {
                let class = &content.item(it.kind).class;
                !player.equipment.contains(it.id)
                    && (matches!(class, ItemClass::Vestment { .. })
                        || matches!(class, ItemClass::Melee { .. }) && !keep.contains(&it.id))
            }) {
                return Some(Command::Drop(old.id));
            }
        }
        if matches!(self.style.missiles, Missiles::Always | Missiles::Smart)
            && player.equipment.ranged.is_none()
            && let Some(ranged) = player
                .inventory
                .iter()
                .find(|it| matches!(content.item(it.kind).class, ItemClass::Ranged { .. }))
        {
            return Some(Command::Equip(ranged.id));
        }
        None
    }

    /// Floor items this style wants.
    fn wants(&self, kind: ItemKindId) -> bool {
        let world = self.world;
        let content = world.content();
        let def = content.item(kind);
        let player = world.player();
        let carried = |pred: &dyn Fn(ItemKindId) -> bool| -> u32 {
            player
                .inventory
                .iter()
                .filter(|it| pred(it.kind))
                .map(|it| it.count)
                .sum()
        };
        let room = |weight: u32| {
            let (light, _) = world.load_limits();
            world.load() + weight <= light
        };
        match &def.class {
            ItemClass::Text { .. } => self.learns(),
            ItemClass::Tincture { .. } => self.style.tinctures && room(def.weight),
            ItemClass::Melee { .. } => {
                self.style.gear && room(def.weight) && self.beats_carried(kind)
            }
            ItemClass::Vestment { defense } => {
                self.style.gear
                    && room(def.weight)
                    && player.equipment.body.is_none_or(|b| {
                        world.inventory_item(b).is_none_or(|b| {
                            matches!(content.item(b.kind).class,
                                ItemClass::Vestment { defense: old } if *defense > old)
                        })
                    })
            }
            ItemClass::Ranged { .. } => {
                matches!(self.style.missiles, Missiles::Always | Missiles::Smart)
                    && player.equipment.ranged.is_none()
                    && room(def.weight)
            }
            _ => match self.style.missiles {
                Missiles::Never => false,
                Missiles::Fliers => {
                    is_missile(world, kind) && carried(&|k| is_missile(world, k)) < 6
                }
                Missiles::Always | Missiles::Smart => {
                    def.thrown.is_some_and(|t| t.noise.is_none() && !t.oil)
                        && carried(&|k| world.content().item(k).thrown.is_some())
                            < MISSILES_WANTED * 2
                        && room(def.weight)
                }
            },
        }
    }

    fn writing_beside(&self) -> Option<Direction> {
        let floor = self.world.floor();
        Direction::ALL.into_iter().find(|&d| {
            floor
                .writings()
                .iter()
                .any(|w| w.at == self.here + d && w.seen && !w.read)
        })
    }

    // ── Getting about ───────────────────────────────────────────────────

    fn travel(&self) -> Command {
        let world = self.world;
        if self.learns()
            && let Some(d) = self.writing_beside()
            && !world.player().candle.is_lit()
        {
            return Command::Move(d);
        }
        let mut goal = if self.pacing { None } else { self.goal() };
        if goal.is_none() && self.wants_to_explore() && world.hostiles_in_view().is_empty() {
            return Command::ExploreStep;
        }
        if goal.is_none() {
            goal = self.stairs();
        }
        let Some(mut goal) = goal else {
            return Command::Wait;
        };
        if !self.reachable(goal)
            && let Some(stairs) = world.map().find(Tile::StairsDown).next()
        {
            goal = stairs;
        }
        if goal == self.here {
            return match world.stage() {
                Stage::Descent if world.map().tile(self.here) == Tile::StairsDown => {
                    Command::Descend
                }
                _ if world.map().tile(self.here) == Tile::StairsUp => Command::Ascend,
                _ => Command::PickUp,
            };
        }
        self.step_toward(goal).unwrap_or(Command::Wait)
    }

    /// Explore the floor while the candle (or for the dark, dread) allows.
    fn wants_to_explore(&self) -> bool {
        let world = self.world;
        if self.explored
            || self.pacing
            || world.stage() != Stage::Descent
            || world.depth() == MAX_DEPTH
        {
            return false;
        }
        let player = world.player();
        match self.style.candle {
            CandleWay::Dark => player.dread.value() < 70,
            _ => player.candle.tallow() >= 180,
        }
    }

    fn stairs(&self) -> Option<Point> {
        let map = self.world.map();
        match self.world.stage() {
            Stage::Descent => map.find(Tile::StairsDown).next(),
            Stage::Ascent(_) => map.find(Tile::StairsUp).next(),
            Stage::Church => map.find(Tile::Altar).next(),
        }
    }

    /// A walkable map without seep rooms, rotten boards or deep water.
    fn walkable(&self, goal: Point) -> Map {
        let world = self.world;
        let mut solid = safe_map(world.map());
        for p in world.map().points() {
            if world.floor().is_seep(p) && p != goal {
                solid.set(p, Tile::Wall);
            }
        }
        solid.set(goal, Tile::Floor);
        solid
    }

    fn reachable(&self, goal: Point) -> bool {
        path::distances(&self.walkable(goal), goal)
            .at(self.here)
            .is_some()
    }

    fn step_toward(&self, goal: Point) -> Option<Command> {
        let world = self.world;
        let mut dist = path::distances(&self.walkable(goal), goal);
        // Cut off from it by water, rotten boards or a seep room: risk them.
        let risky = dist.at(self.here).is_none();
        if risky {
            let mut open = world.map().clone();
            open.set(goal, Tile::Floor);
            dist = path::distances(&open, goal);
        }
        let solid_goal = !world.map().is_walkable(goal) || world.floor().monster_at(goal).is_some();
        let can_step = |p: Point| {
            let open = if risky {
                world.map().is_walkable(p)
                    && world.floor().monster_at(p).is_none()
                    && !world.floor().is_burning(p)
            } else {
                self.free(p)
            };
            (open || (solid_goal && p == goal)) && !self.telegraphed.contains(&p)
        };
        let best = Direction::ALL
            .into_iter()
            .filter(|&d| can_step(self.here + d))
            .filter_map(|d| {
                let p = self.here + d;
                if p == goal {
                    Some((0, d))
                } else {
                    dist.at(p).map(|n| (n, d))
                }
            })
            .min_by_key(|&(n, _)| n);
        best.map(|(_, d)| Command::Move(d))
    }

    /// Where it wants to be next, short of the stairs.
    fn goal(&self) -> Option<Point> {
        let world = self.world;
        let floor = world.floor();
        match world.stage() {
            Stage::Church => world.map().find(Tile::Altar).next(),
            Stage::Ascent(_) => world.map().find(Tile::StairsUp).next(),
            Stage::Descent if world.depth() == MAX_DEPTH => {
                let lord = world.lord()?;
                if world.player().vigil {
                    return world.map().find(Tile::StairsUp).next();
                }
                if lord.phase == Phase::Fallen {
                    return floor
                        .items()
                        .iter()
                        .find(|f| {
                            matches!(world.content().item(f.item.kind).class, ItemClass::Relic)
                        })
                        .map(|f| f.at);
                }
                floor
                    .monsters()
                    .filter(|(_, m)| self.is_boss(m))
                    .map(|(_, m)| m.pos)
                    .min_by_key(|p| p.distance_squared(self.here))
            }
            Stage::Descent => {
                let dist = path::distances(&safe_map(world.map()), self.here);
                let light = world.burden() == Burden::Light;
                // Tallow past the candle's cap is carried as heavy lumps:
                // only the greedy take it.
                let held = world.player().candle.tallow();
                let greedy = match self.style.greed {
                    Greed::Always => true,
                    Greed::Short => held < tallow_core::candle::CAP,
                    Greed::Never => held < 100,
                };
                let tallow = floor
                    .tallow()
                    .iter()
                    .filter(|t| greedy && light && floor.is_explored(t.at) && t.at != self.here)
                    .map(|t| t.at);
                let items = floor
                    .items()
                    .iter()
                    .filter(|f| light && floor.is_explored(f.at) && self.wants(f.item.kind))
                    .map(|f| f.at);
                let short = self.short();
                let bodies = floor
                    .corpses()
                    .iter()
                    .filter(|c| {
                        light
                            && floor.is_explored(c.at)
                            && c.at != self.here
                            && !c.spoiled
                            && (short
                                || self.learns()
                                    && c.rendered == 0
                                    && world.study_would_teach(c.kind)
                                    && teaches_rite(world, c.kind))
                    })
                    .map(|c| c.at);
                let writing = floor
                    .writings()
                    .iter()
                    .filter(|w| self.learns() && w.seen && !w.read)
                    .map(|w| w.at);
                let walk = |at: Point| {
                    // Writing is on a wall: measure to the tile beside it.
                    if world.map().is_walkable(at) {
                        dist.at(at)
                    } else {
                        Direction::ALL
                            .into_iter()
                            .filter_map(|d| dist.at(at + d))
                            .min()
                    }
                };
                tallow
                    .chain(items)
                    .chain(bodies)
                    .chain(writing)
                    .filter_map(|at| walk(at).filter(|&d| d <= FETCH).map(|d| (d, at)))
                    .min()
                    .map(|(_, at)| at)
            }
        }
    }
}

/// Studying this kind would teach a rite (not just Insight).
fn teaches_rite(world: &World, kind: tallow_core::KindId) -> bool {
    world
        .content()
        .monster(kind)
        .teaches
        .iter()
        .filter_map(|id| world.content().rite_by_id(id))
        .any(|r| !world.knows(r))
}

/// A command would be refused right now.
fn refused(world: &World, command: Command) -> bool {
    world.clone().apply(command) == [Event::RunRefused]
}

/// Something to throw at fliers: stones, knives, bolts; not flasks or bells.
pub fn is_missile(world: &World, kind: ItemKindId) -> bool {
    world
        .content()
        .item(kind)
        .thrown
        .is_some_and(|t| !t.breaks && t.noise.is_none())
}

pub fn safe_map(map: &Map) -> Map {
    let mut solid = map.clone();
    for p in map.points() {
        if matches!(map.tile(p), Tile::RottenFloor | Tile::DeepWater | Tile::Pit) {
            solid.set(p, Tile::Wall);
        }
    }
    solid
}
