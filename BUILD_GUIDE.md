# TALLOW — Build Guide

A terminal roguelike in Rust. You are an acolyte who goes under the church floor, finds the stolen Vigil Candle in the court of Beelzebub, and carries it back to the altar.

This guide is the source of truth. Code follows it. If a decision changes, update this file first.

---

## 1. Pillars

Every feature must serve at least one. If a feature fights one, cut it.

1. **Start now.** No character creation, no menus before play. Press a key, you are in the crypt.
2. **Fair dark.** The world is cruel, the rules are not. You can always see why you died.
3. **Knowledge is loot.** Studying, reading, and noticing make you stronger, as much as gear does.
4. **Many answers.** Every big threat can be beaten by force, by rite, or by using the room.
5. **One hour.** A winning run takes about an hour. Death costs time, not progress in understanding.

### Anti-gotcha rules (hard constraints)

- Nothing kills you without at least one explicit, readable warning first.
- No consumable can kill you or cripple you permanently.
- No hunger. The clock is the candle, and it is always on screen.
- No permanent stat drain. No item destroyed without a confirmation or a warning turn.
- Auto-explore and running stop the moment something new is in view.
- Every floor guarantees at least one tallow source and a reachable down-stair.
- `Look` on any creature shows its hit chance against you, your hit chance against it, and every ability you have seen it use.
- Hallucinations (from dread) can never deal damage and are always detectable with `Look`.
- Death screen gives a short recap: what killed you, and the last 5 events that led there.
- No reference to D&D or any franchise: no d20, alignments, elves, mana, fireballs, potions, scrolls, wands, or "hit dice".

---

## 2. Setting

Kept vague on purpose. Names below are placeholders until we like them.

- **The town.** A small parish town. No era named; it feels like late candle-age: oil, wax, iron, paper.
- **The church.** The Church of the Low Bell. For months the clergy and villagers have been going mad. They dream the same dream, stop waking, and die in ways no one will describe.
- **The cause.** The Vigil Candle, the church's oldest relic, burned on the altar for centuries and kept the nightmare out. Someone took it below. Its absence is the wound.
- **The labyrinth.** Under the floorboards: the crypts, then a buried **Collegium**, a seminary where scholar-priests once studied the nightmare and lost. Below that, older and wetter places. At the bottom, Beelzebub holds court around the stolen candle.
- **Tone.** Dark academia meets folk horror. Libraries, lecture halls, anatomy theatres, ossuaries, flooded archives. Grim, quiet, literate. Not jokey, not gory for its own sake.

### Factions (they fight each other)

| Faction | Who | Hates | Fears |
|---|---|---|---|
| **The Dreaming** | Nightmares made flesh. Born in darkness. | All living | Light, holy water |
| **The Taken** | Possessed villagers and clergy. Still human underneath. | Living who resist | Exorcism, bells |
| **The Swarm** | Flies, vermin, rot. Beelzebub's own. | Everything with a pulse | Fire, smoke |
| **The Remnant** | Collegium wardens, bound books, old automata. Guard the archives. | Any intruder, including nightmares | Nothing; predictable |

Factions have a hostility matrix. Mind control, lures, noise and light let you turn them on each other.

---

## 3. The run

```
Church (start) ──► Floors 1–12 (descent) ──► Beelzebub ──► Ascent (4 shifting floors) ──► Altar (win)
```

| Floors | Biome | Mini-boss (floor) | Feel |
|---|---|---|---|
| 1–3 | **Undercroft & Crypts** | The Sexton (3) | Narrow, dark, tutorial-gentle |
| 4–6 | **The Collegium** | The Provost (6) | Halls, libraries, fire hazards, books to study |
| 7–9 | **The Drowned Stacks** | The Drowned Choir (9) | Flooded archives, ossuary wells, deep water drowns your candle |
| 10–11 | **The Rot Court** | — | Inverted cathedral, flies, corruption |
| 12 | **The Throne** | **Beelzebub** | Arena with braziers and corpses |
| A1–A4 | **The Unravelling** (ascent) | Pursuer | Re-rolled, corrupted mixes of earlier biomes |

