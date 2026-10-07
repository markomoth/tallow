# Tallow

A dark-fantasy / dark-academia roguelike for the terminal, in Rust.

**`BUILD_GUIDE.md` is the source of truth** for design, scope, and milestones. Read it before starting a milestone. If a design decision changes, update the guide in the same change.

## Commands

```sh
cargo run                                  # play (binary: `tallow`, needs a 100×30 terminal)
cargo test --workspace                     # all tests
cargo clippy --workspace --all-targets     # must be warning-free
cargo fmt --all                            # format before committing
```

## Layout

- `crates/tallow-core` — pure game logic. `World::apply(Command) -> Vec<Event>`.
- `crates/tallow-tui` — the `tallow` binary: ratatui rendering, key bindings, message log.
- `assets/` — content embedded with `include_str!` (prefab maps now; RON data files later).

## Rules for the code

- `tallow-core` never prints, reads the clock, touches the terminal, or uses unseeded randomness. Same seed + same commands = same game.
- The UI sends `Command`s and reads `Event`s. Log text is written in `tallow-tui/src/log.rs` (`narrate`), never in core.
- Content (monsters, items, rites, boons) goes in data files, not Rust, unless it adds a new mechanic.
- No ECS. Plain structs; `slotmap` IDs once entities exist.
- Keep `ratatui` imports through `ratatui::crossterm` so there is one crossterm version.
- Respect the anti-gotcha rules in `BUILD_GUIDE.md` §1. They are hard constraints, not style.
- No D&D or franchise vocabulary in game text (no mana, potions, scrolls, fireballs, d20).

## Workflow

- Work milestone by milestone (`BUILD_GUIDE.md` §13). A milestone is done when its "Done when you can…" line is true in the real binary, tests pass, and clippy is clean. Then mark it ✅ in the guide.
- Every new system gets unit tests in its module. Map generation gets connectivity tests.
- Commit at the end of each milestone, or at a stable point inside one.

## Checking the real binary

The game is interactive, so drive it with tmux to see the actual screen:

```sh
cargo build
tmux new-session -d -s tallow -x 110 -y 32 ./target/debug/tallow
tmux send-keys -t tallow l l j      # play some keys
tmux capture-pane -t tallow -p      # print the screen
tmux send-keys -t tallow q          # quit
```

Wait for the first frame (poll `capture-pane` for `T A L L O W`) before sending keys.
