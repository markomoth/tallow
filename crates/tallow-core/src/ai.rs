//! How monsters decide what to do on their action.

use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::boons::Trigger;
use crate::combat;
use crate::content::{Faction, Trait};
use crate::events::{Cause, Event, Who};
use crate::geom::{Direction, Point};
use crate::item::Family;
use crate::map::{Tile, path};
use crate::monster::{Mind, MonsterId};
use crate::progress::Source;
use crate::rites::{DECOY_IGNORED_WITHIN, DECOY_PULL};
use crate::skills::{Skill, Technique};
use crate::world::World;

/// Packmates closer than this keep a pack creature brave.
const PACK_RADIUS: i32 = 4;
/// Chance an unaware monster shuffles somewhere on its action.
const WANDER_CHANCE: f64 = 0.3;
/// With your candle out, how close something must be to notice you.
const DARK_NOTICE: i32 = 2;
/// A lit candle is seen this much farther than a creature's own sight...
const LIGHT_REACH: i32 = 4;
/// ...and the Dreaming come to it like moths from farther still.
const LIGHT_REACH_DREAMING: i32 = 8;
/// Tallow a light-eating bite takes.
const TALLOW_BITTEN: u32 = 15;
/// A creature gives up on a foe farther away than this.
const FOE_RANGE: i32 = 10;
/// A thrall with nothing to fight stays this close to you.
const HEEL: i32 = 2;
/// Creatures attack a hated faction this close, if it's nearer than you.
const AGGRO_RANGE: i32 = 5;

impl World {
    pub(crate) fn monster_act(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        // An open wound bleeds before it does anything else.
        if self.floor.monsters[id].bleeding > 0 {
            self.floor.monsters[id].bleeding -= 1;
            self.damage_monster(id, 1, Source::Melee(Some(Family::Blade)), events);
            if !self.floor.monsters.contains_key(id) {
                return;
            }
        }
        self.monster_decide(id, events);
        // A pin holds for whole actions: it wears off once the action is spent.
        if let Some(m) = self.floor.monsters.get_mut(id) {
            m.pinned = m.pinned.saturating_sub(1);
        }
    }

