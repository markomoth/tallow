//! The message log, and the narration that turns core events into text.

use std::collections::VecDeque;

use tallow_core::{Content, DreadBand, Event, Faction, KindId, Tile, Who};

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
pub fn narrate(event: &Event, content: &Content) -> Option<(String, Tone)> {
    use Tone::{Danger, Dread, Good, Normal};
    let name = |kind: KindId| &content.monster(kind).name;
    let (text, tone): (String, Tone) = match *event {
        Event::PlayerBlocked {
            tile: Tile::Wall, ..
        } => ("Cold stone. The wall does not give.".into(), Normal),
        Event::PlayerBlocked { .. } => ("Something blocks the way.".into(), Normal),
        Event::Spotted {
            tile: Tile::StairsDown,
            ..
        } => ("A stair leads further down.".into(), Normal),
        Event::Descended { depth } => (descent_line(depth).into(), Normal),
        Event::NoStairsHere => ("There are no stairs here.".into(), Normal),
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
        Event::Manifested => (
            "Your dread takes a shape. Something wearing your face is coming.".into(),
            Danger,
        ),
        Event::ManifestationBanished => (
            "The thing with your face is gone. You can breathe.".into(),
            Good,
        ),
        Event::ManifestationEscaped => (
            "You leave your dread on the floor above. For now.".into(),
            Good,
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
            assert!(narrate(&Event::Descended { depth }, Content::bundled()).is_some());
        }
    }

    #[test]
    fn combat_lines_name_the_creature_and_show_damage() {
        let content = Content::bundled();
        let gnawer = content.kind_by_id("gnawer").unwrap();
        let hit = Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(gnawer),
            damage: Some(3),
        };
        assert_eq!(
            narrate(&hit, content).unwrap(),
            ("You strike the gnawer (3).".to_string(), Tone::Normal)
        );
        let (sight, _) = narrate(&Event::FirstSighting { kind: gnawer }, content).unwrap();
        assert!(sight.starts_with("A gnawer. A rat"));
    }

    #[test]
    fn whispers_and_warnings_carry_their_tone() {
        let content = Content::bundled();
        assert_eq!(
            narrate(&Event::Whisper { seed: 3 }, content).unwrap().1,
            Tone::Dread
        );
        assert_eq!(
            narrate(&Event::CandleGuttering, content).unwrap().1,
            Tone::Danger
        );
        assert_eq!(
            narrate(&Event::TallowFound { amount: 5 }, content)
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
