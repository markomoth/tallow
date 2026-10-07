//! The message log, and the narration that turns core events into text.

use std::collections::VecDeque;

use tallow_core::{Content, Event, Faction, KindId, Tile, Who};

const CAPACITY: usize = 200;

/// One line in the log. Repeats of the same text collapse into a count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
    pub count: u32,
}

#[derive(Debug, Default)]
pub struct MessageLog {
    entries: VecDeque<Entry>,
}

impl MessageLog {
    pub fn push(&mut self, text: impl Into<String>) {
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
        self.entries.push_back(Entry { text, count: 1 });
    }

    /// Entries from oldest to newest.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &Entry> + ExactSizeIterator {
        self.entries.iter()
    }
}

/// The log line for an event, if it deserves one. Routine movement stays quiet.
pub fn narrate(event: &Event, content: &Content) -> Option<String> {
    let name = |kind: KindId| &content.monster(kind).name;
    let text = match *event {
        Event::PlayerBlocked {
            tile: Tile::Wall, ..
        } => "Cold stone. The wall does not give.".into(),
        Event::PlayerBlocked { .. } => "Something blocks the way.".into(),
        Event::Spotted {
            tile: Tile::StairsDown,
            ..
        } => "A stair leads further down.".into(),
        Event::Descended { depth } => descent_line(depth).into(),
        Event::NoStairsHere => "There are no stairs here.".into(),
        Event::StairsSealed { depth: 1 } => {
            "Above is only the church, and the dreamers. Not yet.".into()
        }
        Event::StairsSealed { .. } => "Rubble chokes the stair behind you. The way is down.".into(),
        Event::RunRefused => "Not with something watching.".into(),

        Event::FirstSighting { kind } => {
            let def = content.monster(kind);
            format!(
                "{}. {}",
                capitalize(&with_article(&def.name)),
                def.description
            )
        }
        Event::Noticed { kind, seen: true } => format!("The {} notices you.", name(kind)),
        Event::Noticed { seen: false, .. } => {
            "Something in the dark has noticed your light.".into()
        }
        Event::Bark { kind, line } => {
            let def = content.monster(kind);
            format!("The {} whispers: {}", def.name, def.barks.get(line)?)
        }
        Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(kind),
            damage: Some(d),
        } => {
            format!("You strike the {} ({d}).", name(kind))
        }
        Event::Attack {
            attacker: Who::Player,
            defender: Who::Monster(kind),
            damage: None,
        } => {
            format!("You miss the {}.", name(kind))
        }
        Event::Attack {
            attacker: Who::Monster(kind),
            damage: Some(d),
            ..
        } => {
            format!("The {} hits you ({d}).", name(kind))
        }
        Event::Attack {
            attacker: Who::Monster(kind),
            damage: None,
            ..
        } => {
            format!("The {} misses you.", name(kind))
        }
        Event::Attack {
            attacker: Who::Player,
            defender: Who::Player,
            ..
        } => return None,
        Event::WindUp { kind, .. } => {
            format!("The {} raises something heavy over you. Move!", name(kind))
        }
        Event::HeavyBlow {
            damage: Some(d), ..
        } => format!("The heavy blow lands ({d})."),
        Event::HeavyBlow {
            kind, damage: None, ..
        } => {
            format!("The {}'s blow smashes empty floor.", name(kind))
        }
        Event::Fled { kind } => format!("The {} squeals and breaks away.", name(kind)),
        Event::MonsterDied { kind, .. } => death_line(content, kind),
        Event::PlayerDied { .. } => "You die.".into(),
        Event::Spotted { .. } | Event::PlayerMoved { .. } | Event::PlayerWaited => return None,
    };
    Some(text)
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
        log.push("a");
        log.push("a");
        log.push("b");
        let entries: Vec<_> = log.entries().cloned().collect();
        assert_eq!(
            entries,
            vec![
                Entry {
                    text: "a".into(),
                    count: 2
                },
                Entry {
                    text: "b".into(),
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
            "You strike the gnawer (3)."
        );
        let sight = narrate(&Event::FirstSighting { kind: gnawer }, content).unwrap();
        assert!(sight.starts_with("A gnawer. A rat"));
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
            log.push(i.to_string());
        }
        assert_eq!(log.entries().len(), CAPACITY);
        assert_eq!(log.entries().next().unwrap().text, "5");
    }
}
