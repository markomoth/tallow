//! The message log, and the narration that turns core events into text.

use std::collections::VecDeque;

use tallow_core::{Event, Tile};

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
pub fn narrate(event: &Event) -> Option<&'static str> {
    match *event {
        Event::PlayerBlocked {
            tile: Tile::Wall, ..
        } => Some("Cold stone. The wall does not give."),
        Event::PlayerBlocked { .. } => Some("Something blocks the way."),
        Event::Spotted {
            tile: Tile::StairsDown,
            ..
        } => Some("A stair leads further down."),
        Event::Descended { depth } => Some(descent_line(depth)),
        Event::NoStairsHere => Some("There are no stairs here."),
        Event::StairsSealed { depth: 1 } => {
            Some("Above is only the church, and the dreamers. Not yet.")
        }
        Event::StairsSealed { .. } => Some("Rubble chokes the stair behind you. The way is down."),
        Event::Spotted { .. } | Event::PlayerMoved { .. } | Event::PlayerWaited => None,
    }
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
            assert!(narrate(&Event::Descended { depth }).is_some());
        }
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
