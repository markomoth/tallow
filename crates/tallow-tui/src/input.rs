//! Key bindings: turns terminal key presses into app actions.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tallow_core::{Command, Direction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Game(Command),
    Quit,
}

/// Arrow keys, vi keys (`hjklyubn`) and the number row / numpad all move.
/// Shift with a vi key or arrow runs.
pub fn map_key(key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return (key.code == KeyCode::Char('c')).then_some(Action::Quit);
    }
    if let Some(dir) = direction(key.code) {
        let run = key.modifiers.contains(KeyModifiers::SHIFT)
            || matches!(key.code, KeyCode::Char(c) if c.is_ascii_uppercase());
        let command = if run {
            Command::Run(dir)
        } else {
            Command::Move(dir)
        };
        return Some(Action::Game(command));
    }
    match key.code {
        KeyCode::Char('.' | '5') => Some(Action::Game(Command::Wait)),
        KeyCode::Char('>') => Some(Action::Game(Command::Descend)),
        KeyCode::Char('<') => Some(Action::Game(Command::Ascend)),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

fn direction(code: KeyCode) -> Option<Direction> {
    let dir = match code {
        KeyCode::Up => Direction::N,
        KeyCode::Down => Direction::S,
        KeyCode::Left => Direction::W,
        KeyCode::Right => Direction::E,
        KeyCode::Home => Direction::NW,
        KeyCode::PageUp => Direction::NE,
        KeyCode::End => Direction::SW,
        KeyCode::PageDown => Direction::SE,
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'k' | '8' => Direction::N,
            'j' | '2' => Direction::S,
            'h' | '4' => Direction::W,
            'l' | '6' => Direction::E,
            'y' | '7' => Direction::NW,
            'u' | '9' => Direction::NE,
            'b' | '1' => Direction::SW,
            'n' | '3' => Direction::SE,
            _ => return None,
        },
        _ => return None,
    };
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Option<Action> {
        map_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn game(command: Command) -> Option<Action> {
        Some(Action::Game(command))
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
    fn shift_runs() {
        assert_eq!(press(KeyCode::Char('L')), game(Command::Run(Direction::E)));
        assert_eq!(press(KeyCode::Char('Y')), game(Command::Run(Direction::NW)));
        let shift_up = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
        assert_eq!(map_key(shift_up), game(Command::Run(Direction::N)));
        // Terminals report Shift on uppercase letters too; that's still a run.
        let shift_j = KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT);
        assert_eq!(map_key(shift_j), game(Command::Run(Direction::S)));
    }

    #[test]
    fn wait_stairs_and_quit() {
        assert_eq!(press(KeyCode::Char('.')), game(Command::Wait));
        assert_eq!(press(KeyCode::Char('5')), game(Command::Wait));
        assert_eq!(press(KeyCode::Char('>')), game(Command::Descend));
        assert_eq!(press(KeyCode::Char('<')), game(Command::Ascend));
        assert_eq!(press(KeyCode::Char('q')), Some(Action::Quit));
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map_key(ctrl_c), Some(Action::Quit));
    }

    #[test]
    fn unbound_keys_do_nothing() {
        assert_eq!(press(KeyCode::Char('z')), None);
        assert_eq!(press(KeyCode::Char('Q')), None);
        let ctrl_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert_eq!(map_key(ctrl_k), None);
    }
}