Target pacing: ~4 minutes per descent floor, ~10 minutes for the ascent, ~1 hour total.

Mini-bosses always drop a **Leaving** (artifact, see §8).

**The descent is one-way.** The stair behind you is sealed (rubble on deeper floors; on floor 1 the church above is no refuge). No stair-scumming, and the candle clock stays honest. Every floor is generated from the run seed and its depth, independent of other floors.

### Beelzebub

Three phases. Each phase has a force answer, a rite answer, and a room answer.

1. **The Court.** He is armored in a living swarm that thickens in darkness. Light the arena braziers (room), burn the swarm (fire flask), or Hush/Veil to make him lose track of you.
2. **The Possession.** He leaps between corpses and controlled creatures. Exorcise or Bind pins him in a body. Rendering or burning corpses removes his hosts.
3. **The Lord.** Slow, huge, every attack telegraphed one turn ahead (marked tiles). Pure dodge-and-hit, with rites as openings.

### The ascent

- Taking the Vigil Candle ends your tallow worries: it is infinite light.
- It also wakes **the Following**: a remnant of the swarm that enters each ascent floor a set number of turns after you, from the stair you came up.
- The Following cannot be killed. It can be slowed: closed doors, holy water, sanctified ground, braziers.
- Ascent floors are re-rolled (not the same maps), shorter, and visibly corrupted.
- Placing the candle on the altar wins. The epilogue varies by: Taken exorcised, dread at the end, rites learned.

---

## 4. Core loop: Candle and Dread

Two linked resources, both always on screen.

### Candle (tallow)

- Your candle is your light radius (6 tiles) and your clock.
- Burns 1 tallow per turn. Start: 900 turns of tallow.
- **Warnings:** the log and HUD warn below 200 ("candle low"). Below 60 it **gutters**: radius drops to 3.
- **Snuff** (`c`, one turn): radius 0, tallow saved, dread rises faster. In the dark, monsters only notice you within 2 tiles (unless you stand in brazier light).
- **Sources:** every floor has one guaranteed lump (180–260) plus 0–2 stubs (50–90), shown as `,`. Walk over tallow to take it. Later: braziers you light (M7) and **rendering corpses** (M6).
- **Lantern-eaters** bite your flame: the candle goes out and loses 15 tallow.
- Tallow is a counter for now. It gains weight when inventory arrives (M4).
- Deep water puts out your candle. Always telegraphed (tile color + confirm prompt).

### Dread (0–100)

| Band | Range | Effect |
|---|---|---|
| Calm | 0–39 | Normal. Rites at 1.0× potency. |
| Uneasy | 40–69 | Whispers in the log, false sounds. Rites at 1.25×. |
| Frayed | 70–99 | Hallucinated creatures (harmless, detectable with Look). Rites at 1.5×. |
| **Manifestation** | 100 | Your nightmare takes form as a strong hunter near you. Kill it or escape the floor. Dread resets to 50. Never an instant death. |

**Raises dread:** time in candlelight (+1 per 20 turns), time in darkness (+1 per 4 turns), first sight of a new kind of creature (+5, or +8 for the Dreaming), casting rites (M6).
**Lowers dread:** standing in brazier light (−1 per ~3 turns), consecrated rooms, incense, killing or escaping your Manifestation (dread settles at 50).

- **Whispers** (Uneasy and up) are flavor only, in violet. They never hint at real threats.
- **Phantoms** (Frayed) take the shape of creatures you've met. They hunt you, never attack, and come apart when they reach you or are struck. Look says plainly that they aren't there; they also shimmer very slightly.
- **The Manifestation** (`M`, 18 health, a little faster than you) always knows where you are. While it lives, dread stays at 100.
- **Rest** (`R`) waits until healed (and, by a brazier, until calm). It stops for anything that happens and won't start with company in view.

The key tension: **rites cost dread, and dread makes rites stronger.** Players dial their own risk.

### Corpses are a decision

Every corpse offers three choices:
- **Study** (10–20 turns): journal entry, weakness revealed, sometimes a rite fragment.
- **Render** (10 turns): tallow.
- **Leave it:** after ~100 turns it rots and a fly swarm hatches. Fire also removes it.

