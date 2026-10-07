//! Tallow: a dark-fantasy roguelike for the terminal.

mod app;
mod input;
mod log;
mod names;
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

/// Testing aids, not for play: start deeper, know every rite.
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    seed: Option<u64>,
    dev_depth: Option<u8>,
    dev_rites: bool,
    dev_kit: bool,
}

fn main() -> Result<()> {
    let options = parse_args(std::env::args().skip(1))?;
    let seed = options.seed.unwrap_or_else(random_seed);

    // `init` enters the alternate screen and installs a panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, seed, &options);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, seed: u64, options: &Options) -> Result<()> {
    let mut app = App::new(seed);
    if let Some(depth) = options.dev_depth {
        app.world_mut().dev_skip_to(depth);
    }
    if options.dev_rites {
        app.world_mut().dev_learn_all_rites();
    }
    if options.dev_kit {
        for (id, n) in [
            ("fire_flask", 3),
            ("lamp_oil", 2),
            ("handbell", 1),
            ("holy_water", 2),
        ] {
            app.world_mut().dev_give(id, n);
        }
    }
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
        {
            app.handle_key(key);
        }
    }
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Options> {
    let mut options = Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                let value = args.next().context(USAGE)?;
                options.seed = Some(
                    value
                        .parse()
                        .with_context(|| format!("bad seed {value:?}\n{USAGE}"))?,
                );
            }
            "--dev-depth" => {
                let value = args.next().context(USAGE)?;
                options.dev_depth = Some(value.parse().context("bad depth")?);
            }
            "--dev-rites" => options.dev_rites = true,
            "--dev-kit" => options.dev_kit = true,
            "-h" | "--help" => bail!(USAGE),
            other => bail!("unknown argument {other:?}\n{USAGE}"),
        }
    }
    Ok(options)
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
        let seed = |list: &[&str]| parse_args(args(list)).map(|o| o.seed);
        assert_eq!(seed(&[]).unwrap(), None);
        assert_eq!(seed(&["--seed", "42"]).unwrap(), Some(42));
        assert!(seed(&["--seed"]).is_err());
        assert!(seed(&["--seed", "x"]).is_err());
        assert!(seed(&["--nope"]).is_err());
    }

    #[test]
    fn dev_flags() {
        let o = parse_args(args(&["--dev-depth", "4", "--dev-rites"])).unwrap();
        assert_eq!(o.dev_depth, Some(4));
        assert!(o.dev_rites);
    }
}
