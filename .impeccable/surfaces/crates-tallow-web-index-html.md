---
version: 1
slug: "crates-tallow-web-index-html"
primary_target: "crates/tallow-web/index.html"
related_targets: []
---

# Surface: web host page (crates/tallow-web/index.html)

Mode: Experience. The visitor is inside the game; the page recedes.
Audience: itch.io strangers and README readers, equally. Job: understand in one line, click, play.
Constraints: keyboard only; 100×30 cell canvas; itch iframe needs a click before keys arrive; saves in localStorage. No emoji, no landing-page devices, no CRT costume, minimal text.

## Direction contract

THESIS: The page is the game's own dark. The canvas sits flush on the same void (#090808), so the screen has no frame; the few words around it are set like the rubrics of an order of service: what to do in amber, what it means in bone. Refuses the category default of a game embed boxed inside a landing page with a hero, cards and a big Play button.

OWN-WORLD: Void #090808 ground; bone #ece0c6 text; ash #968472 secondary; amber #ffd280 rubrics (keys); flame #ff9c40 only on the candle. IM Fell English (Bishop Fell's types, bequeathed to Oxford: church and college in one face) for wordmark, tagline and cover; the game's mono (JetBrains Mono, self-hosted) only for literal keys. Hairline rules in ash at 20% at most. No boxes.

STORY: A stranger reads "Tallow", the tagline and one line of what it is; sees a dimmed church behind a single candle; clicks to light it; plays. A returning player sees their run resume. Anyone who wants more finds all keys, the saves note, the terminal download and source under the screen.

FIRST VIEWPORT: Top: wordmark left, one-line description right, small. Centre, ~85% of height: the 100×30 screen, scaled to fit. Over it until the first click/key: a dimming veil with a small authored SVG candle and "Click to light the candle" in Fell italic; the click spreads light outward from the flame (expanding radial mask) and hands keys to the game. Bottom: one rubric line of essential keys, "All keys" opening a popover table, then saves note · terminal download · source.

FORM: Order-of-service rubric page around a frameless screen; surface scope, precisely specified brief, shaped directly (no concept-seed roll); code-led, no image generation available.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