    fn monster_decide(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        let content = self.content;
        let m = self.floor.monsters[id].clone();
        let def = content.monster(m.kind);
        let kind = m.kind;
        let player = self.player.pos;

        if m.phantom {
            // Phantoms come for you and come apart before they touch you.
            if m.pos.chebyshev(player) <= 1 {
                self.floor.monsters.remove(id);
                self.phantom_bite(kind, events);
            } else {
                self.step_toward(id, player);
            }
            return;
        }

        // Rites wear off one action at a time.
        let monster = &mut self.floor.monsters[id];
        if monster.stunned > 0 {
            monster.stunned -= 1;
            return;
        }
        monster.unseeing = monster.unseeing.saturating_sub(1);
        monster.terrified = monster.terrified.saturating_sub(1);
        if std::mem::take(&mut monster.flinching) {
            if self.floor.is_visible(m.pos) {
                events.push(Event::Flinched { kind });
            }
            return;
        }
        if std::mem::take(&mut monster.slipping) {
            if self.floor.is_visible(m.pos) {
                events.push(Event::Slipped {
                    who: Who::Monster(kind),
                });
            }
            return;
        }
        if m.beckoned > 0 {
            if m.pos.chebyshev(player) <= 1 {
                let monster = &mut self.floor.monsters[id];
                monster.beckoned = 0;
                monster.mind = Mind::Hunting { last_seen: player };
            } else {
                self.step_toward(id, player);
                if let Some(monster) = self.floor.monsters.get_mut(id) {
                    monster.beckoned -= 1;
                }
            }
            return;
        }
        if m.compelled > 0 {
            self.thrall_act(id, events);
            if let Some(monster) = self.floor.monsters.get_mut(id) {
                monster.compelled -= 1;
                if monster.compelled == 0 {
                    monster.mind = Mind::Hunting { last_seen: player };
                    monster.foe = None;
                    let at = monster.pos;
                    if self.floor.is_visible(at) {
                        events.push(Event::CompelEnded { kind });
                    }
                }
            }
            return;
        }
        // The Dreaming can't abide holy ground; caught on it, they get off it.
        if def.faction == Faction::Dreaming && self.floor.is_sanctified(m.pos) {
            self.step_by(id, |_, _| Some(0));
            return;
        }

        // Your candle (or a brazier) shows you from afar. In the dark, only
        // something close by notices you. Shroud hides even a lit candle.
        let relentless = def.has(|t| *t == Trait::Relentless);
        let conspicuous = !self.shrouded()
            && (self.player.candle.is_lit() || self.floor.ambient_light(player).is_lit());
        // A lit candle carries: it is seen from beyond a creature's own sight.
        let reach = def.sight
            + if self.player.candle.is_lit() && !self.shrouded() {
                if def.faction == Faction::Dreaming {
                    LIGHT_REACH_DREAMING
                } else {
                    LIGHT_REACH
                }
            } else {
                0
            };
        let in_range = m.pos.distance_squared(player) <= reach * reach;
        let close = m.pos.chebyshev(player) <= DARK_NOTICE;
        let sees = relentless
            || (m.unseeing == 0
                && self.floor.in_sight(m.pos)
                && in_range
                && (conspicuous || close));

        // A raised blow is held until you've had a turn, then comes down on the
        // tile it was aimed at.
        if m.winding_up.is_some() && !m.blow_ready {
            return;
        }
        if let Some(target) = m.winding_up {
            let ((lo, hi), cooldown) = def.heavy_blow().expect("only heavy hitters wind up");
            let monster = &mut self.floor.monsters[id];
            monster.winding_up = None;
            monster.cooldown = cooldown;
            let braced = self
                .techniques(Skill::Endurance)
                .into_iter()
                .find_map(|t| match t {
                    Technique::Brace { percent } => Some(percent),
                    _ => None,
                });
            let area = self.blow_area(id, target);
            let damage = area.contains(&player).then(|| {
                let raw = combat::roll_damage(&mut self.combat_rng, (lo, hi));
                raw - raw * braced.unwrap_or(0) / 100
            });
            events.push(Event::HeavyBlow {
                kind,
                target,
                damage,
            });
            if let Some(damage) = damage {
                self.hurt_player(damage, Cause::HeavyBlow(kind), events);
            } else if self.floor.is_visible(target) {
                self.trigger(Trigger::DodgeHeavyBlow, events);
            }
            return;
        }
        // A marked leap waits for your turn too, then comes down.
        if m.lunging.is_some() && !m.blow_ready {
            return;
        }
        if let Some(target) = m.lunging {
            self.land_lunge(id, target, events);
            return;
        }
        let monster = &mut self.floor.monsters[id];
        monster.cooldown = monster.cooldown.saturating_sub(1);

        // Rival factions go for each other when they meet, if you aren't nearer.
        let mut foe = m.foe;
        if foe.is_none()
            && let Some((enemy, d)) = self.nearest_enemy(id, AGGRO_RANGE)
            && (!sees || d < m.pos.chebyshev(player))
        {
            self.floor.monsters[id].foe = Some(enemy);
            foe = Some(enemy);
        }
        // A creature fighting another keeps at it.
        if let Some(foe) = foe {
            match self.floor.monsters.get(foe).map(|f| f.pos) {
                Some(at) if at.chebyshev(m.pos) <= FOE_RANGE => {
                    if at.chebyshev(m.pos) == 1 {
                        self.monster_strike(id, foe, events);
                    } else {
                        self.step_toward(id, at);
                    }
                    return;
                }
                _ => self.floor.monsters[id].foe = None,
            }
        }
        if m.terrified > 0 {
            self.step_away(id);
            return;
        }
        // A False Flame draws everything near it, unless you are closer still.
        if let Some(decoy) = self.floor.decoy
            && !relentless
            && m.pos.chebyshev(player) > DECOY_IGNORED_WITHIN
            && m.pos.chebyshev(decoy.at) <= DECOY_PULL
        {
            if def.has(|t| *t == Trait::EatsLight) && m.pos.chebyshev(decoy.at) <= 1 {
                self.floor.decoy = None;
                if self.floor.is_visible(m.pos) || self.floor.is_visible(decoy.at) {
                    events.push(Event::FlameEaten { kind });
                    self.witness(kind, Trait::EatsLight);
                }
                return;
            }
            self.floor.monsters[id].mind = Mind::Hunting {
                last_seen: decoy.at,
            };
            if m.pos != decoy.at {
                self.step_toward(id, decoy.at);
            }
            return;
        }

        match m.mind {
            Mind::Unaware if sees => {
                self.floor.monsters[id].mind = Mind::Hunting { last_seen: player };
                events.push(Event::Noticed {
                    kind,
                    seen: self.floor.is_visible(m.pos),
                });
                if def.has(|t| *t == Trait::Pleads) && !def.barks.is_empty() {
                    let line = self.ai_rng.random_range(0..def.barks.len());
                    events.push(Event::Bark { kind, line });
                    self.witness(kind, Trait::Pleads);
                }
                return;
            }
            Mind::Unaware => {
                if self.ai_rng.random_bool(WANDER_CHANCE) {
                    self.wander(id);
                }
                return;
            }
            Mind::Hunting { .. } if sees => {
                self.floor.monsters[id].mind = Mind::Hunting { last_seen: player };
            }
            _ => {}
        }

        if def.has(|t| *t == Trait::PackCourage) {
            let packmates = self
                .floor
                .monsters
                .iter()
                .filter(|&(other, o)| {
                    other != id && o.kind == kind && o.pos.chebyshev(m.pos) <= PACK_RADIUS
                })
                .count();
            let mind = self.floor.monsters[id].mind;
            if packmates == 0 && mind != Mind::Fleeing {
                self.floor.monsters[id].mind = Mind::Fleeing;
                if self.floor.is_visible(m.pos) {
                    events.push(Event::Fled { kind });
                    self.witness(kind, Trait::PackCourage);
                }
            } else if packmates > 0 && mind == Mind::Fleeing {
                self.floor.monsters[id].mind = Mind::Hunting { last_seen: player };
            }
        }

        if self.tactics(id, sees, events) {
            return;
        }
        if matches!(self.floor.monsters[id].mind, Mind::Hunting { .. })
            && self.use_ability(id, sees, events)
        {
            return;
        }
        let adjacent = m.pos.chebyshev(player) == 1;
        match self.floor.monsters[id].mind {
            Mind::Hunting { last_seen } => {
                if adjacent {
                    if let Some(heavy) = def
                        .traits
                        .iter()
                        .find(|t| matches!(t, Trait::HeavyBlow { .. }))
                        && m.cooldown == 0
                    {
                        let monster = &mut self.floor.monsters[id];
                        monster.winding_up = Some(player);
                        monster.blow_ready = false;
                        self.stats.heavy_blows_seen += 1;
                        events.push(Event::WindUp {
                            kind,
                            target: player,
                        });
                        self.witness(kind, *heavy);
                    } else if def.has(|t| *t == Trait::Sweeps) {
                        // Every blow it strikes is marked first; between them it
                        // heaves itself up, and that is your opening.
                        if self.floor.is_visible(m.pos) {
                            events.push(Event::Recovering { kind });
                        }
                    } else {
                        self.monster_attack(id, events);
                    }
                } else if self.past_leash(&m) {
                    // The Remnant keep their posts: past the leash they go
                    // back and wait.
                    if m.pos != m.post {
                        self.step_toward(id, m.post);
                    }
                } else if m.pos == last_seen && !sees {
                    // Lost the trail.
                    self.floor.monsters[id].mind = Mind::Unaware;
                } else {
                    self.step_toward(id, last_seen);
                }
            }
            Mind::Fleeing => {
                if !self.step_away(id) && adjacent {
                    // Cornered.
                    self.monster_attack(id, events);
                }
            }
            Mind::Unaware => {}
        }
    }

