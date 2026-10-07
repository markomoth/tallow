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

**Matrix (M7):** the Dreaming attack the Swarm and the Taken; the Swarm attack the Taken; the Remnant attack all three; the Taken only want you. A creature goes for a hated creature it can see within 5 tiles when that one is nearer than you (or it can't see you). Struck creatures fight back. Turncoat makes a creature and its own kind enemies. The Taken and the Remnant open doors; the Dreaming and the Swarm can't. Look shows each creature's faction and whom it attacks.

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

Mini-bosses always drop a **Leaving** (artifact, see §8; from M9). Each waits by its floor's stair down (`boss_floor` in `monsters.ron`). Bosses resist Binding, can't be freed or banished, and stay out of faction fights: they only want you.

**Biomes in the generator (M8):** the Crypts get chapels (pews) and rotten boards; the Collegium libraries (half its big rooms) and rotten boards; the Drowned Stacks flood half their rooms (shallow water, deep pools in big rooms) and keep a few libraries; the Rot Court and the Throne get pews and old bodies lying about (3–5, they never rot, and the Rot Court's creatures raise them). The Sexton's crypt has 4 old bodies near his stair. Each biome tints its stone.

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

### As built (M10)

- **The Throne (floor 12):** one great hall with two rows of pillars, six cold braziers, five old bodies and the Vigil Candle on the dais to the north. You come in from a shut door to the south. Nothing else lives there.
- **Phase 1, the Court** ("Prince of Flies", 24 health): a coat of 12 flies. While it lasts, blows only strip it (1 + a third of the damage). It thins by 2 a turn while he stands in brazier light and by 6 in fire, and regrows by 1 every 4 turns only in true darkness. He calls flies in the dark. The HUD shows the swarm.
- **Phase 2, the Possession:** when the Court body dies he leaves it; flies gather over the nearest body (announced), and next turn it stands up as a possessed body (14 health). Kill it and he leaps again. A Binding rite or Exorcise on a possessed body pins him there, so its death ends the phase; so does running out of bodies (render or burn them).
- **Phase 3, the Lord** (60 health, slow): every blow is a heavy blow marked a turn ahead on a cross of five tiles (the target and the four beside it). Step diagonally out of it.
- **The candle:** guarded until the Lord falls. Taken, it is endless light (radius 7, needs no tallow, water and lantern-eaters can't put it out) and opens the stair up.
- **The ascent (the Unravelling):** 4 re-rolled floors (generated from the run seed and the ascent floor, built like floors 10, 7, 4 and 2, with half the creatures, tinted violet). You arrive on a stair down; the way on is a stair up. The way down is closed.
- **The Following:** comes up the stair 30, 25, 22 and 18 turns after you arrive (HUD countdown, a warning 10 turns ahead). It cannot be killed (5–8 a hit, your speed). It can't open doors but chews through shut ones in 3 actions (sealed ones in 8), won't cross brazier light or holy ground, and ignores factions. Leaving the floor leaves it behind. Decided: the ascent shows a turn counter, not only log cues.
- **The church:** a lit nave with pews and the altar at the far end. Walk into the altar with the candle to win. Epilogue lines follow the Taken you exorcised, your dread at the end, and whether you learned 8+ rites.

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

Every corpse offers three choices (stand on it, press `s`):
- **Study** (10 + 2 × threat turns, at most 20): the first study of a kind reveals all its tricks in Look and gives 10 Insight. Some kinds teach a rite (`teaches` in `monsters.ron`: parishioner → Exorcise, pallbearer → Kneel, gnawer → Unsee, Proctor → Sanctify). A kind with nothing left to teach is refused at once, costing nothing. Later: journal entry.
- **Render** (10 turns): tallow, 4 × the creature's health (12–90).
- **Leave it:** at 70 turns it visibly swells (log + Look), at 100 it rots. Bodies of creatures with 8+ health hatch a fly swarm; smaller ones just go. Fire also removes it (M7).
- Both tasks are refused with a hostile in view and stop the moment anything happens. Progress stays on the body, so you can come back to it.
- The Dreaming and swarms leave no body.

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
| Binding | Casting control rites (each cast trains its school by the dread it cost, at least 4) |
| Communion | Casting leech / transfer rites |
| Veil | Casting concealment / illusion rites |
| Warding | Casting protection / banishing rites |
| Endurance | Taking hits while armored |

- Ranks 0–10. Rank n needs `k × n²` experience in total (`k` is 6–10 per skill, in `assets/skills.ron`).
- **Meaningful use only:** experience is the damage you deal to real creatures, never more than they had left to lose. Monsters are finite and phantoms teach nothing, so there is nothing to grind. Endurance is damage taken while wearing a vestment.
- Each rank: +2 accuracy with that skill (Endurance: +1 defense instead). The `@` sheet shows every skill's progress and what unlocks next.
- Rite skills (Binding, Communion, Veil, Warding) train from casting. Their ranks add no accuracy; their techniques make rites stronger or cheaper.

| Technique | Rank 3 | Rank 6 |
|---|---|---|
| Blades: **Riposte** | When a creature misses you in melee, strike back at once (50%) | 100% |
| Bludgeons: **Stagger** | A hit can cost the creature its next action (25%) | 40% |
| Reach: **Long Reach** | Moving toward a visible creature two tiles away in a straight line strikes it instead | +10 accuracy |
| Missiles: **Pin** | A hit can hold the creature in place for 2 actions (30%) | 50%, 3 actions |
| Endurance: **Brace** | Heavy blows that land do 25% less | 40% less |
| Binding: **Deep Rites** | Binding rites +50% potency | +100% |
| Communion: **Deep Rites** | Communion rites +50% potency | +100% |
| Veil: **Quiet Rites** | Veil rites cost 25% less dread | 50% less |
| Warding: **Quiet Rites** / **Deep Rites** | Warding rites cost 25% less dread | and +50% potency |

### Combat basics

- **Hit chance** = attacker accuracy − defender defense, clamped to 5–95%. Look shows the exact numbers.
- **Speed:** 10 is one action per turn. Energy-based: faster creatures act more often.
- **Health:** the acolyte starts with 24 and regains 1 every 12 turns. The candle clock keeps resting honest.
- **Telegraphs:** a raised heavy blow marks its target tile in pulsing red and always waits for your next action.
- **Sight is symmetric:** anything in your line of sight can see your candle. Monsters in the dark can notice you before you see them; the log always says so ("Something in the dark has noticed your light.").
- **First sight** of each creature kind prints its description, so you learn its trick before it matters.
- **Runs** are refused while anything hostile is in view.

### Character level

- **Insight** (the in-game word for XP) comes from: first sight of a creature kind (8), each new floor (12), kills (6 × the creature's threat; a Manifestation 20; kills by your thrall count), learning a tincture (5), first study of a kind (10), learning a rite (10), reading a text with nothing new (6), freeing one of the Taken (10).
- Level `L` → `L+1` at `15 × L × (L+1)` total Insight (30, 90, 180, 300, …). A diving bot ends runs around level 5–6.
- Each level: +3 health, then a **draft of 3 boons** pops up. Press 1, 2 or 3. Choosing takes no time.
- Boons are either **passive** (Steady Hand: +5 accuracy; Slow Wick: the candle burns a quarter slower; Night Eyes: feel two tiles in the dark; Hearth-Kin; Broad Back; …) or **trigger × reward** ("When you kill with bludgeons, gain 12 tallow"; "When you snuff your candle, the nearest hunter loses your trail"). Rare triggers pay more.
- The draft is weighted toward skills you actually use, never repeats a unique boon, and never offers a dead pick: no blade boons before you've used a blade, no snuff boons before you've snuffed, and so on.
- Rite boons: Familiar Words (rites cost a quarter less dread; offered once you know a rite), Answered Prayer ("when you cast a rite, …"; after your first cast), Scholar's Reward ("when you finish studying a body, …"; after your first study).
- Later: "Thrown flasks shatter in a cross".

### Rites (magic)

Situational, manipulative, useful to every build. Cost dread. Learned by **study**: hymnal pages, heretic notes, studied corpses.

| School | Example rites |
|---|---|
| **Binding** | *Compel* (control one creature for N turns), *Kneel* (root), *Turncoat* (switch a creature's faction) |
| **Communion** | *Leech* (drain life), *Transference* (give your wounds or dread to another), *Borrowed Eyes* (see through a creature) |
| **Veil** | *Hush* (silence an area), *Unsee* (one creature forgets you), *False Flame* (a decoy light that lures) |
| **Warding** | *Sanctify* (ground nightmares can't cross), *Exorcise* (free a Taken; they flee as a villager), *Seal* (bar a door) |

Target for v1: 16 rites (4 per school). No damage-only spells. Every rite changes a situation.

**How rites work (M6):**
- `z` lists known rites with their current numbers; pick a letter, then a target if the rite needs one. Rites are defined in `assets/rites.ron`.
- **Cost:** the dread listed, minus the school's Quiet Rites and the Familiar Words boon. The list warns when a cast would take dread to 100. A failed cast (no target, out of range, wrong kind) costs nothing.
- **Potency:** calm 1.0×, uneasy 1.25×, frayed or worse 1.5×, times the school's Deep Rites. Potency scales durations and amounts.
- **Learning:** texts (`?`) teach a random unknown rite of their school; studying bodies teaches their kind's rites. Floor 1 always has a text. Rites last the run only.
- **Bosses** resist Binding (durations ÷ 3) and can't be freed by Exorcise (it burns them instead). Your Manifestation ignores Compel, Unsee and Transference.

| Rite | School | Dread | Range | Effect (1.0×) |
|---|---|---|---|---|
| Compel | Binding | 15 | 6 | The creature fights the nearest creature it can see and follows you, for 12 of its actions. You swap places with it. It never strikes you; when it wakes it hunts you. |
| Kneel | Binding | 8 | 6 | Can't move for 5 actions (can still strike). |
| Leech | Communion | 10 | 4 | Drain 5 health from it into you. |
| Transference | Communion | 0 | 5 | Your dread −20; it flees for 8 actions. Each creature can carry your dread only once. |
| Borrowed Eyes | Communion | 6 | 8 | 30 turns: you also see what it sees within 7 tiles, lit or not. |
| Unsee | Veil | 6 | 6 | It forgets you and can't notice you for 10 actions. |
| False Flame | Veil | 8 | 8 | A decoy light on open ground for 20 turns. Creatures within 10 of it go to it and ignore you unless you're within 2. Lantern-eaters eat it. |
| Shroud | Veil | 8 | self | 15 turns: your lit candle doesn't give you away; only adjacent creatures notice you. |
| Sanctify | Warding | 12 | self | Ground within 2 tiles is holy for 30 turns: the Dreaming can't enter (and leave it if caught on it), the Taken lose an action when they step on it. |
| Exorcise | Warding | 15 | 3 | One of the Taken wakes and runs home (gone, counted for the epilogue). Bosses take 8 instead. |

| Turncoat | Binding | 12 | 6 | It and its own kind become enemies for good. It still hates you. Not on bosses. |
| Beckon | Binding | 6 | 7 | It walks to you for up to 6 steps, calm and heedless of fire and oil, then wakes. |
| Exchange | Communion | 8 | 6 | You and it trade places. |
| Hush | Veil | 4 | self | 25 turns: your fights make no noise. |
| Seal | Warding | 6 | 5 | A door shuts and holds for 40 turns; nothing but you can open it (opening breaks the seal). It still burns. |
| Banish | Warding | 10 | 6 | A creature of the Dreaming goes far away on this floor (25+ steps, out of sight) and forgets you. Not on bosses. |

All 16 v1 rites exist as of M7.

**Creatures fighting each other (M6):** a creature struck by another turns on it until one dies or they're 10+ tiles apart. M7 adds the faction hostility matrix on top.

---

## 6. Items

### Weight-based inventory

- Every item has weight. The HUD shows a Load bar; the pack screen shows exact numbers.
  - **Unburdened:** up to 25.0. Normal.
  - **Burdened:** over 25.0. Moving costs 1.5× time. (Tallow burns per turn, so this matters.)
  - **Overloaded:** over 38.0. Cannot move until you drop something.
- Tallow weighs 1.0 per 100 turns of light. A full candle is a real weight; it lightens as it burns.
- **Starting kit:** iron candlestick (in hand), cassock (worn), one mending tincture. Load 14.3.
- The pack holds up to 26 kinds of thing (one letter each); stackables share a slot.
- Each floor has 3–5 loose items, chosen by depth from `assets/items.ron`.

### Throwing and shooting

- `t` picks something throwable from the pack, then a target. `f` shoots the readied sling or crossbow.
- The target cursor starts on the nearest creature; the shaded line is exactly where the projectile will fly (it stops at walls, range, and the first creature you can see).
- Hand range is 7; a sling reaches 8, a crossbow 10. Stones and bolts land where they stop and can be picked up again.
- **Holy water** shatters on landing: it burns the Dreaming (5–9), makes the Taken flinch (1–3), and does nothing to vermin.
- Projectiles pass straight through phantoms, which come apart.

### Categories

| Category | Examples |
|---|---|
| Melee | Sickle, cleaver, censer-on-chain, iron candlestick, boathook, verger's staff |
| Ranged | Sling (stones also probe anomalies, see §8), crossbow |
| Throwables | Throwing knives, holy water (also leaves the ground holy for 20 turns), fire flask (bursts into flame in a cross where it breaks, never on your own tile), lamp oil (spills oil around where it breaks), handbell (rings where it lands, can be picked up again); later smoke pot, chalk (draws a ward line) |
| Vestments | Cassock, gambeson, sexton's leathers, choir mail (heavy) |
| Tinctures | Mending (+health), steadying (−dread), seeing (reveals the floor around you) |
| Texts | Hymnal page (Warding), heretic's note (Binding), anatomist's notes (Communion), lecture fragment (Veil). Read with `a` in the pack: 3 quiet turns, then a rite. Used up. |
| Tools | Handbell (ring it with `a`, or throw it), later crowbar (pry, break), incense |

### Identification: known category, learn by use

- You always see the category: "mending tincture (untried)".
- Each tincture kind rolls, per run, a strength (weak, middling, strong) and maybe a side effect.
- Drinking one reveals both for that kind for the rest of the run; the name then shows it ("strong mending tincture (bitter)").
- The only side effect so far is **bitter**: +6 dread. Side effects are never lethal and never permanent.

---

## 7. Environment

The room is a weapon. All of these must be readable on screen.

- **Light:** braziers, wall candles, fallen candles. Light hurts the Dreaming and reveals you.
- **Fire:** spreads over books, pews, rugs, oil, dry bone. Burns the Swarm. Burns you too.
- **Oil:** spill and ignite. Slippery.
- **Holy water:** consecrates tiles for N turns. Nightmares avoid, Taken flinch.
- **Bells:** wall bells and handbells make noise; noise draws nearby creatures. Lure factions into each other.
- **Doors:** open, close, bar, Seal. Wooden ones burn.
- **Rotten floors** (`,` brown): stepping on them needs a second step; then they give way and you fall to the next floor (2–4 damage, never fatal), landing anywhere on open floor. Only placed where floor surrounds them, so no path needs them. Crypts and Collegium only. Creatures never step on them.
- **Water** (`~`): shallow costs 1.5× time to wade, and nothing burns or spills there. Deep (dark `~`) costs 2×, needs a second step to wade into, puts your candle out, and you can't relight it while standing in it. Only swimmers (drowned deacons, the Drowned Choir) enter deep water by choice.

**How it works (M7):**
- **Doors** (`'` open, `+` shut, gold `+` sealed): walk into a shut door to open it (a turn). `C` shuts every open door beside you with nothing in the doorway. 40% of doors start shut; shut doors block sight and light.
- **Fire** (`^` flickering): fuel by tile, shelves 10 turns, pews and doors 6, oil 5, bare floor 3 (a flask only). Each turn fire may catch a neighbor: shelves and pews 30%, doors 15%, oil 70%. What burns away becomes open floor. Fire lights its surroundings (radius 2), burns bodies, and does 2–4 a turn to whatever stands in it (the Swarm ×2; the Swarm also won't step next to it). Creatures won't walk into fire unless Beckoned. Walking into fire yourself needs a second step in the same direction.
- **Oil:** dark yellow floor. 25% of steps onto it slip and lose an action (you and creatures). It catches fire fast.
- **Noise:** a bell rope (`|` on a wall, pull it by walking into it) is heard 18 steps away; a handbell 12; a fight 5 (not under Hush). Creatures that hear it come to look. The Taken within 4 steps of a bell cower for 4 actions instead. Relentless hunters and creatures already fighting ignore noise.
- **Braziers:** 40% start cold (grey `&`). Walk into one with your candle lit to light it. Braziers keep lantern-eaters off and ease dread.
- **Furniture:** Collegium floors and deeper (4+) get libraries: rows of shelves (`#` in brown) with floor all around each row, so they never cut a floor in two. Crypt floors get chapels with pews (`=`, walkable). Most floors have one bell rope.

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

### As built (M9)

- **Seep rooms:** one small room on 40% of floors 2–10, and always on floor 11. Its floor shimmers; the first sight of it logs a warning; Look says to throw something through. It holds a Leaving in the middle and 3–6 invisible anomalies clustered around it. 3–5 stones lie just outside, on the side nearest the arrival stair.
- **Anomalies** (revealed `:` violet): a heat well burns 4–7; a snare does 2–3 and holds you 3 turns; a time pocket loses you 10 turns; a swap point throws you elsewhere on the floor. An anomaly's own damage never takes your last health. Anything thrown or fired through one stops there and reveals it (a swap point throws it somewhere else in the room). Creatures never step on anomalies. Walking into a revealed one needs a second step.
- **Leavings** (`*`, colored by tier: green mild, amber strange, red deadly). Rules come from 6 triggers (on use, every N turns while carried, on kill, when hurt, when dread turns frayed, when your candle goes out) × 14 effects (swap with the nearest creature, stop time for everything in view, pull, push, see in the dark, duplicate a stack, mend, calm, kindle tallow, blink, fire all around, reveal the floor, holy ground, banish the nearest of the Dreaming) × 5 prices (health, dread, tallow, weight, or the attention of a faction). Generated names come from word lists in `assets/leavings.ron`.
- **Telegraphs:** the tier and a tell are shown from the start; the first Leaving you take comes with a warning; the rule is learned when it first wakes. A price that would kill you warns first ("Drop it now"): an involuntary one takes it next turn unless dropped; one you use takes it on the next use. Effects with nothing to act on stay quiet and cost nothing.
- **Named:** the Sexton's spade (when you kill: mend 4, 4 dread), the Provost's ring (use: everything in view loses 3 actions, 4 health), the Choir's tuning fork (candle goes out: see in the dark 40 turns, 6 dread), and a wick that remembers (floor 11's seep: every 60 turns, +60 tallow, 3 health). Bosses drop theirs where they die.
- Pack screen (`i`): Leavings are listed under their own heading; press a number to open one (`a` use, `d` drop).

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

**As built (M8):** all of the above exist, plus two raised creatures: the risen husk (Dreaming; what the Sexton and courtiers raise) and maggot spawn (what the Mother of Maggots makes of bodies). Tricks, all data in `monsters.ron` and shown by Look once seen:

| Trick | Who | Rule |
|---|---|---|
| Blinds | Inkling | A hit shrinks your light to 2 for 12 turns. |
| Chants | Feverish Scholar | Announces a chant; next action it lands (3–5 and +3 dread) if it can still see you. Break line of sight to stop it. |
| Drags | Drowned Deacon | A hit pulls you one step toward deep water within 8. |
| Sings | Bone Choir, Drowned Choir | While it can see you, dread +1.5 (+1 for the Choir) per action. |
| Bursts | Bloatfly | Dies into a fly swarm. |
| Summons | Fly Herald | In darkness, calls a fly swarm (3 at most) every 5 actions. |
| Raises | Sexton, Courtier, Mother of Maggots | Turns a body within range into a risen husk (or maggot spawn). Render or burn bodies to deny them. |
| Rewrites | Provost | Every 8 actions: locks every door within 8 for 15 turns (you can't open them either, but they burn), or sets a shelf alight. |
| Chorus | Drowned Choir | Three bodies, one shared life (45). They crowd you in the open; fight them in a doorway. |
| Swims | Deacon, Choir | Enters deep water. |

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

- **Glyphs:** ASCII. `@` you, `#` wall, `.` floor, `+` door, `>` `<` stairs, `&` brazier, `,` tallow, letters for creatures, `!` tinctures, `?` texts, `*` Leavings, `~` water, `"` vestments, `/` melee weapons, `)` ranged weapons, `(` throwables and ammunition.
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
| `<` `>` | Stairs (`<` only works on the way back up, with the Vigil Candle) |
| `g` | Pick up |
| `i` | Pack: pick a letter, then `a` drink, `e` equip / take off, `t` throw, `d` drop |
| `z` | Cast rite |
| `f` | Fire the readied sling or crossbow |
| `t` | Throw (choose item, then target; Enter lets fly) |
| `x` | Look (cursor; Tab cycles creatures, Esc leaves) |
| `s` | Study / render corpse |
| `c` | Snuff / relight candle |
| `C` | Shut the open doors beside you |
| `R` | Rest until healed (or calm, by a brazier) |
| `o` | Auto-explore (stops when anything new is in view; skips fire, deep water, rotten boards, seep rooms) |
| `@` | Character sheet: skills, techniques, boons |
| `M` | Journal (`J` is taken by running south) |
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

**As built (M11):**
- A save is the seed plus every command given (`save.ron`, RON). The core is deterministic, so loading replays the commands, log and all. This replaced the planned `postcard` world snapshot: nothing to keep in sync as the world grows. Bump `save::VERSION` whenever a change would make old saves replay differently.
- Quitting (`q`, Ctrl-C) mid-run saves; launching without `--seed` resumes and deletes the save. Dead or won runs leave no save. Dev flags never read or write saves or the journal.
- Files live in the platform data directory (`directories`), or `$TALLOW_HOME` if set.
- **Journal** (`M`; `J` runs south): runs, deaths, wins, deepest floor; every creature met (marked if studied), every rite ever learned, every Leaving held, and 9 pages of the town's history unlocked by reaching each biome, killing each mini-boss and bringing the candle home. Lore only.

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

**As built (M11):** `cargo run --release -p tallow-sim -- 500` plays whole runs (descent, Beelzebub, candle, ascent, altar) and reports where they end, softlocks (a run stuck on one floor for 5000 commands, or not over after 30 000) and, for the first 300 seeds, any floor whose way onward can't be reached without deep water or rotten boards. `SIM_TRACE=<seed>` prints the end of one run. Last result (500 seeds): 0 softlocks, 0 unreachable stairs, the bot wins 8% and its deaths cluster at the four bosses (floor 3 24%, 6 13%, 9 17%, 12 11%); it never casts rites or uses items, so human runs should go better. Fixes it drove: tallow you can't carry spills instead of pinning you overloaded; seep rooms never sit on the path to the stairs and hold nothing but their Leaving; deep water is drained if it would be the only way somewhere; the Lord of Flies heaves himself up between blows (an opening; still no unmarked attack).

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
| M4 ✅ | Items & weight | Pick up, equip, throw, get Burdened, learn a tincture by drinking it | 3–4 h |
| M5 ✅ | Skills & levels | See Blades rank up from use, pick 1 of 3 boons on level up | 2–3 h |
| M6 ✅ | Rites & study | Study a corpse, learn Compel from a page, mind-control a Proctor into a Taken | 3–4 h |
| M7 ✅ | Factions & environment | Ring a bell to pull a swarm into a nightmare; set a library on fire | 4–5 h |

### Phase C: The full run (~3–4 days)

| # | Milestone | Done when you can… | Est. |
|---|---|---|---|
| M8 ✅ | Biomes & content | Descend all 12 floors through 4 biomes, beat 3 mini-bosses | 5–6 h |
| M9 ✅ | Leavings | Throw a stone into a Seep room, find an anomaly, take a Leaving, survive its warning | 3–4 h |
| M10 ✅ | Beelzebub & ascent | Beat 3 phases, take the candle, outrun the Following, win at the altar | 5–6 h |
| M11 ✅ | Polish & balance | Save/quit/resume, Journal persists, help screen, sim shows ~0 softlocks | 4–6 h |

Total: roughly 40–55 hours of build + playtest time.

---

## 14. Open questions (decide when we get there)

- Final names: town, church, Collegium, the Following.
- ~~In-game words for level and XP~~ Decided in M5: "Level" and "Insight".
- Exact numbers: tallow per floor, dread rates, skill XP curve. Tune with `tallow-sim` in M11.
- ~~Whether the ascent should show a turn counter for the Following or only audio-style log cues.~~ Decided in M10: a HUD counter plus a warning 10 turns ahead.