---

## 5. Character growth

No attributes. No skill trees. No classes. Your build is what you do.

### Skills (level from use)

| Skill | Grows from |
|---|---|
| Blades | Hitting with knives, swords, sickles |
| Bludgeons | Hitting with censers, maces, staves |
| Reach | Hitting with spears, hooks, poles |
| Missiles | Slings, crossbows, thrown things |
| Binding | Casting control rites |
| Communion | Casting leech / transfer rites |
| Veil | Casting concealment / illusion rites |
| Warding | Casting protection / banishing rites |
| Endurance | Taking hits while armored |

- Ranks 0–10. Rank n needs roughly `n² × k` meaningful uses.
- **Meaningful use only:** XP counts only against creatures that are a real threat (no grinding rats).
- Ranks unlock fixed techniques (e.g. Blades 3: riposte; Reach 4: hit two tiles; Missiles 5: pin to wall). Hand-written, listed in data files.

### Combat basics

- **Hit chance** = attacker accuracy − defender defense, clamped to 5–95%. Look shows the exact numbers.
- **Speed:** 10 is one action per turn. Energy-based: faster creatures act more often.
- **Health:** the acolyte starts with 24 and regains 1 every 12 turns. The candle clock keeps resting honest.
- **Telegraphs:** a raised heavy blow marks its target tile in pulsing red and always waits for your next action.
- **Sight is symmetric:** anything in your line of sight can see your candle. Monsters in the dark can notice you before you see them; the log always says so ("Something in the dark has noticed your light.").
- **First sight** of each creature kind prints its description, so you learn its trick before it matters.
- **Runs** are refused while anything hostile is in view.

### Character level

- XP from: first-time discoveries (new monster, new rite, new floor), studying, defeating threats.
- Each level: +health, then **pick 1 of 3 boons**. One keypress. No stat math shown.
- Boons are procedural: `trigger × effect`, weighted toward skills you actually use.
  - "When you snuff your candle, the nearest Dreaming loses track of you."
  - "Leech also lowers your dread."
  - "Thrown flasks shatter in a cross."
