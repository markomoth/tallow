//! How monsters decide what to do on their action.

use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::combat::{self, PLAYER_DEFENSE};
use crate::content::Trait;
use crate::events::{Cause, Event, Who};
use crate::geom::{Direction, Point};
use crate::map::path;
use crate::monster::{Mind, MonsterId};
use crate::world::World;

/// Packmates closer than this keep a pack creature brave.
const PACK_RADIUS: i32 = 4;
/// Chance an unaware monster shuffles somewhere on its action.
const WANDER_CHANCE: f64 = 0.3;
/// With your candle out, how close something must be to notice you.
const DARK_NOTICE: i32 = 2;
/// Tallow a light-eating bite takes.
const TALLOW_BITTEN: u32 = 15;

impl World {
    pub(crate) fn monster_act(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        let content = self.content;
        let m = self.floor.monsters[id].clone();
        let def = content.monster(m.kind);
        let kind = m.kind;
        let player = self.player.pos;

        if m.phantom {
            // Phantoms come for you and come apart before they touch you.
            if m.pos.chebyshev(player) <= 1 {
                self.floor.monsters.remove(id);
                if self.floor.is_visible(m.pos) {
                    events.push(Event::PhantomFaded {
                        kind,
                        struck: false,
                    });
                }
            } else {
                self.step_toward(id, player);
            }
            return;
        }

        // Your candle (or a brazier) shows you from afar. In the dark, only
        // something close by notices you.
        let conspicuous = self.player.candle.is_lit() || self.floor.ambient_light(player).is_lit();
        let in_range = m.pos.distance_squared(player) <= def.sight * def.sight;
        let close = m.pos.chebyshev(player) <= DARK_NOTICE;
        let sees = def.has(|t| *t == Trait::Relentless)
            || (self.floor.in_sight(m.pos) && in_range && (conspicuous || close));

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
            let damage =
                (player == target).then(|| combat::roll_damage(&mut self.combat_rng, (lo, hi)));
            events.push(Event::HeavyBlow {
                kind,
                target,
                damage,
            });
            if let Some(damage) = damage {
                self.hurt_player(damage, Cause::HeavyBlow(kind), events);
            }
            return;
        }
        let monster = &mut self.floor.monsters[id];
        monster.cooldown = monster.cooldown.saturating_sub(1);

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
                        events.push(Event::WindUp {
                            kind,
                            target: player,
                        });
                        self.witness(kind, *heavy);
                    } else {
                        self.monster_attack(id, events);
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

    fn monster_attack(&mut self, id: MonsterId, events: &mut Vec<Event>) {
        let kind = self.floor.monsters[id].kind;
        let def = self.content.monster(kind);
        let chance = combat::hit_chance(def.accuracy, PLAYER_DEFENSE);
        let damage = combat::roll_attack(&mut self.combat_rng, chance, def.damage);
        events.push(Event::Attack {
            attacker: Who::Monster(kind),
            defender: Who::Player,
            damage,
        });
        if let Some(damage) = damage {
            self.hurt_player(damage, Cause::Attack(kind), events);
            if def.has(|t| *t == Trait::EatsLight) && self.player.candle.is_lit() {
                let amount = TALLOW_BITTEN.min(self.player.candle.tallow());
                self.player.candle.eat(amount);
                events.push(Event::CandleEaten { kind, amount });
                self.witness(kind, Trait::EatsLight);
            }
        }
    }

    /// Whether a monster may step onto `p`.
    fn can_enter(&self, id: MonsterId, p: Point) -> bool {
        let def = self.content.monster(self.floor.monsters[id].kind);
        self.map().is_walkable(p)
            && p != self.player.pos
            && self.floor.monster_at(p).is_none()
            && !(def.has(|t| *t == Trait::ShunsLight) && self.floor.ambient_light(p).is_lit())
    }

    /// Steps one tile closer to `target`, choosing randomly between equally good
    /// steps so packs spread around you. Returns whether it moved.
    fn step_toward(&mut self, id: MonsterId, target: Point) -> bool {
        let dist = path::distances(self.map(), target);
        self.step_by(id, |here, there| {
            let (Some(here), Some(there)) = (dist.at(here), dist.at(there)) else {
                return None;
            };
            (there < here).then_some(there as i64)
        })
    }

    /// Steps one tile farther from the player. Returns whether it moved.
    fn step_away(&mut self, id: MonsterId) -> bool {
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
        self.floor.monsters[id].pos = to;
        true
    }

    fn wander(&mut self, id: MonsterId) {
        let here = self.floor.monsters[id].pos;
        let options: Vec<Point> = Direction::ALL
            .iter()
            .map(|&d| here + d)
            .filter(|&p| self.can_enter(id, p))
            .collect();
        if let Some(&to) = options.choose(&mut self.ai_rng) {
            self.floor.monsters[id].pos = to;
        }
    }
}
