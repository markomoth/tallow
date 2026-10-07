# Tallow

Something is wrong with the Church of the Low Bell. The clergy dream the same dream and stop waking. Under the floorboards behind the altar there is a passage, and it keeps going down.

Tallow is a roguelike for the terminal. No character creation: press a key and you're in the crypt. Your candle is your light and your clock. Your build comes from what you do.

## Play

Needs [Rust](https://rustup.rs) and a truecolor terminal of at least 100×30.

```sh
cargo run --release
cargo run --release -- --seed 42   # replay a specific dungeon
```

| Key | Action |
|---|---|
| arrows / `hjkl` / numpad | Move |
| `yubn` | Move diagonally |
| Shift + direction | Run |
| `>` | Descend stairs |
| `.` / `5` | Wait |
| `q` | Quit |

## Status

Early development. See [`BUILD_GUIDE.md`](BUILD_GUIDE.md) for the design and milestone plan.