- Draft never offers dead picks (boons for rites you don't know, etc.).

### Rites (magic)

Situational, manipulative, useful to every build. Cost dread. Learned by **study**: hymnal pages, heretic notes, studied corpses.

| School | Example rites |
|---|---|
| **Binding** | *Compel* (control one creature for N turns), *Kneel* (root), *Turncoat* (switch a creature's faction) |
| **Communion** | *Leech* (drain life), *Transference* (give your wounds or dread to another), *Borrowed Eyes* (see through a creature) |
| **Veil** | *Hush* (silence an area), *Unsee* (one creature forgets you), *False Flame* (a decoy light that lures) |
| **Warding** | *Sanctify* (ground nightmares can't cross), *Exorcise* (free a Taken; they flee as a villager), *Seal* (bar a door) |

Target for v1: 16 rites (4 per school). No damage-only spells. Every rite changes a situation.

---

## 6. Items

### Weight-based inventory

- Every item has weight. Thresholds shown on HUD:
  - **Unburdened:** normal.
  - **Burdened:** moving costs 1.5× time. (Tallow burns per turn, so this matters.)
  - **Overloaded:** cannot move until you drop something.
- Tallow lumps have weight. Carrying light is a real trade.

### Categories

| Category | Examples |
|---|---|
| Melee | Sickle, cleaver, censer-on-chain, iron candlestick, boathook, verger's staff |
| Ranged | Sling (stones also probe anomalies, see §8), crossbow, throwing knives |
| Throwables | Oil flask, holy water, smoke pot, chalk (draws a ward line) |
| Vestments | Cassock, gambeson, sexton's leathers, choir mail (heavy) |
| Tinctures | Mending, steadying (−dread), waking (+speed), etc. |
| Texts | Hymnal pages, heretic notes, Collegium lecture fragments (teach rites) |
| Tools | Handbell (noise lure), crowbar (pry, break), tallow lumps, incense |

### Identification: known category, learn by use

- You always see the category: "a mending tincture (strength unknown)".
- Using it reveals the exact strength and any side effect for the rest of the run.
- Side effects are never lethal and never permanent.

---

## 7. Environment

The room is a weapon. All of these must be readable on screen.

- **Light:** braziers, wall candles, fallen candles. Light hurts the Dreaming and reveals you.
- **Fire:** spreads over books, pews, rugs, oil, dry bone. Burns the Swarm. Burns you too.
- **Oil:** spill and ignite. Slippery.
- **Holy water:** consecrates tiles for N turns. Nightmares avoid, Taken flinch.
- **Bells:** wall bells and handbells make noise; noise draws nearby creatures. Lure factions into each other.
- **Doors:** open, close, bar, Seal. Wooden ones burn.
- **Rotten floors:** look different, collapse under weight, drop you one floor. Never on the critical path without an alternative.
- **Water:** shallow slows you; deep drowns your candle (confirm prompt).

---

## 8. Leavings (artifacts)

Inspired by the Zone in *Roadside Picnic*: things the nightmare leaves behind where reality broke. Powerful, strange, dangerous.

### Where they come from

- Mini-boss drops (guaranteed).
- **Seep rooms:** rare rooms full of invisible **anomalies** (gravity pulls, time pockets, heat wells, swap points).
- Anomalies are invisible until probed: **throw a stone** or any object to reveal what it does. (Direct homage to throwing bolts.)

### How they work

- Each Leaving has a fixed rule for the run: `trigger × effect × cost`.
  - Trigger: on use, while carried, on hit, at a dread threshold, on entering darkness.
  - Effect: swap places with a creature, skip time, pull everything adjacent, invert light and dark, duplicate an item, etc.
  - Cost: health, dread, tallow, weight, attention of a faction.
- They can be truly dangerous, even lethal. **But always telegraphed:**
  1. A danger tier shown by glyph color and a tell ("it is warm and hums near the living").
  2. A warning message on first pickup.
  3. Any lethal effect gives a warning turn before it fires, and dropping the item stops it.
- Target for v1: ~20 rules generated from ~8 triggers, ~15 effects, ~8 costs, plus 4 hand-made named Leavings for mini-bosses.

---

## 9. Monsters (starter roster)

Target for v1: ~30 types. Each has one clear trick.

| Biome | Monster | Faction | Trick |
|---|---|---|---|
| Crypts | Gnawer pack | Swarm | Weak alone, flanks in groups |
| Crypts | Taken Parishioner | Taken | Slow, pleads; Exorcise frees them |
| Crypts | Lantern-Eater | Dreaming | Won't cross brazier light; snuffs nearby light (M3) |
| Crypts | Pallbearer | Taken | Raises a heavy blow at a marked tile; step aside |
| Crypts | **The Sexton** | Taken | Buries and raises corpses |
| Collegium | Inkling | Dreaming | Blinds (radius drop) |
| Collegium | Proctor | Remnant | Attacks any intruder, even nightmares |
| Collegium | Feverish Scholar | Taken | Casts rites back at you |
| Collegium | Bound Folio | Remnant | Flies, studying it teaches a rite |
| Collegium | **The Provost** | Remnant | Rewrites the room (doors lock, books ignite) |
| Drowned Stacks | Drowned Deacon | Taken | Drags you toward deep water |
| Drowned Stacks | Bone Choir | Dreaming | Singing raises dread |
| Drowned Stacks | Bloatfly | Swarm | Bursts into a fly swarm |
| Drowned Stacks | **The Drowned Choir** | Dreaming | Several bodies, one voice |
| Rot Court | Fly Herald | Swarm | Summons swarms in darkness |
| Rot Court | Courtier | Dreaming | Possesses corpses |
| Rot Court | Mother of Maggots | Swarm | Turns corpses into spawn |

Every attack that hits for more than ~30% of your health must be telegraphed one turn ahead.

---

## 10. Presentation

### Layout (minimum 100×30)

```
┌──────────────────────────── map ─────────────────────────────┬──── status ────┐
│                                                              │ Acolyte  Lv 4  │
│                    ASCII map, truecolor,                     │ Health ████░░  │
│                    candlelight falloff,                      │ Dread  ██░░░░  │
│                    fog of war tinted blue-grey               │ Candle █████░  │
│                                                              │ Burdened       │
│                                                              │ ─ in view ─    │
│                                                              │ T Parishioner  │
│                                                              │ i Inkling      │
├──────────────────────────── log ─────────────────────────────┴────────────────┤
│ The candle gutters. Something in the stacks stops breathing.                  │
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Glyphs:** ASCII. `@` you, `#` wall, `.` floor, `+` door, `>` `<` stairs, `&` brazier, `,` tallow, letters for creatures, `!` tinctures, `?` texts, `*` Leavings, `~` water, `"` vestments, `/` `|` `)` weapons.
- **Lighting:** per-tile light value blended into foreground and background. Warm amber falloff from the candle; braziers orange; holy light pale gold; darkness near-black.
- **Flicker:** small per-frame jitter on candle light (visual only, never affects rules).
- **Remembered tiles:** desaturated blue-grey.
- **Truecolor** with automatic 256-color fallback.
- **Animations:** short and skippable: projectile trails, fire spread, swarm movement.

### Controls

| Key | Action |
|---|---|
| arrows / `hjklyubn` / numpad | Move, bump to attack |
| Shift + direction (`HJKLYUBN`, ⇧arrows) | Run until something interesting: a wall, a door or stair underfoot or alongside, a new landmark in view, or a side passage in a corridor. Later: any creature or item coming into view. |
| `.` / `5` | Wait |
| `<` `>` | Stairs |
| `g` | Pick up |
| `i` | Inventory (use, equip, drop) |
| `z` | Cast rite |
| `f` | Fire ranged |
| `t` | Throw |
| `x` | Look (cursor; Tab cycles creatures, Esc leaves) |
| `s` | Study / render corpse |
| `c` | Snuff / relight candle |
| `R` | Rest until healed (or calm, by a brazier) |
| `o` | Auto-explore |
| `J` | Journal |
| `?` | Help |
| `q` / Ctrl-C | Quit (saves the run once saves exist) |

---

## 11. Meta: lore only

- Nothing that adds power carries over.
- The **Journal** persists across runs: monsters met (and weaknesses studied), rite names, Leavings seen, town history pages.
- Saved separately from the run save.

### Saves

- Save on quit, resume on launch. Save is deleted when loaded (no save-scumming).
- Every run has a seed shown on death/win screens. A seed can be passed on the command line to replay the same dungeon.

---

## 12. Technical architecture

### Stack

| Need | Choice |
|---|---|
| Language | Rust, stable, edition 2024 |
| Terminal UI | `ratatui` + `crossterm` |
| RNG | `rand` + `rand_pcg` (seeded, separate streams) |
| Data / saves | `serde` + `ron` (content), `postcard` (saves) |
| Entities | `slotmap` (generational IDs), plain structs. **No ECS.** |
| Paths | `directories` (save + journal location) |
| Errors | `anyhow` in binary, `thiserror` in core |

Why no ECS: ~30 monster types and ~60 item types is small. Plain structs with optional fields are easier to debug and to let Claude Code edit safely.

Algorithms we write ourselves (small, well-known): symmetric shadowcasting FOV, A* and Dijkstra maps, BSP rooms + cellular caves + prefab vaults, light propagation.

### Workspace layout

```
tallow/
├── Cargo.toml                 # workspace
├── BUILD_GUIDE.md
├── assets/                    # RON content, embedded with include_str!
│   ├── prefabs/               # hand-drawn ASCII vaults (parser: map/prefab.rs)
│   ├── monsters.ron
│   ├── items.ron
│   ├── rites.ron
│   ├── boons.ron
│   ├── leavings.ron
│   └── biomes.ron
├── crates/
│   ├── tallow-core/           # pure game logic. No IO, no terminal. Deterministic.
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── world.rs       # World, floors, entities
│   │       ├── map/           # tiles, mapgen, fov, light, pathing
│   │       ├── actions.rs     # Command enum → resolution
│   │       ├── events.rs      # Event enum (feeds log + animations)
│   │       ├── time.rs        # energy-based turn scheduler
│   │       ├── combat.rs
│   │       ├── ai.rs          # behaviours + faction matrix
│   │       ├── candle.rs      # tallow + light
│   │       ├── dread.rs
│   │       ├── skills.rs
│   │       ├── rites.rs
│   │       ├── items.rs
│   │       ├── leavings.rs
│   │       ├── environment.rs # fire, water, holy water, collapse
│   │       └── content.rs     # loads RON
│   ├── tallow-tui/            # the game binary (`tallow`): rendering + input
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs         # screens + state machine
│   │       ├── render/        # map, hud, log, menus, lighting
│   │       └── input.rs
│   └── tallow-sim/            # headless bot: plays N seeds, reports softlocks + stats
└── tests/
```

### Core rules for the code

- `tallow-core` never prints, never reads the clock, never touches the terminal. UI sends `Command`s, core returns `Event`s.
- All randomness goes through the world's seeded RNG streams (mapgen stream per floor, combat stream, AI stream). Same seed + same inputs = same game.
- Content lives in `assets/*.ron`. New monsters or items should not need Rust changes unless they add a new mechanic.
- Every message in the log comes from an `Event`. No ad hoc strings scattered in logic.

### Testing

- Unit tests: FOV symmetry, pathing, light, dread bands, weight thresholds.
- Property tests on mapgen: every floor connected, stairs reachable, ≥1 tallow source.
- `tallow-sim`: a dumb bot plays 1000 seeds; reports softlocks, average death floor, tallow starvation rate. Used for balance.

---

## 13. Milestones

Each milestone ends with something you can play. Mark a milestone ✅ in the table when it is done. Estimates assume Claude Code writes the code and you review and playtest; they are rough.

### Phase A: Playable core (~1–2 days)

| # | Milestone | Done when you can… | Est. |
|---|---|---|---|
| M0 ✅ | Skeleton | Run `cargo run`, see `@` on a map, move, quit | 1–2 h |
| M1 ✅ | Map & sight | Walk procedurally generated floors with FOV, candlelight falloff, fog, stairs down | 3–4 h |
| M2 ✅ | Combat & monsters | Fight 4 crypt monsters with energy-based turns, see the log, die, see death recap | 3–4 h |
| M3 ✅ | Candle & dread | Watch the candle burn, snuff it, gain dread, trigger a Manifestation | 2–3 h |

### Phase B: Depth (~2–3 days)

| # | Milestone | Done when you can… | Est. |
|---|---|---|---|
| M4 | Items & weight | Pick up, equip, throw, get Burdened, learn a tincture by drinking it | 3–4 h |
| M5 | Skills & levels | See Blades rank up from use, pick 1 of 3 boons on level up | 2–3 h |
| M6 | Rites & study | Study a corpse, learn Compel from a page, mind-control a Proctor into a Taken | 3–4 h |
| M7 | Factions & environment | Ring a bell to pull a swarm into a nightmare; set a library on fire | 4–5 h |

### Phase C: The full run (~3–4 days)

| # | Milestone | Done when you can… | Est. |
|---|---|---|---|
| M8 | Biomes & content | Descend all 12 floors through 4 biomes, beat 3 mini-bosses | 5–6 h |
| M9 | Leavings | Throw a stone into a Seep room, find an anomaly, take a Leaving, survive its warning | 3–4 h |
| M10 | Beelzebub & ascent | Beat 3 phases, take the candle, outrun the Following, win at the altar | 5–6 h |
| M11 | Polish & balance | Save/quit/resume, Journal persists, help screen, sim shows ~0 softlocks | 4–6 h |

Total: roughly 40–55 hours of build + playtest time.

---

## 14. Open questions (decide when we get there)

- Final names: town, church, Collegium, the Following.
- In-game words for level and XP (candidates: "Vigil" / "Resolve").
- Exact numbers: tallow per floor, dread rates, skill XP curve. Tune with `tallow-sim` in M11.
- Whether the ascent should show a turn counter for the Following or only audio-style log cues.
