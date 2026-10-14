# Changelog

What changed in each release of Tallow. Each version's section is attached to its [GitHub release](https://github.com/markomoth/tallow/releases) when its tag is pushed.

## [Unreleased]

### Added

- 🌐 Tallow in the browser: the same game on a canvas, saved in your browser, published to itch.io on each version tag.
- 🦋 The tallow moth: it flies over lit ground, out of reach of anything held in the hand. Throw at it, or put your light out so it settles. Floors with moths have stones near the stair.
- Auto-explore now walks one step at a time, five a second, and any key stops it.
- Too close to aim: throwing or firing at a creature beside you is −30 to hit, shown in the aim panel.
- Stones, bolts and knives are sometimes spent when they hit (one time in three).

### Changed

- The Manifestation is harder: 26 health (was 18), 3–5 damage (was 2–4).
- Vestments retuned: gambeson 9, sexton's leathers 12, choir mail 18 defense.
- Balanced so no single play style wins alone: starter weapon only, always lit, never lit, rites only, missiles only and best-gear-only bots each win 9% of runs or fewer, and a mixed style wins 45%.

### Fixed

- The build guide now matches the code on the Manifestation: dread isn't held at 100 while it lives, and Banish only buys distance from it.

## [0.2.0] - 2026-10-08

### Changed

- The screen, redrawn:
  - The sidebar shows the floor name, health with its maximum, dread bands, what each creature in view is doing, and the keys that fit the moment.
  - A turn's log lines light up together, and wrapped lines hang under their text.
  - Look opens as a card beside its target, never over it.
  - Popups share one frame with a "more" count instead of cutting text silently.
  - Rites open in two panes: read with the arrow keys, cast with a letter or Enter.
- When frayed, the map's edges bruise violet.
- Remembered tiles and dim text are easier to read.

## [0.1.0] - 2026-10-08

The first release: a whole run, from the Undercroft down to Beelzebub's throne and back up to the church.

### Added

- **The dungeon:** 12 generated floors in four biomes, symmetric sight, candlelight and fog, doors, fire, water, rotten boards, bells and noise.
- **Candle and dread:** scarce tallow, light that's seen from afar, darkness that hides you, and dread as the price of rites. Whispers, phantoms, Nightmares and the Manifestation come as dread rises.
- **Combat:** energy turns, telegraphed heavy blows, factions that fight each other and in their own ways. You can shove and guard.
- **Gear:** weapons with quirks, armour that matters by faction, slings, crossbows, throwing knives, holy water, tinctures you learn by drinking, and a weight limit.
- **Growth:** skills that rise with use, levels, boon drafts, and 16 rites in four schools, learned from texts and bodies, with recharge times and limited slots.
- **Leavings:** strange artifacts with a rule learned when they wake, found in seep rooms among hidden anomalies.
- **Bosses:** the Sexton, the Provost, the Drowned Choir, Beelzebub's three phases, the Vigil Candle and the ascent with the Following behind you.
- **Around the game:** saves, a journal that lasts across runs, help, auto-explore, a start menu, `--simple` for plain terminals, and downloads for Linux, Windows and macOS.

[Unreleased]: https://github.com/markomoth/tallow/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/markomoth/tallow/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/markomoth/tallow/commits/v0.1.0
