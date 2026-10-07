//! Tallow: a dark-fantasy roguelike for the terminal.

mod app;
mod input;
mod journal;
mod log;
mod names;
mod render;
mod save;

use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;

/// Redraw at least this often so candlelight can flicker.
const FRAME: Duration = Duration::from_millis(80);
/// In simple mode, wait for input this long before redrawing anyway.
const IDLE: Duration = Duration::from_secs(5);

const USAGE: &str = "usage: tallow [--seed <number>] [--simple]

  --seed <number>   play (or replay) a particular dungeon
  --simple          plain terminal colors on your own background, no animation";

/// Testing aids, not for play: start deeper, know every rite.
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    seed: Option<u64>,
    dev_depth: Option<u8>,
    dev_rites: bool,
    dev_kit: bool,
    dev_level: Option<u32>,
    dev_near_stairs: bool,
    dev_ascent: Option<u8>,
    /// Plain terminal colors, the terminal's own background, no animation.
    simple: bool,
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
    let dev = options.dev_depth.is_some()
        || options.dev_rites
        || options.dev_kit
        || options.dev_level.is_some()
        || options.dev_near_stairs
        || options.dev_ascent.is_some();
    // Dev runs and replays of a chosen seed don't touch your save or journal.
    let home = if dev { None } else { save::home() };
    let save_path = home.as_ref().map(|h| h.join("save.ron"));
    let journal_path = home.as_ref().map(|h| h.join("journal.ron"));
    let mut journal = journal_path
        .as_deref()
        .map(journal::Journal::load)
        .unwrap_or_default();
    let resumed = match (&save_path, options.seed) {
        (Some(path), None) => save::Save::take(path),
        _ => None,
    };
    let mut app = match &resumed {
        Some(save) => App::resume(save),
        None => App::new(seed),
    };
    app.set_journal(journal.clone());
    app.set_simple(options.simple);
    // Dev runs go straight in; everyone else starts at the church door.
    if !dev {
        app.show_title();
    }
    let mut recorded = false;
    // Simple mode doesn't animate, so it only needs to redraw on input.
    let frame = if options.simple { IDLE } else { FRAME };
    if let Some(depth) = options.dev_depth {
        app.world_mut().dev_skip_to(depth);
    }
    if options.dev_rites {
        app.world_mut().dev_learn_all_rites();
    }
    if let Some(floor) = options.dev_ascent {
        app.world_mut().dev_ascent(floor);
    }
    if options.dev_near_stairs {
        app.world_mut().dev_near_stairs(8);
    }
    if let Some(level) = options.dev_level {
        app.world_mut().dev_level_to(level);
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
            app.set_journal(journal.clone());
            app.set_simple(options.simple);
            recorded = false;
        }
        // A finished run goes into the journal once.
        if app.is_over() && !recorded {
            recorded = true;
            journal.end_run(app.world());
            if let Some(path) = &journal_path {
                journal.save(path).ok();
            }
            app.set_journal(journal.clone());
        }
        let time = started.elapsed().as_secs_f32();
        terminal.draw(|frame| render::draw(frame, &app, time))?;
        if event::poll(frame)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }
    }
    // Quitting mid-run saves it; what you learned goes into the journal either way.
    // Leaving from the start menu before a single move saves nothing.
    if !app.is_over() && app.started() {
        journal.record(app.world());
        if let Some(path) = &save_path {
            app.save().write(path)?;
        }
    }
    if let Some(path) = &journal_path {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        journal.save(path).ok();
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
            "--simple" => options.simple = true,
            "--dev-kit" => options.dev_kit = true,
            "--dev-near-stairs" => options.dev_near_stairs = true,
            "--dev-ascent" => {
                let value = args.next().context(USAGE)?;
                options.dev_ascent = Some(value.parse().context("bad ascent floor")?);
            }
            "--dev-level" => {
                let value = args.next().context(USAGE)?;
                options.dev_level = Some(value.parse().context("bad level")?);
            }
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
    fn simple_flag() {
        assert!(parse_args(args(&["--simple"])).unwrap().simple);
        assert!(!parse_args(args(&[])).unwrap().simple);
    }

    #[test]
    fn dev_flags() {
        let o = parse_args(args(&["--dev-depth", "4", "--dev-rites"])).unwrap();
        assert_eq!(o.dev_depth, Some(4));
        assert!(o.dev_rites);
    }
}
