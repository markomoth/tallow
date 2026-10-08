//! The message log, and the narration that turns core events into text.

use std::collections::VecDeque;

use tallow_core::{
    Burden, Content, DreadBand, Event, Faction, ItemClass, KindId, Potency, SideEffect, Tile,
    TinctureEffect, Who, World,
};

use crate::names::{boon_text, item_name, item_phrase, skill_name, technique_text};
use tallow_core::RiteFailure;

const CAPACITY: usize = 200;

/// How a line should feel. The renderer picks its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Normal,
    /// Whispers, phantoms, the mind going.
    Dread,
    /// Act now.
    Danger,
    /// Relief.
    Good,
}

/// One line in the log. Repeats of the same text collapse into a count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
    pub tone: Tone,
    pub count: u32,
}

#[derive(Debug, Default)]
pub struct MessageLog {
    entries: VecDeque<Entry>,
}

impl MessageLog {
    pub fn push(&mut self, text: impl Into<String>, tone: Tone) {
        let text = text.into();
        if let Some(last) = self.entries.back_mut()
            && last.text == text
        {
            last.count += 1;
            return;
        }
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            text,
            tone,
            count: 1,
        });
    }

    /// Entries from oldest to newest.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &Entry> + ExactSizeIterator {
        self.entries.iter()
    }
}

const WHISPERS: [&str; 8] = [
    "Someone says your name, close to your ear.",
    "A bell, very far away. Then not.",
    "Footsteps that stop when yours do.",
    "\"Acolyte,\" says the dark, fondly.",
    "Something breathes on the back of your neck.",
    "You hear the choir, singing the hymn backwards.",
    "Wax drips somewhere. You are not near a candle.",
    "A child laughs inside the wall.",
];