    /// A compelled creature's action: fight the nearest creature it can see,
    /// or keep close to you.
    fn thrall_act(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        let me = &self.floor.monsters[id];
        let (pos, sight) = (me.pos, self.content.monster(me.kind).sight);
        let foe = self
            .floor
            .monsters
            .iter()
            .filter(|&(other, o)| {
                other != id
                    && !o.phantom
                    && o.compelled == 0
                    && o.pos.chebyshev(pos) <= sight
                    && self.clear_line(pos, o.pos)
            })
            .min_by_key(|(_, o)| o.pos.distance_squared(pos))
            .map(|(other, o)| (other, o.pos));
        if let Some((foe, at)) = foe {
            if at.chebyshev(pos) == 1 {
                self.monster_strike(id, foe, events);
            } else {
                self.step_toward(id, at);
            }
        } else if pos.chebyshev(self.player.pos) > HEEL {
            self.step_toward(id, self.player.pos);
        }
    }

    /// The nearest creature `id` hates that it can see within `range`, and how far.
    fn nearest_enemy(&self, id: MonsterId, range: i32) -> Option<(MonsterId, i32)> {
        let pos = self.floor.monsters[id].pos;
        self.floor
            .monsters
            .iter()
            .filter(|&(other, o)| {
                o.pos.chebyshev(pos) <= range
                    && self.hostile(id, other)
                    && self.clear_line(pos, o.pos)
            })
            .map(|(other, o)| (other, o.pos.chebyshev(pos)))
            .min_by_key(|&(_, d)| d)
    }

