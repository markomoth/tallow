//! Key bindings: turns terminal key presses into app actions.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tallow_core::{Command, Direction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Game(Command),
    Quit,
}

/// Arrow keys, vi keys (`hjklyubn`) and the number row / numpad all move.
pub fn map_key(key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return (key.code == KeyCode::Char('c')).then_some(Action::Quit);
    }

    let step = |dir| Some(Action::Game(Command::Move(dir)));
    match key.code {
        KeyCode::Up | KeyCode::Char('k' | '8') => step(Direction::N),
        KeyCode::Down | KeyCode::Char('j' | '2') => step(Direction::S),
        KeyCode::Left | KeyCode::Char('h' | '4') => step(Direction::W),
        KeyCode::Right | KeyCode::Char('l' | '6') => step(Direction::E),
        KeyCode::Char('y' | '7') | KeyCode::Home => step(Direction::NW),
        KeyCode::Char('u' | '9') | KeyCode::PageUp => step(Direction::NE),
        KeyCode::Char('b' | '1') | KeyCode::End => step(Direction::SW),
        KeyCode::Char('n' | '3') | KeyCode::PageDown => step(Direction::SE),
        KeyCode::Char('.' | '5') => Some(Action::Game(Command::Wait)),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Option<Action> {
        map_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn arrows_vi_keys_and_numbers_agree() {
        for (a, b, c) in [
            (KeyCode::Up, KeyCode::Char('k'), KeyCode::Char('8')),
            (KeyCode::Left, KeyCode::Char('h'), KeyCode::Char('4')),
            (KeyCode::Home, KeyCode::Char('y'), KeyCode::Char('7')),
        ] {
            assert_eq!(press(a), press(b));
            assert_eq!(press(b), press(c));
            assert!(press(a).is_some());
        }
    }

    #[test]
    fn wait_and_quit() {
        assert_eq!(press(KeyCode::Char('.')), Some(Action::Game(Command::Wait)));
        assert_eq!(press(KeyCode::Char('5')), Some(Action::Game(Command::Wait)));
        assert_eq!(press(KeyCode::Char('q')), Some(Action::Quit));
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map_key(ctrl_c), Some(Action::Quit));
    }

    #[test]
    fn unbound_keys_do_nothing() {
        assert_eq!(press(KeyCode::Char('z')), None);
        let ctrl_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert_eq!(map_key(ctrl_k), None);
    }
}