/// The log line for an event, if it deserves one. Routine movement stays quiet.
pub fn narrate(event: &Event, world: &World) -> Option<(String, Tone)> {
    use Tone::{Danger, Dread, Good, Normal};
    let content = world.content();
    let thing = |kind, count| item_name(world, kind, count);
    let name = |kind: KindId| &content.monster(kind).name;
    let (text, tone): (String, Tone) = match *event {
        Event::PlayerBlocked {
            tile: Tile::Wall, ..
        } => ("Cold stone. The wall does not give.".into(), Normal),
        Event::PlayerBlocked { .. } => ("Something blocks the way.".into(), Normal),
        Event::Spotted {
            tile: Tile::StairsDown,
            ..
        } if world.stage() == tallow_core::Stage::Descent => {
            ("A stair leads further down.".into(), Normal)
        }
        Event::Spotted {
            tile: Tile::StairsUp,
            ..
        } if matches!(world.stage(), tallow_core::Stage::Ascent(_)) => {
            ("A stair leads up. The way home.".into(), Good)
        }
        Event::Spotted {
            tile: Tile::Altar, ..
        } => ("The altar. Bare, where the candle should be.".into(), Good),
        Event::Descended { depth } => (descent_line(depth).into(), Normal),
        Event::NoStairsHere => ("There are no stairs here.".into(), Normal),
        Event::StairsSealed { depth: tallow_core::MAX_DEPTH } => (
            "The stair up is shut by a weight of flies. Not without the candle.".into(),
            Normal,
        ),
        Event::StairsSealed { depth: 1 } => (
            "Above is only the church, and the dreamers. Not yet.".into(),
            Normal,
        ),
        Event::StairsSealed { .. } => (
            "Rubble chokes the stair behind you. The way is down.".into(),
            Normal,
        ),
        Event::RunRefused => ("Not with something watching.".into(), Normal),
        Event::NothingToRest => ("You are as rested as you will get here.".into(), Normal),
        Event::Rested { turns } => (format!("You rest for {turns} turns."), Normal),

        Event::SpottedTallow { .. } => ("Tallow glints in the light.".into(), Normal),
        Event::TallowFound { amount } => (
            format!("You gather up tallow for your candle (+{amount})."),
            Good,
        ),
        Event::CandleLow => ("Your candle burns low. Find tallow.".into(), Danger),
        Event::CandleGuttering => ("Your candle gutters. The light draws in.".into(), Danger),
        Event::CandleBurnedOut => ("Your candle goes out. The dark comes close.".into(), Danger),
        Event::CandleSnuffed => ("You pinch the wick. Darkness.".into(), Normal),
        Event::CandleLit => ("You light the candle again.".into(), Normal),
        Event::CandleSpent => ("There is no tallow left to light.".into(), Danger),
        Event::CandleEaten { kind, amount } => (
            format!(
                "The {} closes its mouth on your flame. The candle goes out (−{amount} tallow).",
                name(kind)
            ),
            Danger,
        ),

        Event::DreadChanged { band } => (
            match band {
                DreadBand::Calm => "Your breathing slows.",
                DreadBand::Uneasy => "Unease settles on you. The dark has started to listen.",
                DreadBand::Frayed => {
                    "Your hands won't stop shaking. Not everything you see now is there."
                }
                DreadBand::Manifest => "Your dread is complete.",
            }
            .into(),
            Dread,
        ),
        Event::Whisper { seed } => (WHISPERS[seed as usize % WHISPERS.len()].into(), Dread),
        Event::PhantomFaded { struck: true, .. } => (
            "Your blow passes through nothing. It was never there.".into(),
            Dread,
        ),
        Event::PhantomFaded {
            kind,
            struck: false,
        } => (
            format!(
                "The {} comes apart before it reaches you. It was never there.",
                name(kind)
            ),
            Dread,
        ),
        Event::Manifested { grown } => (
            if grown == 0 {
                "Your dread takes a shape. Something wearing your face is coming.".into()
            } else {
                "Your dread takes a shape again. It has learned more of you: it is stronger than last time.".into()
            },
            Danger,
        ),
        Event::NightmareComing => (
            "Your fear has been heard. Something is coming for you. Calm yourself (spend your dread) and it may not.".into(),
            Danger,
        ),
        Event::NightmareArrives { kind } => (
            format!("A {} has come for you. It knows where you are.", name(kind)),
            Danger,
        ),
        Event::NightmareTurnedAway => (
            "Your breathing slows. Whatever was coming turns away.".into(),
            Good,
        ),
        Event::NightmareFades { kind } => (
            format!("You are calm. The {} comes apart like smoke.", name(kind)),
            Good,
        ),
        Event::PhantomBit { kind, damage } => (
            match damage {
                Some(damage) => format!(
                    "The {} reaches you and comes apart, but its teeth were real enough ({damage}).",
                    name(kind)
                ),
                None => format!(
                    "The {} reaches you and comes apart. Your heart hammers (+5 dread).",
                    name(kind)
                ),
            },
            Dread,
        ),
        Event::Resisted { kind } => (
            format!(
                "Your blow barely bites: the {} shrugs off half of it. {}",
                name(kind),
                match content.monster(kind).faction {
                    tallow_core::content::Faction::Swarm => "Blunt things are wasted on a swarm.",
                    tallow_core::content::Faction::Remnant => "Blades only nick paper and brass.",
                    _ => "In the dark it is barely there. Light holds it.",
                }
            ),
            Normal,
        ),
        Event::Bleeding { kind } => (format!("The {} bleeds.", name(kind)), Normal),
        Event::Kindled { kind } => (
            format!("Sparks fly from the censer. The ground under the {} catches!", name(kind)),
            Danger,
        ),
        Event::Hooked { kind } => (format!("You haul the {} in.", name(kind)), Normal),
        Event::Shoved { kind } => (format!("The {} is driven back.", name(kind)), Normal),
        Event::FedOnFear { kind, dread } => (
            format!("The {}'s touch is cold. Your fear grows (+{dread} dread).", name(kind)),
            Dread,
        ),
        Event::CandleSnuffedBy { kind } => (
            format!("The {} comes close, and your candle goes out.", name(kind)),
            Danger,
        ),
        Event::Doubled { kind } => (
            format!("There are two of the {} now. One of them isn't there.", name(kind)),
            Dread,
        ),
        Event::ManifestationBanished => (
            "The thing with your face is gone. You can breathe.".into(),
            Good,
        ),
        Event::ManifestationEscaped => (
            "You leave your dread on the floor above. For now.".into(),
            Good,
        ),

        Event::SpottedItem { kind, .. } => {
            (format!("You see {}.", item_phrase(world, kind, 1)), Normal)
        }
        Event::NothingToPickUp => ("There is nothing here to pick up.".into(), Normal),
        Event::PickedUp { kind, count } => (
            format!("You pick up {}.", item_phrase(world, kind, count)),
            Normal,
        ),
        Event::PackFull { kind } => (
            format!(
                "Your pack has no room for another kind of thing. The {} stays.",
                thing(kind, 1)
            ),
            Danger,
        ),
        Event::Dropped { kind, count } => (
            format!("You drop {}.", item_phrase(world, kind, count)),
            Normal,
        ),
        Event::Equipped { kind } => {
            let verb = match content.item(kind).class {
                ItemClass::Vestment { .. } => "put on",
                ItemClass::Ranged { .. } => "ready",
                _ => "take up",
            };
            (format!("You {verb} the {}.", thing(kind, 1)), Normal)
        }
        Event::Unequipped { kind } => {
            let verb = match content.item(kind).class {
                ItemClass::Vestment { .. } => "take off",
                _ => "put away",
            };
            (format!("You {verb} the {}.", thing(kind, 1)), Normal)
        }
        Event::CantUse { kind } => (
            format!("The {} has no use of its own.", thing(kind, 1)),
            Normal,
        ),
        Event::Drank {
            effect,
            potency,
            side,
            amount,
            learned,
            ..
        } => {
            let what = match effect {
                TinctureEffect::Mending => format!("Your wounds close (+{amount})."),
                TinctureEffect::Steadying => format!("Your dread recedes (−{amount})."),
                TinctureEffect::Seeing => "The shape of the floor comes to you.".into(),
            };
            let opening = match (learned, potency) {
                (false, _) => "",
                (true, Potency::Weak) => "Weak stuff. ",
                (true, Potency::Common) => "Middling strength. ",
                (true, Potency::Strong) => "Strong stuff. ",
            };
            let aftertaste = match side {
                SideEffect::None => "",
                SideEffect::Bitter => " It leaves a bitter taste of the dark.",
            };
            (format!("{opening}{what}{aftertaste}"), Good)
        }
        Event::CantThrow { kind } => (
            format!("The {} isn't made for throwing.", thing(kind, 1)),
            Normal,
        ),
        Event::BadTarget => (
            "Nothing can fly that way. Choose another target.".into(),
            Normal,
        ),
        Event::Thrown { .. } | Event::Fired { .. } => return None,
        Event::NoRangedWeapon => (
            "You have nothing ready to shoot. Equip a sling or crossbow.".into(),
            Normal,
        ),
        Event::NoAmmo { kind } => (
            format!("You have no {}.", content.item(kind).plural),
            Normal,
        ),
        Event::ProjectileHit {
            item,
            target,
            damage,
        } => {
            let (item, target) = (thing(item, 1), name(target));
            match damage {
                Some(0) => (
                    format!("The {item} splashes the {target}, harmlessly."),
                    Normal,
                ),
                Some(d) => (format!("The {item} hits the {target} ({d})."), Normal),
                None => (format!("The {item} misses the {target}."), Normal),
            }
        }
        Event::Shattered { kind, .. } => (format!("The {} shatters.", thing(kind, 1)), Normal),
        Event::BurdenChanged { burden } => match burden {
            Burden::Light => ("Your load feels manageable again.".into(), Good),
            Burden::Burdened => ("You are burdened. Every step takes longer.".into(), Danger),
            Burden::Overloaded => ("You carry too much to move. Drop something.".into(), Danger),
        },
        Event::SkillRankUp { skill, rank } => (
            format!(
                "Your skill with {} grows: rank {rank}.",
                skill_name(skill).to_lowercase()
            ),
            Good,
        ),
        Event::TechniqueLearned { skill, technique } => {
            let (name, what) = technique_text(skill, technique);
            (format!("You learn {name}. {what}"), Good)
        }
        Event::LevelUp { level } => (
            format!("You have grown. Level {level}: choose a boon."),
            Good,
        ),
        Event::BoonTaken { boon } => {
            let (title, what) = boon_text(boon);
            (format!("{title}: {what}"), Good)
        }
        Event::Riposte { kind } => (
            format!("The {} misses, and you strike back!", name(kind)),
            Normal,
        ),
        Event::Staggered { kind } => (format!("The {} staggers.", name(kind)), Normal),
        Event::Pinned { kind } => (format!("The {} is pinned in place.", name(kind)), Normal),
        Event::TooHeavy => (
            "You can't move under this weight. Open your pack (i) and drop something.".into(),
            Danger,
        ),

        Event::FirstSighting { kind } => {
            let def = content.monster(kind);
            (
                format!(
                    "{}. {}",
                    capitalize(&with_article(&def.name)),
                    def.description
                ),
                Normal,
            )
        }
        Event::Noticed { kind, seen: true } => (format!("The {} notices you.", name(kind)), Normal),
        Event::Noticed { seen: false, .. } => (
            "Something in the dark has noticed your light.".into(),
            Danger,
        ),
        Event::Bark { kind, line } => {
            let def = content.monster(kind);
            (
                format!("The {} whispers: {}", def.name, def.barks.get(line)?),
                Normal,
            )
        }
        Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(kind),
            damage: Some(d),
        } => (format!("You strike the {} ({d}).", name(kind)), Normal),
        Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(kind),
            damage: None,
        } => (format!("You miss the {}.", name(kind)), Normal),
        Event::Attack {
            attacker: Who::Monster(a),
            defender: Who::Monster(d),
            damage,
        } => match damage {
            Some(n) => (
                format!("The {} strikes the {} ({n}).", name(a), name(d)),
                Normal,
            ),
            None => (format!("The {} misses the {}.", name(a), name(d)), Normal),
        },
        Event::Attack {
            attacker: Who::Monster(kind),
            damage: Some(d),
            ..
        } => (format!("The {} hits you ({d}).", name(kind)), Normal),
        Event::Attack {
            attacker: Who::Monster(kind),
            damage: None,
            ..
        } => (format!("The {} misses you.", name(kind)), Normal),
        Event::Attack {
            attacker: Who::Player,
            defender: Who::Player,
            ..
        } => return None,
        Event::WindUp { kind, .. } => (
            format!("The {} raises something heavy over you. Move!", name(kind)),
            Danger,
        ),
        Event::HeavyBlow {
            damage: Some(d), ..
        } => (format!("The heavy blow lands ({d})."), Danger),
        Event::HeavyBlow {
            kind, damage: None, ..
        } => (
            format!("The {}'s blow smashes empty floor.", name(kind)),
            Normal,
        ),
        Event::Fled { kind } => (
            format!("The {} squeals and breaks away.", name(kind)),
            Normal,
        ),
        Event::MonsterDied { kind, .. } => (death_line(content, kind), Normal),
        Event::PlayerDied { .. } => ("You die.".into(), Danger),

        Event::NoCorpse => ("There is no body here.".into(), Normal),
        Event::NothingToLearn { kind } => (
            format!("You have learned all a {} can teach you.", name(kind)),
            Normal,
        ),
        Event::Studied { kind, first: true } => (
            format!(
                "You finish your study of the {}. You know its ways now (Look shows them all). The body is spoiled for rendering.",
                name(kind)
            ),
            Good,
        ),
        Event::Studied { kind, first: false } => {
            (format!("You study the {} again.", name(kind)), Good)
        }
        Event::WorkInterrupted { .. } => (
            "You stop what you're doing. Your work will keep.".into(),
            Danger,
        ),
        Event::RenderSmell => (
            "The smell of rendering fat drifts through the halls. Something will come to it.".into(),
            Danger,
        ),
        Event::WaxLeft { kind, amount } => (
            format!(
                "Where the {} came apart, a little grave-wax is left ({amount} tallow).",
                name(kind)
            ),
            Normal,
        ),
        Event::Rendered { kind, tallow } => (
            format!(
                "You render the {} down. Grim work, but it will burn (+{tallow} tallow).",
                name(kind)
            ),
            Good,
        ),
        Event::CorpseSwelling { kind, .. } => (
            format!(
                "The {}'s body has begun to swell. Something moves inside it.",
                name(kind)
            ),
            Danger,
        ),
        Event::CorpseHatched { kind, .. } => (
            format!("The {}'s body splits, and flies pour out!", name(kind)),
            Danger,
        ),
        Event::Read { kind, learned } => (
            if learned {
                format!("You read the {} closely, twice.", thing(kind, 1))
            } else {
                format!(
                    "You read the {}. It teaches nothing you don't know, but it fills a gap.",
                    thing(kind, 1)
                )
            },
            Normal,
        ),
        Event::ReadingInterrupted { .. } => ("You look up from the page. Not now.".into(), Danger),
        Event::RiteLearned { rite } => {
            let def = content.rite(rite);
            (
                format!(
                    "You learn the rite of {} ({}). Cast it with z.",
                    def.name,
                    crate::names::school_name(def.school)
                ),
                Good,
            )
        }
        Event::RiteFailed { why, .. } => (
            match why {
                RiteFailure::Unknown => "You don't know that rite.",
                RiteFailure::NoTarget => "The rite needs a creature you can see.",
                RiteFailure::OutOfRange => "Too far. The words won't carry.",
                RiteFailure::NotOpenGround => "It needs open ground you can see.",
                RiteFailure::NotTaken => {
                    "There is nothing in it to cast out. Only the Taken can be freed."
                }
                RiteFailure::Immune => "Your own dread will not obey you.",
                RiteFailure::AlreadyCarries => {
                    "It already carries your dread. It can hold no more."
                }
                RiteFailure::NotDoor => "It needs a door you know, with nothing in the doorway.",
                RiteFailure::NotDreaming => "Only the Dreaming can be sent back into the dark.",
                RiteFailure::NotEnoughDread => {
                    "You haven't the dread to pay for it. Rites are bought with fear: go into the dark."
                }
                RiteFailure::Recharging => {
                    "The words are still hot in your mouth. That rite isn't ready again yet."
                }
            }
            .into(),
            Normal,
        ),
        Event::Cast { rite } => (
            format!("You speak the rite of {}.", content.rite(rite).name),
            Dread,
        ),
        Event::Compelled { kind } => (
            format!(
                "The {} goes still, then turns to stand with you.",
                name(kind)
            ),
            Good,
        ),
        Event::CompelEnded { kind } => (
            format!("The {} shakes off your hold. It remembers.", name(kind)),
            Danger,
        ),
        Event::Knelt { kind } => (format!("The {}'s legs fold under it.", name(kind)), Good),
        Event::Leeched { kind, amount } => (
            format!("You draw the warmth out of the {} (+{amount}).", name(kind)),
            Good,
        ),
        Event::Transferred { kind, amount } => (
            format!(
                "Your dread pours into the {} (−{amount}). It runs from what it feels.",
                name(kind)
            ),
            Good,
        ),
        Event::EyesBorrowed { kind } => (
            format!(
                "You see through the {}'s eyes as well as your own.",
                name(kind)
            ),
            Good,
        ),
        Event::EyesReturned => ("Your sight is your own again.".into(), Normal),
        Event::Unseen { kind } => (
            format!("The {} looks through you, puzzled.", name(kind)),
            Good,
        ),
        Event::FalseFlameLit { .. } => ("A pale flame burns where nothing burns.".into(), Good),
        Event::FalseFlameOut => ("The false flame goes out.".into(), Normal),
        Event::FlameEaten { kind } => (
            format!("The {} swallows the false flame.", name(kind)),
            Normal,
        ),
        Event::Shrouded => (
            "Your candle burns on, but nothing notices it now.".into(),
            Good,
        ),
        Event::ShroudFaded => ("Your shroud thins. Your light shows again.".into(), Danger),
        Event::Sanctified => ("The ground around you remembers it was holy.".into(), Good),
        Event::SanctityFaded => ("The holy ground forgets itself.".into(), Normal),
        Event::Exorcised { kind, .. } => (
            format!(
                "The dream goes out of the {}. They wake, stare at you, and run for home.",
                name(kind)
            ),
            Good,
        ),
        Event::ExorciseResisted { kind, damage } => (
            format!(
                "The {} is too deep in the dream. It burns instead ({damage}).",
                name(kind)
            ),
            Normal,
        ),
        Event::Flinched { kind } => (
            format!("The {} flinches from the holy ground.", name(kind)),
            Normal,
        ),
        Event::SwappedPlaces { .. } => return None,

        Event::BellRung { .. } => (
            "A bell rings out through the dark. Things will come to see.".into(),
            Danger,
        ),
        Event::FireSpread { .. } => (
            "The shelves catch. The books go up like dry grass.".into(),
            Danger,
        ),
        Event::FireOut => ("The last of the fire dies to embers.".into(), Normal),
        Event::Burned {
            who: Who::Player,
            damage,
        } => (format!("The fire burns you ({damage})!"), Danger),
        Event::Burned {
            who: Who::Monster(kind),
            damage,
        } => (format!("The {} burns ({damage}).", name(kind)), Normal),
        Event::FireAhead { .. } => (
            "That is fire. Step that way again to walk into it.".into(),
            Danger,
        ),
        Event::Ignited { .. } => ("The flask bursts into flame!".into(), Danger),
        Event::OilSpilled { .. } => ("Oil spreads across the floor.".into(), Normal),
        Event::Slipped { who: Who::Player } => {
            ("You slip on the oil and lose your footing.".into(), Normal)
        }
        Event::Slipped {
            who: Who::Monster(kind),
        } => (format!("The {} slips on the oil.", name(kind)), Normal),
        Event::DoorOpened { .. } => ("You open the door.".into(), Normal),
        Event::DoorShut { .. } => ("You shut the door.".into(), Normal),
        Event::NoDoorToClose => ("There is no open door beside you.".into(), Normal),
        Event::DoorBlocked => ("Something is in the doorway.".into(), Normal),
        Event::SealBroken { .. } => ("You open the door, and your seal breaks.".into(), Normal),
        Event::SealFaded { .. } => (
            "A seal lapses. The door is only a door again.".into(),
            Normal,
        ),
        Event::NeedFlame => ("You need a lit candle to light it.".into(), Normal),
        Event::Offered { dread, health } => (
            format!(
                "You give the brazier your fear. It takes {dread} dread and mends {health} health."
            ),
            Good,
        ),
        Event::NothingToOffer => (
            "The brazier wants dread, and you have too little to give. Gather some in the dark."
                .into(),
            Normal,
        ),
        Event::WholeAlready => (
            "You are whole. The brazier would take your dread for nothing.".into(),
            Normal,
        ),
        Event::LightDrawn => (
            "Somewhere on this floor, something has seen your light, and is coming to it.".into(),
            Danger,
        ),
        Event::WritingSpotted { .. } => (
            "In the dark, letters glow faintly on a wall. Walk into them to trace them, before you relight.".into(),
            Good,
        ),
        Event::WritingWashedOut => (
            "Your candlelight washes the letters out. They can only be read in the dark.".into(),
            Normal,
        ),
        Event::WritingRead { learned } => (
            if learned {
                "You trace the glowing letters with your fingers. They are words of a rite.".into()
            } else {
                "You trace the glowing letters. A rite you already know, said another way.".into()
            },
            Good,
        ),
        Event::BrazierLit { .. } => (
            "You touch your flame to the brazier. It roars up, warm and steady.".into(),
            Good,
        ),
        Event::Consecrated { .. } => (
            "Where the holy water fell, the ground is holy.".into(),
            Good,
        ),
        Event::Turned { kind } => (
            format!("The {} looks at its own kind with new hatred.", name(kind)),
            Good,
        ),
        Event::Beckoned { kind } => (
            format!("The {} comes toward you, calm and blank.", name(kind)),
            Good,
        ),
        Event::Exchanged { kind } => (format!("You and the {} trade places.", name(kind)), Good),
        Event::Hushed => ("The sounds you make fall dead around you.".into(), Good),
        Event::HushFaded => (
            "Your hush lifts. You can hear yourself again.".into(),
            Normal,
        ),
        Event::Sealed { .. } => ("The door swings shut and holds.".into(), Good),
        Event::Banished { kind } => (
            format!(
                "The {} is gone, back into the dark somewhere far from here.",
                name(kind)
            ),
            Good,
        ),
        Event::DeepWaterAhead { .. } => (
            "Deep water. Your candle will go out in it. Step that way again to wade in.".into(),
            Danger,
        ),
        Event::CandleDrowned => ("The water takes your flame. Darkness.".into(), Danger),
        Event::CantLightInWater => ("Not while you stand in deep water.".into(), Normal),
        Event::RottenAhead { .. } => (
            "The boards ahead are rotten. They will give way. Step that way again to risk it."
                .into(),
            Danger,
        ),
        Event::Fell { damage, .. } => (
            format!("The boards give way! You fall, and land hard on the floor below ({damage})."),
            Danger,
        ),
        Event::Blinded { kind } => (
            format!(
                "The {} splashes ink in your eyes. Your light shrinks.",
                name(kind)
            ),
            Danger,
        ),
        Event::SightReturns => (
            "You blink the ink away. Your light reaches out again.".into(),
            Good,
        ),
        Event::ChantBegins { kind } => (
            format!(
                "The {} begins to chant at you. Get out of its sight!",
                name(kind)
            ),
            Danger,
        ),
        Event::ChantLands { kind, damage } => (
            format!(
                "The {}'s rite takes hold of you ({damage}). Your dread deepens.",
                name(kind)
            ),
            Danger,
        ),
        Event::ChantBroken { kind } => (
            format!(
                "The {}'s chant falls apart without you in its sight.",
                name(kind)
            ),
            Good,
        ),
        Event::Dragged { kind } => (
            format!("The {} drags you toward the deep water!", name(kind)),
            Danger,
        ),
        Event::Singing { kind } => (
            format!("The {} sings. It gets under your skin.", name(kind)),
            Dread,
        ),
        Event::Burst { kind, .. } => (
            format!("The {} bursts, and flies pour out!", name(kind)),
            Danger,
        ),
        Event::Summoned { kind, .. } => (
            format!("Flies gather out of the dark around the {}.", name(kind)),
            Danger,
        ),
        Event::Raised { kind, body, into } => (
            format!(
                "The {} reaches into the {}'s body. It rises as {}.",
                name(kind),
                name(body),
                with_article(name(into))
            ),
            Danger,
        ),
        Event::DoorsLocked { kind } => (
            format!(
                "The {} raises a hand. Every door around slams and locks.",
                name(kind)
            ),
            Danger,
        ),
        Event::BooksIgnited { kind } => (
            format!(
                "The {} speaks a word, and the shelves burst into flame!",
                name(kind)
            ),
            Danger,
        ),
        Event::DoorLocked => (
            "The door is locked fast. It will give in time.".into(),
            Normal,
        ),
        Event::BossDefeated { kind } => (
            format!("The {} is finished. The floor goes very quiet.", name(kind)),
            Good,
        ),
        Event::SeepSpotted => (
            "The air in a room ahead ripples like heat over a road. Something is wrong in there. Throw something through it before you walk in.".into(),
            Danger,
        ),
        Event::SpottedLeaving { id } => (
            format!("Something lies there that should not be: {}.", crate::names::leaving_name(world, id)),
            Normal,
        ),
        Event::LeavingTaken { id, first: true } => (
            format!(
                "You take {}. This is a Leaving: it will wake, it will do something, and it will take something. Look at it in your pack (i, then its number). Dropping it stops it.",
                crate::names::leaving_name(world, id)
            ),
            Danger,
        ),
        Event::LeavingTaken { id, first: false } => (
            format!("You take {}.", crate::names::leaving_name(world, id)),
            Normal,
        ),
        Event::LeavingDropped { id } => (
            format!("You put down {}.", crate::names::leaving_name(world, id)),
            Normal,
        ),
        Event::LeavingWontWake { id } => (
            format!("{} does not answer when you will it.", capitalize(&crate::names::leaving_name(world, id))),
            Normal,
        ),
        Event::LeavingResting { .. } => ("It is quiet. It will answer again soon.".into(), Normal),
        Event::LeavingThreatens { id } => (
            format!(
                "{} grows heavy and cold. Its price would kill you. Drop it now, or it takes it next turn!",
                capitalize(&crate::names::leaving_name(world, id))
            ),
            Danger,
        ),
        Event::LeavingWoke { id, learned } => {
            let name = capitalize(&crate::names::leaving_name(world, id));
            if learned {
                (format!("{name} wakes. Now you know it: {}", crate::names::leaving_rule(world, id)), Dread)
            } else {
                (format!("{name} wakes."), Dread)
            }
        }
        Event::Duplicated { kind } => (format!("There is one more {} than there was.", thing(kind, 1)), Good),
        Event::DarkSightFaded => ("The dark closes over your eyes again.".into(), Normal),
        Event::AnomalyRevealed { kind, .. } => (
            format!("It stops in mid-air. There is {} there.", crate::names::anomaly_text(kind).0),
            Good,
        ),
        Event::AnomalyStruck { kind, damage } => (
            match kind {
                tallow_core::leavings::AnomalyKind::Heat => format!("The air itself burns you ({damage})!"),
                tallow_core::leavings::AnomalyKind::Snare => format!("Something seizes you and bears down ({damage}). You can't move!"),
                tallow_core::leavings::AnomalyKind::Pocket => "Everything goes still. When it lets go, time has passed without you.".into(),
                tallow_core::leavings::AnomalyKind::Swap => "The room turns inside out, and you are somewhere else.".into(),
            },
            Danger,
        ),
        Event::AnomalyAhead { .. } => (
            "An anomaly is there. Step that way again to walk into it.".into(),
            Danger,
        ),
        Event::RiteOffered { rite } => (
            format!(
                "The rite of {} is there for the taking, but your mind holds all the rites it can. Forget one to make room, or let it go.",
                content.rite(rite).name
            ),
            Dread,
        ),
        Event::RiteForgotten { rite } => (
            format!("The rite of {} slips out of your memory.", content.rite(rite).name),
            Normal,
        ),
        Event::RiteLetGo { rite } => (
            format!("You let the rite of {} go. Something of it stays with you.", content.rite(rite).name),
            Normal,
        ),
        Event::RiteSlotGained { slots } => (
            format!("Your mind has room for another rite ({slots} now)."),
            Good,
        ),
        Event::BodySpoiled { kind } => (
            format!("You opened the {} up to study it. The fat is spoiled; it won't render.", name(kind)),
            Normal,
        ),
        Event::LeavingsFull { id } => (
            format!(
                "You can't carry {} as well. You carry all the Leavings you can bear; drop one first.",
                crate::names::leaving_name(world, id)
            ),
            Normal,
        ),
        Event::PushThroughAhead { .. } => (
            "It fills the way. Step into it again to force your way through: it will tear at you (6–9) and the fear will stay (+15 dread).".into(),
            Danger,
        ),
        Event::PushedThrough { kind, damage } => (
            format!("You shove into the {} and through it. It tears at you as you pass ({damage}).", name(kind)),
            Danger,
        ),
        Event::Flared { kind } => (
            format!("The Vigil Candle flares white. The {} is driven back, reeling.", name(kind)),
            Good,
        ),
        Event::FlareSpent => (
            "The candle has flared once on this floor. It will not again until the next.".into(),
            Normal,
        ),
        Event::NothingToFlare => (
            "Nothing is near enough for the flare to drive back.".into(),
            Normal,
        ),
        Event::Undying { kind } => (
            format!("Your blow passes through the {}. It cannot die. Run.", name(kind)),
            Danger,
        ),
        Event::SwarmAbsorbs { left } => (
            format!("The flies take the blow for him ({left} of his swarm left). Light thins them; fire burns them."),
            Normal,
        ),
        Event::SwarmThins { left } => (
            format!("His swarm boils off in the light ({left} left)."),
            Good,
        ),
        Event::LordLeavesBody => (
            "The body falls empty. Something leaves it, buzzing, looking for another.".into(),
            Danger,
        ),
        Event::LordGathers { .. } => (
            "Flies gather thick over one of the bodies on the floor. He will rise in it next turn.".into(),
            Danger,
        ),
        Event::LordPossesses { .. } => (
            "The body stands up, wearing him. Bind it, exorcise it, or leave him no bodies.".into(),
            Danger,
        ),
        Event::LordBound => (
            "Your rite closes on him like a fist. He is trapped in this body now.".into(),
            Good,
        ),
        Event::LordRises => (
            "Nowhere left to hide. Beelzebub rises as himself: the Lord of Flies. Watch the marked ground.".into(),
            Danger,
        ),
        Event::LordFalls => (
            "The Lord of Flies comes apart into a thousand dying flies, and then into nothing. The candle is unguarded.".into(),
            Good,
        ),
        Event::CandleGuarded => (
            "The flies around the candle will not let you near it while he lives.".into(),
            Normal,
        ),
        Event::VigilTaken => (
            "You lift the Vigil Candle. Its flame steadies, and your own stops mattering. Somewhere far above, a bell. Take it home. Something stirs behind you.".into(),
            Good,
        ),
        Event::Ascended { floor } => (
            match floor {
                1 => "You climb. The stairs are not where they were. The labyrinth is coming apart behind you.".into(),
                4 => "The last of the way up. You smell candle smoke from the church.".into(),
                _ => "Up again. The walls lean in the wrong directions.".into(),
            },
            Normal,
        ),
        Event::ReachedChurch => (
            "You climb up through the floorboards into the church. It is night, and the braziers burn low. The dreamers lie in their pews, not breathing deeply enough. The altar waits at the far end.".into(),
            Good,
        ),
        Event::FollowingNear => (
            "Below you, on the stair, a sound like a thousand wings folding. The Following is coming.".into(),
            Danger,
        ),
        Event::FollowingArrives => (
            "The Following pours up the stair behind you. It cannot be killed. Shut doors; keep to holy ground and brazier light; go.".into(),
            Danger,
        ),
        Event::AltarEmpty => ("You have nothing to set on the altar.".into(), Normal),
        Event::Recovering { kind } => (
            format!("The {} heaves itself up, open. Strike now.", name(kind)),
            Normal,
        ),
        Event::TallowSpilled { amount } => (
            format!("You can't carry any more; {amount} tallow spills at your feet."),
            Normal,
        ),
        Event::Explored => ("There is nothing more here you can reach safely.".into(), Normal),
        Event::NoGoingBack => ("Down is behind you now. The way is up.".into(), Normal),
        Event::Won => (
            "You set the Vigil Candle on the altar. Its light fills the church.".into(),
            Good,
        ),
        Event::Spotted { .. } | Event::PlayerMoved { .. } | Event::PlayerWaited => return None,
    };
    Some((text, tone))
}