    /// Nothing solid between two points.
    fn clear_line(&self, a: Point, b: Point) -> bool {
        let line = crate::inventory::line(a, b);
        line[1..line.len().saturating_sub(1)]
            .iter()
            .all(|&p| !self.map().blocks_sight(p))
    }

    /// One creature strikes another. The struck one turns on its attacker.
    pub(crate) fn monster_strike(
        &mut self,
        attacker: MonsterId,
        defender: MonsterId,
        events: &mut Vec<Event>,
    ) {
        let a = &self.floor.monsters[attacker];
        let d = &self.floor.monsters[defender];
        let (akind, apos, thrall) = (a.kind, a.pos, a.compelled > 0);
        let (dkind, dpos) = (d.kind, d.pos);
        let adef = self.content.monster(akind);
        let chance = combat::hit_chance(adef.accuracy, self.content.monster(dkind).defense);
        let damage = combat::roll_attack(&mut self.combat_rng, chance, adef.damage);
        if self.floor.is_visible(apos) || self.floor.is_visible(dpos) {
            events.push(Event::Attack {
                attacker: Who::Monster(akind),
                defender: Who::Monster(dkind),
                damage,
            });
        }
        self.floor.monsters[defender].foe = Some(attacker);
        if let Some(damage) = damage {
            let source = if thrall {
                Source::Thrall
            } else {
                Source::Other
            };
            self.damage_monster(defender, damage, source, events);
        }
    }

    pub(crate) fn monster_attack(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        let (kind, at) = (self.floor.monsters[id].kind, self.floor.monsters[id].pos);
        let def = self.content.monster(kind);
        let (accuracy, damage) = self.monster_strength(&self.floor.monsters[id]);
        let chance = combat::hit_chance(accuracy, self.guarded_defense());
        let damage = combat::roll_attack(&mut self.combat_rng, chance, damage);
        if self.in_the_dark(at) {
            self.shift_dread(crate::dread::DARK_BLOW, events);
        }
        events.push(Event::Attack {
            attacker: Who::Monster(kind),
            defender: Who::Player,
            damage,
        });
        self.fight_noise();
        // Guarding answers every miss; a blade-hand answers some.
        let riposte = || match self.melee_technique(|t| matches!(t, Technique::Riposte { .. })) {
            Some(Technique::Riposte { chance }) if self.wielded_family() == Some(Family::Blade) => {
                chance
            }
            _ => 0,
        };
        let chance = if self.player.guarding { 100 } else { riposte() };
        if damage.is_none() && chance > 0 && self.combat_rng.random_range(0..100) < chance {
            events.push(Event::Riposte { kind });
            self.player_attack(id, 0, events);
            return;
        }
        if let Some(damage) = damage {
            self.hurt_player(damage, Cause::Attack(kind), events);
            if self.death.is_none() {
                self.on_hit_player(id, events);
            }
            if def.has(|t| *t == Trait::EatsLight)
                && self.player.candle.is_lit()
                && !self.player.vigil
            {
                let amount = TALLOW_BITTEN.min(self.player.candle.tallow());
                self.player.candle.eat(amount);
                events.push(Event::CandleEaten { kind, amount });
                self.witness(kind, Trait::EatsLight);
                self.wake_leavings(crate::leavings::Wake::EnteringDarkness, events);
            }
        }
    }

    /// Whether a monster may step onto `p`.
    pub(crate) fn can_enter(&self, id: MonsterId, p: Point) -> bool {
        let m = &self.floor.monsters[id];
        let def = self.content.monster(m.kind);
        let heedless = m.beckoned > 0;
        let tile = self.map().tile(p);
        let near_fire = || Direction::ALL.iter().any(|&d| self.floor.is_burning(p + d));
        self.map().is_walkable(p)
            && p != self.player.pos
            && self.floor.monster_at(p).is_none()
            && (tile != Tile::DoorSealed || def.has(|t| *t == Trait::Gnaws))
            && tile != Tile::RottenFloor
            && !(def.has(|t| *t == Trait::Unholy) && self.floor.is_sanctified(p))
            && self.anomaly_at(p).is_none()
            && (tile != Tile::DeepWater || heedless || def.has(|t| *t == Trait::Swims))
            && (tile != Tile::DoorClosed
                || def.faction.opens_doors()
                || def.has(|t| matches!(t, Trait::Gnaws | Trait::Nightmare)))
            && (heedless || !self.floor.is_burning(p))
            && (heedless || def.faction != Faction::Swarm || !near_fire())
            && !(def.has(|t| *t == Trait::ShunsLight) && self.floor.ambient_light(p).is_lit())
            && !(def.faction == Faction::Dreaming && self.floor.is_sanctified(p))
    }

