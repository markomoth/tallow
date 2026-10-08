---
name: Tallow
description: The browser host page for Tallow, set as an order of service around the game's frameless screen.
colors:
  void: "#090808"
  bone: "#ece0c6"
  ash: "#968472"
  amber: "#ffd280"
  flame: "#ff9c40"
  rule: "rgb(150 132 114 / 0.2)"
  veil: "rgb(9 8 8 / 0.9)"
typography:
  display:
    fontFamily: "IM Fell English SC, IM Fell English, Iowan Old Style, Palatino, Georgia, serif"
    fontSize: "30px"
    fontWeight: 400
    lineHeight: 1
    letterSpacing: "0.32em"
  headline:
    fontFamily: "IM Fell English, Iowan Old Style, Palatino, Georgia, serif"
    fontSize: "26px"
    fontWeight: 400
    lineHeight: 1.15
  title:
    fontFamily: "IM Fell English SC, IM Fell English, Iowan Old Style, Palatino, Georgia, serif"
    fontSize: "22px"
    fontWeight: 400
    letterSpacing: "0.14em"
  body:
    fontFamily: "IM Fell English, Iowan Old Style, Palatino, Georgia, serif"
    fontSize: "17px"
    fontWeight: 400
    lineHeight: 1.4
  body-small:
    fontFamily: "IM Fell English, Iowan Old Style, Palatino, Georgia, serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.35
  label:
    fontFamily: "IM Fell English, Iowan Old Style, Palatino, Georgia, serif"
    fontSize: "15px"
    fontWeight: 400
  key:
    fontFamily: "JetBrains Mono, Menlo, Consolas, monospace"
    fontSize: "0.92em"
    fontWeight: 400
    letterSpacing: "0"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
  phrase: "1.6em"
components:
  key:
    textColor: "{colors.amber}"
    typography: "{typography.key}"
  text-action:
    textColor: "{colors.bone}"
    typography: "{typography.body-small}"
    padding: "0"
  text-action-hover:
    textColor: "{colors.amber}"
  all-keys-panel:
    backgroundColor: "{colors.void}"
    textColor: "{colors.bone}"
    padding: "28px 32px 24px"
    width: "min(640px, calc(100vw - 32px))"
  veil-call:
    textColor: "{colors.bone}"
    typography: "{typography.headline}"
  colophon:
    textColor: "{colors.ash}"
    typography: "{typography.label}"
---

# Design System: Tallow

## Overview

**Creative North Star: "The Order of Service"**

The page is the game's own dark. The 100×30 game screen is a canvas drawn by the game and is not part of this system; the page is built to sit flush on that canvas's void, so the screen has no frame, no bezel and no card around it. Everything the page adds is a few lines of words set above and below the screen like the rubrics of a printed order of service: what to do in amber, what it means in bone, the small print in ash.