fn death_line(content: &Content, kind: KindId) -> String {
    let def = content.monster(kind);
    match def.faction {
        Faction::Taken => format!("The {} falls, and does not get up.", def.name),
        Faction::Dreaming => format!("The {} comes apart like smoke.", def.name),
        Faction::Swarm | Faction::Remnant => format!("The {} dies.", def.name),
    }
}

/// What lies where you stand, one line per thing: its name and what it is.
pub fn underfoot(world: &World) -> Vec<String> {
    let here = world.player().pos;
    let floor = world.floor();
    // Only the name: what a thing is for is learned by looking at it.
    let items = floor.items_at(here).map(|f| {
        format!(
            "Underfoot: {} (g to pick up).",
            item_phrase(world, f.item.kind, f.item.count)
        )
    });
    let leavings = floor
        .leavings()
        .iter()
        .filter(|&&(at, _)| at == here)
        .map(|&(_, id)| {
            format!(
                "Underfoot: {} (g to pick up).",
                crate::names::leaving_name(world, id)
            )
        });
    items.chain(leavings).collect()
}

/// "a gnawer", "an eel".
pub fn with_article(name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "aeiouAEIOU".contains(c));
    format!("{} {name}", if vowel { "an" } else { "a" })
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn descent_line(depth: u8) -> &'static str {
    const ROUTINE: [&str; 4] = [
        "You descend. The steps are slick with old wax.",
        "Down again. Your candle leans toward something below.",
        "The stair turns more times than it should.",
        "You go down. Far above, very faintly, the bell.",
    ];
    // The first floor of each biome gets its own line.
    match depth {
        4 => "The crypt gives way to carved halls and empty lecterns. The Collegium.",
        7 => "Water on the steps, then at your ankles. The lower stacks are drowned.",
        10 => "The walls are warm here, and they hum. Flies.",
        tallow_core::MAX_DEPTH => "The stair ends. Whatever lives under the church is here.",
        _ => ROUTINE[usize::from(depth) % ROUTINE.len()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_collapse_into_a_count() {
        let mut log = MessageLog::default();
        log.push("a", Tone::Normal);
        log.push("a", Tone::Normal);
        log.push("b", Tone::Normal);
        let entries: Vec<_> = log.entries().cloned().collect();
        assert_eq!(
            entries,
            vec![
                Entry {
                    text: "a".into(),
                    tone: Tone::Normal,
                    count: 2
                },
                Entry {
                    text: "b".into(),
                    tone: Tone::Normal,
                    count: 1
                }
            ]
        );
    }

    #[test]
    fn every_depth_has_a_descent_line() {
        for depth in 2..=tallow_core::MAX_DEPTH {
            assert!(narrate(&Event::Descended { depth }, &World::new(1)).is_some());
        }
    }

    #[test]
    fn combat_lines_name_the_creature_and_show_damage() {
        let world = World::new(1);
        let content = world.content();
        let gnawer = content.kind_by_id("gnawer").unwrap();
        let hit = Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(gnawer),
            damage: Some(3),
        };
        assert_eq!(
            narrate(&hit, &world).unwrap(),
            ("You strike the gnawer (3).".to_string(), Tone::Normal)
        );
        let (sight, _) = narrate(&Event::FirstSighting { kind: gnawer }, &world).unwrap();
        assert!(sight.starts_with("A gnawer. A rat"));
    }

    #[test]
    fn whispers_and_warnings_carry_their_tone() {
        let world = World::new(1);
        assert_eq!(
            narrate(&Event::Whisper { seed: 3 }, &world).unwrap().1,
            Tone::Dread
        );
        assert_eq!(
            narrate(&Event::CandleGuttering, &world).unwrap().1,
            Tone::Danger
        );
        assert_eq!(
            narrate(&Event::TallowFound { amount: 5 }, &world)
                .unwrap()
                .1,
            Tone::Good
        );
    }

    #[test]
    fn articles() {
        assert_eq!(with_article("gnawer"), "a gnawer");
        assert_eq!(with_article("eel"), "an eel");
        assert_eq!(capitalize("a gnawer"), "A gnawer");
    }

    #[test]
    fn oldest_entries_fall_off() {
        let mut log = MessageLog::default();
        for i in 0..CAPACITY + 5 {
            log.push(i.to_string(), Tone::Normal);
        }
        assert_eq!(log.entries().len(), CAPACITY);
        assert_eq!(log.entries().next().unwrap().text, "5");
    }
}