    /// Steps one tile closer to `target`, choosing randomly between equally good
    /// steps so packs spread around you. Returns whether it moved.
    pub(crate) fn step_toward(&mut self, id: MonsterId, target: Point) -> bool {
        let dist = path::distances(self.map(), target);
        self.step_by(id, |here, there| {
            let (Some(here), Some(there)) = (dist.at(here), dist.at(there)) else {
                return None;
            };
            (there < here).then_some(there as i64)
        })
    }

    /// Steps one tile farther from the player. Returns whether it moved.
    pub(crate) fn step_away(&mut self, id: MonsterId) -> bool {
        let dist = path::distances(self.map(), self.player.pos);
        self.step_by(id, |here, there| {
            let (Some(here), Some(there)) = (dist.at(here), dist.at(there)) else {
                return None;
            };
            (there > here).then_some(-(there as i64))
        })
    }

    /// Moves to the enterable neighbor with the lowest score (`None` = not allowed).
    fn step_by(&mut self, id: MonsterId, score: impl Fn(Point, Point) -> Option<i64>) -> bool {
        if self.floor.monsters[id].pinned > 0 {
            return false;
        }
        let here = self.floor.monsters[id].pos;
        let options: Vec<(i64, Point)> = Direction::ALL
            .iter()
            .map(|&d| here + d)
            .filter(|&p| self.can_enter(id, p))
            .filter_map(|p| score(here, p).map(|s| (s, p)))
            .collect();
        let Some(best) = options.iter().map(|&(s, _)| s).min() else {
            return false;
        };
        let ties: Vec<Point> = options
            .iter()
            .filter(|&&(s, _)| s == best)
            .map(|&(_, p)| p)
            .collect();
        let &to = ties
            .choose(&mut self.ai_rng)
            .expect("ties include the best");
        self.move_monster(id, to);
        true
    }

    /// Moves a monster one step. The Taken flinch on stepping onto holy ground.
    fn move_monster(&mut self, id: MonsterId, to: Point) {
        // A shut door takes an action to open, or several to chew through.
        let tile = self.map().tile(to);
        if matches!(tile, Tile::DoorClosed | Tile::DoorSealed) {
            let def = self.content.monster(self.floor.monsters[id].kind);
            if def.has(|t| *t == Trait::Gnaws) {
                let need = if tile == Tile::DoorSealed {
                    crate::throne::GNAW_SEAL
                } else {
                    crate::throne::GNAW_DOOR
                };
                let m = &mut self.floor.monsters[id];
                m.gnawed += 1;
                if m.gnawed >= need {
                    m.gnawed = 0;
                    self.floor.seals.retain(|&(p, _)| p != to);
                    self.floor.locks.retain(|&(p, _)| p != to);
                    self.floor.set_tile(to, Tile::Floor);
                }
            } else {
                self.floor.set_tile(to, Tile::Door);
            }
            return;
        }
        if self.floor.has_oil(to) && self.slips(to) {
            self.floor.monsters[id].slipping = true;
        }
        let from = self.floor.monsters[id].pos;
        let onto_holy = self.floor.is_sanctified(to) && !self.floor.is_sanctified(from);
        let monster = &mut self.floor.monsters[id];
        monster.pos = to;
        if onto_holy && self.content.monster(monster.kind).faction == Faction::Taken {
            monster.flinching = true;
        }
    }

    fn wander(&mut self, id: MonsterId) {
        let here = self.floor.monsters[id].pos;
        let options: Vec<Point> = Direction::ALL
            .iter()
            .map(|&d| here + d)
            .filter(|&p| self.can_enter(id, p))
            .collect();
        if let Some(&to) = options.choose(&mut self.ai_rng) {
            self.move_monster(id, to);
        }
    }
}