The world is sober, warm and nearly empty. One seventeenth-century face (IM Fell English, Bishop Fell's types, church and college in one) carries every word; the game's monospace appears only where a literal key is named. Density is low on purpose: the screen takes most of the height, and the words above and below are aligned to its measured edges. Light is the only spectacle. A dimming veil covers the screen until the first click or key, a small drawn candle stands at its centre, and lighting it opens the dark outward from the flame.

The product rejects the category default for a game embed: no hero, no feature cards, no big Play button, no CRT costume (scanlines, glow, curvature), no emoji.

**Key Characteristics:**
- Void ground continuous with the game screen; no frame around the canvas.
- One serif voice (IM Fell English, roman, italic and small caps) for all words; monospace for literal keys only.
- Amber marks what to do; bone carries meaning; ash carries the small print.
- Hairline rules in ash, never boxes or bordered panels.
- Square everywhere; zero radius.
- Motion is light itself: a flame that flickers and a dark that opens.

## Colors

A near-black ground with warm, candlelit text, taken from the game's own palette (`crates/tallow-tui/src/render/palette.rs`).

### Primary
- **Rubric Amber** (`amber`): the player's colour in the game, and on the page the colour of instruction. Every literal key name, the wordmark, the popover title, the focus ring, link and text-action hover, and (at 45% alpha) the underline of every link and text action. At 28% alpha it is the text selection.

### Tertiary
- **Candle Flame** (`flame`): the flame at the base of the candle drawing and the favicon. Nothing else. It never colours text or interface.

### Neutral
- **Church Void** (`void`): the page ground, the popover ground, and the colour the game screen draws on. The page and the screen must read as one surface.
- **Bone** (`bone`): primary text, the veil's call, the first line of the tagline, the right column of the key table.
- **Ash** (`ash`): secondary text: the descriptive line of the tagline, the veil note, the colophon, the "or/and" joiners in the key table, the Close action, the popover scrollbar thumb, and the loading-state call.
- **Hairline** (`rule`): ash at 20%, the single rule above the colophon. Key-table row dividers use ash at 10%.
- **Veil Dark** (`veil`): void at 90%, the dimming over the screen before it is lit; deepened to 96% in an ellipse behind the words, and to a flat 94% in the touch and failed states. The popover backdrop is void at 55%.

The candle drawing also uses one-off illustration tints (wax highlight and shadow, wick, inner-flame cream); they belong to the drawing, not to the palette.

### Named Rules
**The Rubric Rule.** Amber means "this is what you do". It goes on keys, the name, focus and hover, and never on body prose or decoration.

**The One Flame Rule.** Flame orange appears only on the candle. If something else on the page is that colour, it is wrong.

**The Flush Void Rule.** The page ground is exactly the game's void (`void`). No off-black, no gradient, no texture behind the screen.

## Typography

**Display Font:** IM Fell English SC (with IM Fell English, Iowan Old Style, Palatino, Georgia)
**Body Font:** IM Fell English, roman and italic (with Iowan Old Style, Palatino, Georgia)
**Label/Mono Font:** JetBrains Mono (with Menlo, Consolas), for literal keys only

**Character:** A cut of early English printing type, slightly irregular, set small and quiet, against a clean monospace that is the game's own screen face. The serif speaks; the mono names the keys you press. All fonts are self-hosted under the OFL in `crates/tallow-web/fonts/`.

### Hierarchy
- **Display** (Fell SC 400, 30px, line-height 1, tracked 0.32em): the wordmark only, in amber. The trailing tracking is cancelled with a matching negative right margin.
- **Headline** (Fell italic 400, 26px, line-height 1.15, balanced): the veil's call, "Click to light the candle".
- **Title** (Fell SC 400, 22px, tracked 0.14em): the popover title, in amber.
- **Body** (Fell 400, 17px, line-height 1.4; 16px below 720px): the base page text.
- **Body small** (Fell 400, 16px, line-height 1.35): the tagline (max 76ch), the key line, the veil note (max 44ch), the key table.
- **Label** (Fell 400, 15px): the colophon.
- **Key** (JetBrains Mono 400, 0.92em of its line, amber, no tracking): every literal key, inline with Fell text.

### Named Rules
**The Literal Key Rule.** Monospace appears only for a key the player presses. Words like "direction", "or" and "and" between keys stay in Fell (joiners in ash italic at 0.9em).

**The Italic Is The Voice Rule.** Italic carries the voice that speaks to the player: the veil's call, the tagline's first line, and text actions. It is not used for emphasis inside prose.

## Layout

Three rows fill the viewport height (`100dvh`): the masthead, the screen, the rubric. The screen row takes everything left over and the game canvas is centred in it and scaled down (never up) to fit. A script measures the drawn screen and sets its width as a variable, so the masthead and rubric are exactly as wide as the screen and their text aligns with its edges; the page has no other container width.

The masthead is a baseline-aligned row: wordmark at the left, tagline right-aligned at the right. The rubric is a short stack: one wrapping line of essential keys (phrases separated by a 1.6em gap, never broken inside a phrase) with the "All keys" action pushed to the right, then the colophon under a hairline with its links pushed right.

Vertical padding around the masthead and rubric scales with viewport height (about 10 to 28px). Spacing is small and close: 4px between wrapped lines, 6 to 8px between rubric rows, 14px between the candle and its words, 16px side gutter around the screen and on narrow screens.

Below 720px the masthead stacks (wordmark above a left-aligned tagline), right-pushed actions return to the left, and the gutter becomes 16px. On touch-only devices the key line is hidden and the veil states that the game needs a keyboard.

## Elevation & Depth

The page is flat. Depth is darkness, not lift: the veil is a translucent layer of the same void over the screen, darker in an ellipse behind its words, and it is opened by a radial mask centred just above the flame. The single exception is the All-keys popover, a void panel lifted off the dimmed page by one wide, soft shadow and a 55% void backdrop; it has no border.

### Shadow Vocabulary
- **Popover lift** (`box-shadow: 0 24px 80px rgb(0 0 0 / 0.7)`): the All-keys popover only. Diffuse, centred, never offset sideways.

### Named Rules
**The Dark Is The Depth Rule.** Hierarchy over the screen is made by dimming and revealing the void, not by panels and shadows. A second shadow on the page needs a reason as strong as a modal.

## Shapes

Square and borderless. Nothing has a radius. The only lines are hairlines: the 1px ash rule above the colophon and the fainter row dividers of the key table; the focus ring is a 1px amber outline offset 4px (inset 6px on the full-screen veil). Buttons are text, not shapes. The one drawn form is the candle (an inline SVG, 30×60, viewBox 24×48): a short bone taper with two wax drips, a wick, and a teardrop flame that grows from the wick and flickers. It is the page's sole illustration and its sole icon; the favicon is the same flame on void.

### Named Rules
**The Hairline Not Box Rule.** Separate with a 1px ash hairline (20% or less), never with a bordered or filled box. The popover is the one panel, and it is void on void.

## Components

### Text actions (links and buttons)
Quiet, typographic, printed. There are no filled or outlined buttons.
- **Shape:** none; zero padding, no border, no background.
- **Default:** bone text with an amber underline at 45% alpha (1px, offset 0.22em). Buttons that act on the page ("All keys") are set in italic; links that leave the page ("Play in your terminal", "Source") are roman.
- **Hover / Focus:** text and underline turn full amber over 160ms ease-out; keyboard focus shows the 1px amber ring.

### Keys
- **Style:** JetBrains Mono at 0.92em in amber, inline in a Fell line, no keycap box, no background. Adjacent keys are separated by a space; a key and its meaning sit together and never wrap apart.

### All-keys popover
- **Corner Style:** square.
- **Background:** void, over a 55% void backdrop.
- **Shadow Strategy:** the popover lift (see Elevation & Depth).
- **Border:** none.
- **Internal Padding:** 28px top, 32px sides, 24px bottom; width up to 640px, scrolls within the viewport with a thin ash-on-void scrollbar.
- **Content:** an amber small-caps title, then a two-column table: keys in the left column (34%), meaning in bone at right, rows divided by ash hairlines at 10%. A Close action in ash italic sits at the bottom right and turns amber on hover.

### The veil (signature component)
A full-screen button over the game screen until it is lit. It shows the candle, one call in Fell italic and one ash note, one message per state (loading, ready, away, failed, touch). In the loading state the call is ash; the flame is unlit (only a faint ember on the wick). On click or any key, the ember goes out, the flame grows from the wick (260ms, `cubic-bezier(0.16, 1, 0.3, 1)`) and flickers (2.4s loop); the words fade (220ms after 180ms); the dark opens outward from the flame by an expanding radial mask (1500ms, `cubic-bezier(0.4, 0, 0.3, 1)`, after 420ms), and the candle fades last. When focus leaves the page after play has started, the veil returns in the "away" state with the flame already lit. With reduced motion, the veil simply fades in 200ms and the flame does not flicker.

## Do's and Don'ts

### Do:
- **Do** set the page ground to exactly `void` and let the game screen sit on it with no frame.
- **Do** align the words above and below the screen to the screen's measured width.
- **Do** set every literal key in JetBrains Mono amber, and everything else in IM Fell English.
- **Do** make actions text: bone with a 45% amber underline, amber on hover over 160ms.
- **Do** separate with a 1px ash hairline at 20% or less.
- **Do** honour reduced motion: no flicker, no expanding light, a 200ms fade instead.
- **Do** self-host every font and keep its OFL licence beside it.

### Don't:
- **Don't** box the game screen, or put the page inside cards, panels or bordered sections.
- **Don't** add a hero, feature cards or a filled Play button; the veil's call is the only invitation.
- **Don't** use flame orange anywhere but the candle and the favicon.
- **Don't** use amber on body prose or decoration; it means "press this".
- **Don't** add scanlines, glow, screen curvature or other CRT effects.
- **Don't** use emoji or icon glyphs; the candle is the only drawn image.
- **Don't** round corners.
