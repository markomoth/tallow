# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

The game is Rust. The browser build (`crates/tallow-web`) is the same game compiled to WebAssembly and drawn on a WebGL canvas by Ratzilla. Its host page is static HTML/CSS bundled by `trunk`, deployed to GitHub Pages and uploaded to itch.io as an HTML5 game (where it runs inside an iframe).

## Users

Two audiences, served equally by one page:

- Strangers browsing itch.io who have never heard of Tallow and need to understand what it is in seconds and start playing.
- People arriving from the GitHub README who already know it is a terminal roguelike and want to play without installing.

## Product Purpose

Tallow is a dark-fantasy / dark-academia roguelike. You are an acolyte who goes under the church floor, finds the stolen Vigil Candle in the court of Beelzebub, and carries it back to the altar. A winning run takes about an hour. The web page exists so anyone can play it in a browser; the terminal download remains the primary release.

## Positioning

The candle is both your light and your clock: every turn burns tallow, and tallow comes from what you kill. Light lets you see and fight; dark hides you but feeds dread. Knowledge carries over between runs (the journal), power never does.

## Operating Context

- Keyboard only. No mouse play, no touch; phones cannot play it.
- The game needs exactly a 100×30 cell screen.
- Saves and the journal live in the browser's localStorage: they stay in that browser and vanish if site data is cleared.
- On itch.io the iframe only receives keys after the player clicks into it.

## Capabilities and Constraints

- Terminal builds for Linux, Windows and macOS: https://github.com/markomoth/tallow/releases
- Source: https://github.com/markomoth/tallow
- Same seed and same keys always play out the same way.

## Brand Commitments

- Name: Tallow. Tagline in use: "Something under the church has taken the candle."
- The game's own palette (`crates/tallow-tui/src/render/palette.rs`): near-black void, warm amber reserved for the acolyte and fire, bone-coloured text, cold blue-grey for memory.
- No D&D or franchise vocabulary (no mana, potions, scrolls, fireballs, d20).
- The web page stays sober and in-world: no emoji, no generic landing-page devices (hero banners, feature cards), no fake retro-CRT effects (scanlines, glow, curvature).

## Evidence on Hand

- Screenshots: `docs/screenshots/` (title, Collegium floor).
- README copy and key table (`README.md`).
- No reviews, player counts or press exist; none may be invented.

## Product Principles

1. The game leads. Everything else on the page is there when needed and never competes with the screen.
2. Fair dark: the world is cruel, the rules are not.
3. Start now: no menus or creation before play.
4. Knowledge is loot; nothing that adds power carries over.

## Accessibility & Inclusion

The terminal build has `--simple` mode (16 standard colours, no animation) for low-colour terminals, light themes and screen readers. The web page should at least state controls in text and respect reduced motion.
