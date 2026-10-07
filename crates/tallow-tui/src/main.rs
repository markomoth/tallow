//! Tallow: a dark-fantasy roguelike for the terminal.

mod app;
mod input;
mod log;
mod render;

use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;

/// Redraw at least this often so candlelight can flicker.
const FRAME: Duration = Duration::from_millis(80);

const USAGE: &str = "usage: tallow [--seed <number>]";

fn main() -> Result<()> {
    let seed = parse_seed(std::env::args().skip(1))?.unwrap_or_else(random_seed);

    // `init` enters the alternate screen and installs a panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, seed);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, seed: u64) -> Result<()> {
    let mut app = App::new(seed);
    let started = Instant::now();
    while !app.should_quit() {
        if app.wants_restart() {
            app = App::new(random_seed());
        }
        let time = started.elapsed().as_secs_f32();
        terminal.draw(|frame| render::draw(frame, &app, time))?;
        if event::poll(FRAME)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && let Some(action) = input::map_key(key)
        {
            app.handle(action);
        }
    }
    Ok(())
}

fn parse_seed(mut args: impl Iterator<Item = String>) -> Result<Option<u64>> {
    let mut seed = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                let value = args.next().context(USAGE)?;
                seed = Some(
                    value
                        .parse()
                        .with_context(|| format!("bad seed {value:?}\n{USAGE}"))?,
                );
            }
            "-h" | "--help" => bail!(USAGE),
            other => bail!("unknown argument {other:?}\n{USAGE}"),
        }
    }
    Ok(seed)
}

/// A fresh seed from the OS-seeded hasher keys; no extra dependency needed.
fn random_seed() -> u64 {
    RandomState::new().hash_one(Instant::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn seed_flag() {
        assert_eq!(parse_seed(args(&[])).unwrap(), None);
        assert_eq!(parse_seed(args(&["--seed", "42"])).unwrap(), Some(42));
        assert!(parse_seed(args(&["--seed"])).is_err());
        assert!(parse_seed(args(&["--seed", "x"])).is_err());
        assert!(parse_seed(args(&["--nope"])).is_err());
    }
}
