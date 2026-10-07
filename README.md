# Tallow

Something is wrong with the Church of the Low Bell. The clergy dream the same dream and stop waking. Under the floorboards behind the altar there is a passage, and it keeps going down.

Tallow is a roguelike for the terminal. No character creation: press a key and you're in the crypt. Your candle is your light and your clock. Your build comes from what you do.

## Play

Needs [Rust](https://rustup.rs) and a truecolor terminal of at least 100×30.

```sh
cargo run --release
cargo run --release -- --seed 42   # replay a specific dungeon
cargo run --release -- --simple    # plain terminal colors on your own background, no animation
```

`--simple` is for terminals without truecolor, light themes, transparent backgrounds, screen readers or recordings: it uses only the 16 standard colors, leaves the background to your terminal, and redraws only when you press a key.

| Key | Action |
|---|---|
| arrows / `hjkl` / numpad | Move |
| `yubn` | Move diagonally |
| Shift + direction | Run |
| `>` / `<` | Down the stairs / up, once you carry the Vigil Candle |
| `x` | Look (Tab cycles creatures) |
| `c` | Snuff / light your candle |
| `C` | Shut doors beside you |
| `R` | Rest |
| `g` | Pick up |
| `i` | Pack |
| `t` / `f` | Throw / fire |
| `s` | Study or render the body underfoot |
| `z` | Cast a rite |
| `@` | Character sheet |
| `o` | Auto-explore |
| `M` | Journal (what all your runs have taught you) |
| `?` | Help |
| `.` / `5` | Wait |
| `q` | Save and quit (the next launch picks up where you left off) |

## Status

All milestones of the build guide are in: twelve floors down, Beelzebub, the ascent and the altar. See [`BUILD_GUIDE.md`](BUILD_GUIDE.md) for the design.
