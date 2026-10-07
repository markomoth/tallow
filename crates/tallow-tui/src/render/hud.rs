//! The status sidebar: who you are, where you are, what you can see.
//! While looking, it shows what's under the cursor instead.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap};
use tallow_core::candle::{LOW_AT, START_TALLOW};
use tallow_core::item::LIGHT_LOAD;
use tallow_core::{
    Biome, Burden, CandleState, DreadBand, MAX_DEPTH, Mind, MonsterInfo, Point, Tile, Trait, World,
};

use crate::app::Aim;
use crate::names::{item_phrase, item_stats};

use super::palette::{self, rgb};
use crate::app::{App, Mode};
use crate::log::capitalize;

const BAR_WIDTH: usize = 10;
const MAX_IN_VIEW: usize = 3;

pub struct Hud<'a> {
    app: &'a App,
}

impl<'a> Hud<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

pub fn biome_name(biome: Biome) -> &'static str {
    match biome {
        Biome::Crypts => "The Crypts",
        Biome::Collegium => "The Collegium",
        Biome::DrownedStacks => "The Drowned Stacks",
        Biome::RotCourt => "The Rot Court",
        Biome::Throne => "The Throne",
    }
}

fn stage_name(world: &World) -> &'static str {
    match world.stage() {
        tallow_core::Stage::Descent => biome_name(Biome::for_depth(world.depth())),
        tallow_core::Stage::Ascent(_) => "The Unravelling",
        tallow_core::Stage::Church => "The Church of the Low Bell",
    }
}

/// The fight at the bottom, or what's behind you on the way up.
fn pursuit(world: &World) -> Line<'static> {
    use tallow_core::Phase;
    if world.following_present() {
        return Line::styled(
            "The Following is here",
            Style::new()
                .fg(palette::DANGER)
                .add_modifier(Modifier::BOLD),
        );
    }
    if let Some(turns) = world.following_in() {
        return Line::styled(
            format!("The Following: {turns} turns"),
            Style::new().fg(palette::DREAD),
        );
    }
    match world.lord() {
        Some(lord) if world.stage() == tallow_core::Stage::Descent => match lord.phase {
            Phase::Court => Line::styled(
                format!("Beelzebub: the Court · swarm {}", lord.swarm),
                Style::new().fg(palette::DANGER),
            ),
            Phase::Possession => Line::styled(
                if lord.bound {
                    "Beelzebub: bound in a body".to_string()
                } else {
                    "Beelzebub: the Possession".to_string()
                },
                Style::new().fg(palette::DANGER),
            ),
            Phase::Lord => Line::styled("Beelzebub: the Lord", Style::new().fg(palette::DANGER)),
            Phase::Fallen => Line::styled(
                if world.player().vigil {
                    "The candle is yours. Go up."
                } else {
                    "Beelzebub has fallen. Take the candle."
                },
                Style::new().fg(palette::GOOD),
            ),
        },
        _ => Line::default(),
    }
}

fn dim() -> Style {
    Style::new().fg(palette::TEXT_DIM)
}

fn text() -> Style {
    Style::new().fg(palette::TEXT)
}

impl Widget for Hud<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let world = self.app.world();
        let player = world.player();
        let depth = world.depth();

        let mut lines = vec![
            Line::styled(
                "T A L L O W",
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            bar(
                "Health",
                player.health,
                player.max_health,
                palette::HEALTH,
                palette::HEALTH_EMPTY,
            ),
            bar(
                "Candle",
                player.candle.tallow(),
                START_TALLOW,
                palette::CANDLE,
                palette::CANDLE_EMPTY,
            ),
            bar(
                "Dread ",
                player.dread.value(),
                100,
                palette::DREAD,
                palette::DREAD_EMPTY,
            ),
            bar(
                "Load  ",
                world.load() / 10,
                LIGHT_LOAD / 10,
                palette::LOAD,
                palette::LOAD_EMPTY,
            ),
            conditions(world),
            Line::styled(
                match world.stage() {
                    tallow_core::Stage::Descent => {
                        format!("Floor {depth}/{MAX_DEPTH} · Level {}", player.level)
                    }
                    tallow_core::Stage::Ascent(a) => format!(
                        "Ascent {a}/{} · Level {}",
                        tallow_core::throne::ASCENT_FLOORS,
                        player.level
                    ),
                    tallow_core::Stage::Church => format!("The Church · Level {}", player.level),
                },
                text(),
            ),
            Line::styled(stage_name(world), dim()),
            pursuit(world),
            Line::default(),
        ];
        match self.app.mode() {
            Mode::Look { cursor } => lines.extend(look_panel(world, cursor)),
            Mode::Target { aim, cursor } => {
                lines.extend(target_panel(world, aim, cursor, self.app.aim_path().len()));
            }
            Mode::Play
            | Mode::Dead
            | Mode::Won
            | Mode::Help
            | Mode::Journal(_)
            | Mode::Pack { .. }
            | Mode::Draft
            | Mode::Sheet
            | Mode::Corpse
            | Mode::Rites
            | Mode::Leaving(_) => {
                lines.extend(in_view(world));
                lines.extend(keys());
            }
        }

        let block = Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::new().fg(palette::BORDER))
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        block.render(area, buf);

        let [body, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .render(body, buf);
        Line::styled(
            format!("turn {} · seed {}", world.turn(), world.seed()),
            dim(),
        )
        .render(footer, buf);
    }
}

fn bar(label: &'static str, value: u32, full: u32, fill: Color, empty: Color) -> Line<'static> {
    let filled = (value as usize * BAR_WIDTH)
        .div_ceil(full.max(1) as usize)
        .min(BAR_WIDTH);
    Line::from(vec![
        Span::styled(format!("{label} "), dim()),
        Span::styled("█".repeat(filled), Style::new().fg(fill)),
        Span::styled("░".repeat(BAR_WIDTH - filled), Style::new().fg(empty)),
        Span::styled(format!(" {value}"), text()),
    ])
}

/// One line naming anything unusual about the candle and the mind.
fn conditions(world: &World) -> Line<'static> {
    let player = world.player();
    let mut spans = Vec::new();
    let candle = match player.candle.state() {
        CandleState::Lit if player.candle.tallow() < LOW_AT => {
            Some(("candle low", palette::DANGER))
        }
        CandleState::Lit => None,
        CandleState::Guttering => Some(("guttering!", palette::DANGER)),
        CandleState::Snuffed => Some(("snuffed", palette::TEXT_DIM)),
        CandleState::Out => Some(("no light", palette::DANGER)),
    };
    let mind = match player.dread.band() {
        DreadBand::Calm => None,
        DreadBand::Uneasy => Some("uneasy"),
        DreadBand::Frayed => Some("frayed"),
        DreadBand::Manifest => Some("hunted"),
    };
    let load = match world.burden() {
        Burden::Light => None,
        Burden::Burdened => Some("burdened"),
        Burden::Overloaded => Some("can't move!"),
    };
    let shroud = world.shrouded().then_some(("shrouded", palette::GOOD));
    let blind = player
        .blind_until
        .is_some()
        .then_some(("ink in your eyes", palette::DANGER));
    let hush = world.hushed().then_some(("hushed", palette::GOOD));
    let eyes = world
        .borrowed_eyes()
        .is_some()
        .then_some(("borrowed eyes", palette::GOOD));
    for (word, color) in candle
        .into_iter()
        .chain(load.map(|w| (w, palette::DANGER)))
        .chain(blind)
        .chain(shroud)
        .chain(hush)
        .chain(eyes)
    {
        if !spans.is_empty() {
            spans.push(Span::styled(" · ", dim()));
        }
        spans.push(Span::styled(word, Style::new().fg(color)));
    }
    if let Some(word) = mind {
        if !spans.is_empty() {
            spans.push(Span::styled(" · ", dim()));
        }
        spans.push(Span::styled(word, Style::new().fg(palette::DREAD)));
    }
    Line::from(spans)
}

fn in_view(world: &World) -> Vec<Line<'static>> {
    let ids = world.floor().visible_monsters(world.player().pos);
    if ids.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![Line::styled("─ in view ─", dim())];
    for info in ids
        .iter()
        .filter_map(|&id| world.inspect(id))
        .take(MAX_IN_VIEW)
    {
        let def = world.content().monster(info.kind);
        let mut spans = vec![
            Span::styled(
                format!("{} ", def.glyph),
                Style::new().fg(rgb(def.color)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(def.name.clone(), text()),
        ];
        if info.winding_up.is_some() {
            spans.push(Span::styled(
                " !",
                Style::new()
                    .fg(palette::DANGER)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if info.compelled > 0 {
            spans.push(Span::styled(" (yours)", Style::new().fg(palette::GOOD)));
        } else if let Some(state) = wound_word(&info) {
            spans.push(Span::styled(format!(" ({state})"), dim()));
        }
        lines.push(Line::from(spans));
    }
    if ids.len() > MAX_IN_VIEW {
        lines.push(Line::styled(
            format!("…and {} more", ids.len() - MAX_IN_VIEW),
            dim(),
        ));
    }
    lines.push(Line::default());
    lines
}

fn keys() -> Vec<Line<'static>> {
    let key = |k: &'static str, what: &'static str| {
        Line::from(vec![
            Span::styled(format!("{k:<11}"), text()),
            Span::styled(what, dim()),
        ])
    };
    vec![
        Line::styled("─ keys ─ (? for all)", dim()),
        key("hjkl yubn", "move"),
        key("HJKL o", "run, explore"),
        key(". R", "wait, rest"),
        key("g i", "take, pack"),
        key("t f", "throw, fire"),
        key("s z", "study, rites"),
        key("c C >", "light,doors,go"),
        key("x @ M", "look,self,notes"),
        key("q", "save and quit"),
    ]
}

fn wound_word(info: &MonsterInfo) -> Option<&'static str> {
    let left = info.health * 4 / info.max_health.max(1);
    match left {
        _ if info.health == info.max_health => None,
        3.. => Some("hurt"),
        2 => Some("wounded"),
        1 => Some("badly hurt"),
        0 => Some("near death"),
    }
}

fn look_panel(world: &World, cursor: Point) -> Vec<Line<'static>> {
    let floor = world.floor();
    let mut lines = vec![Line::styled("─ look ─", dim())];
    let monster = floor.monster_at(cursor).and_then(|id| world.inspect(id));

    if let Some(info) = monster.as_ref().filter(|info| info.phantom) {
        let def = world.content().monster(info.kind);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", def.glyph),
                Style::new().fg(rgb(def.color)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(capitalize(&def.name), text().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(
            "It casts no shadow in your light. It isn't really there.",
            Style::new().fg(palette::DREAD),
        ));
    } else if let Some(info) = monster {
        let def = world.content().monster(info.kind);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", def.glyph),
                Style::new().fg(rgb(def.color)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(capitalize(&def.name), text().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(def.description.clone(), dim()));
        lines.push(Line::styled(faction_line(def.faction), dim()));
        if def.boss {
            lines.push(Line::styled(
                "One of the great ones below. It resists Binding.",
                Style::new().fg(palette::DANGER),
            ));
        }
        let mind = match info.mind {
            _ if info.compelled > 0 => "bound to your will".to_string(),
            _ if info.terrified > 0 => "fleeing your dread".to_string(),
            _ if info.unseeing > 0 => "can't see you".to_string(),
            Mind::Unaware => "unaware of you".to_string(),
            Mind::Hunting { .. } => "hunting you".to_string(),
            Mind::Fleeing => "fleeing".to_string(),
        };
        let health = wound_word(&info).unwrap_or("unhurt");
        lines.push(Line::styled(
            format!("{} · {mind}", capitalize(health)),
            text(),
        ));
        lines.push(Line::styled(
            format!("You hit it   {}%", info.your_hit_chance),
            text(),
        ));
        let (lo, hi) = info.its_damage;
        lines.push(Line::styled(
            format!("It hits you  {}% · {lo}–{hi}", info.its_hit_chance),
            text(),
        ));
        if info.winding_up.is_some() {
            lines.push(Line::styled(
                "Its blow is raised. Get off the red tile!",
                Style::new()
                    .fg(palette::DANGER)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if info.compelled > 0 {
            lines.push(Line::styled(
                format!("Compelled for {} more of its actions.", info.compelled),
                Style::new().fg(palette::GOOD),
            ));
        }
        if info.pinned > 0 {
            lines.push(Line::styled(
                format!("Can't move for {} actions.", info.pinned),
                text(),
            ));
        }
        if info.carries_dread {
            lines.push(Line::styled("It carries some of your dread.", dim()));
        }
        for t in &info.known_traits {
            lines.push(Line::styled(format!("Seen: {}", trait_line(t)), dim()));
        }
        if world.has_studied(info.kind) {
            lines.push(Line::styled("You have studied its kind.", dim()));
        }
    } else if cursor == world.player().pos {
        lines.push(Line::styled(
            "You. Acolyte of the Low Bell, a candle in one hand.",
            text(),
        ));
    } else if let Some(top) = floor
        .items_at(cursor)
        .last()
        .filter(|_| floor.is_explored(cursor))
    {
        let count = floor.items_at(cursor).count();
        let def = world.content().item(top.item.kind);
        lines.push(Line::styled(
            crate::log::capitalize(&item_phrase(world, top.item.kind, top.item.count)),
            text().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(def.description.clone(), dim()));
        lines.extend(
            item_stats(world, top.item.kind)
                .into_iter()
                .map(|s| Line::styled(s, text())),
        );
        if count > 1 {
            lines.push(Line::styled(
                format!("…and {} more things here.", count - 1),
                dim(),
            ));
        }
    } else if let Some(&(_, id)) = floor
        .leavings()
        .iter()
        .find(|&&(at, _)| at == cursor && floor.is_explored(cursor))
    {
        let l = world.leaving(id);
        lines.push(Line::styled(
            capitalize(&crate::names::leaving_name(world, id)),
            Style::new()
                .fg(rgb(palette::tier_color(l.tier())))
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            format!("A Leaving ({}).", crate::names::tier_name(l.tier())),
            text(),
        ));
        lines.push(Line::styled(crate::names::leaving_tell(world, id), dim()));
        if l.known {
            lines.push(Line::styled(crate::names::leaving_rule(world, id), text()));
        }
    } else if let Some(a) = world
        .anomaly_at(cursor)
        .filter(|a| a.revealed && floor.is_explored(cursor))
    {
        let (name, what) = crate::names::anomaly_text(a.kind);
        lines.push(Line::styled(
            capitalize(name),
            Style::new().fg(rgb(palette::ANOMALY_FG)),
        ));
        lines.push(Line::styled(what, text()));
    } else if let Some(corpse) = floor
        .corpse_at(cursor)
        .filter(|_| floor.is_explored(cursor))
    {
        let def = world.content().monster(corpse.kind);
        lines.push(Line::styled(
            format!("The body of {}.", crate::log::with_article(&def.name)),
            text().add_modifier(Modifier::BOLD),
        ));
        let state = match corpse.decay(world.turn()) {
            tallow_core::Decay::Fresh => "Fresh.",
            tallow_core::Decay::Swelling if def.health >= tallow_core::corpse::HATCH_HEALTH => {
                "Swelling. Flies will hatch from it soon."
            }
            tallow_core::Decay::Swelling => "Rotting. It will soon be gone.",
        };
        lines.push(Line::styled(state, text()));
        lines.push(Line::styled(
            "Stand on it and press s to study or render it.",
            dim(),
        ));
    } else if let Some(tallow) = floor
        .tallow_at(cursor)
        .filter(|_| floor.is_explored(cursor))
    {
        lines.push(Line::styled(
            format!(
                "Tallow, enough for about {} turns of light. Walk over it to take it.",
                tallow.amount
            ),
            text(),
        ));
    } else if !floor.is_explored(cursor) {
        lines.push(Line::styled("You don't know what is there.", dim()));
    } else {
        lines.push(Line::styled(tile_line(floor.map().tile(cursor)), text()));
        if floor.is_burning(cursor) && floor.is_visible(cursor) {
            lines.push(Line::styled(
                "On fire! It burns whatever stands in it, the Swarm worst of all.",
                Style::new().fg(palette::DANGER),
            ));
        }
        if floor.has_oil(cursor) {
            lines.push(Line::styled(
                "Spilled lamp oil: slippery, and it burns fast.",
                text(),
            ));
        }
        if floor.is_seep(cursor) {
            lines.push(Line::styled(
                "The air here ripples like heat over a road. Something unseen is wrong with it: throw something through before you walk in.",
                Style::new().fg(rgb(palette::ANOMALY_FG)),
            ));
        }
        if floor.is_sanctified(cursor) {
            lines.push(Line::styled(
                "Holy ground. The Dreaming can't cross it; the Taken flinch.",
                Style::new().fg(palette::GOOD),
            ));
        }
        if floor.decoy().is_some_and(|d| d.at == cursor) {
            lines.push(Line::styled(
                "Your false flame. Creatures near it go to it.",
                Style::new().fg(palette::GOOD),
            ));
        }
        if !floor.is_visible(cursor) {
            lines.push(Line::styled("(remembered, not in view)", dim()));
        }
    }
    lines.push(Line::default());
    lines.push(Line::styled("Tab next · Esc done", dim()));
    lines
}

fn target_panel(world: &World, aim: Aim, cursor: Point, path_len: usize) -> Vec<Line<'static>> {
    let doing = match aim {
        Aim::Throw(id) => {
            let what = world
                .inventory_item(id)
                .map_or_else(|| "something".into(), |it| item_phrase(world, it.kind, 1));
            format!("Throwing {what}.")
        }
        Aim::Fire => "Shooting.".into(),
        Aim::Rite(rite) => format!("Casting {}.", world.content().rite(rite).name),
    };
    let mut lines = vec![Line::styled("─ aim ─", dim()), Line::styled(doing, text())];
    if let Some(info) = world
        .floor()
        .monster_at(cursor)
        .and_then(|id| world.inspect(id))
    {
        let def = world.content().monster(info.kind);
        lines.push(Line::styled(format!("At the {}.", def.name), text()));
    }
    if let Aim::Rite(rite) = aim {
        lines.push(Line::styled(crate::names::rite_numbers(world, rite), dim()));
        if let Err(why) = world.rite_check(rite, cursor) {
            let text = crate::log::narrate(&tallow_core::Event::RiteFailed { rite, why }, world)
                .map_or_else(String::new, |(t, _)| t);
            lines.push(Line::styled(text, Style::new().fg(palette::DANGER)));
        }
        lines.push(Line::default());
        lines.push(Line::styled("Enter cast · Tab next", dim()));
        lines.push(Line::styled("Esc cancel", dim()));
        return lines;
    }
    if path_len == 0 {
        lines.push(Line::styled(
            "Nothing can fly that way.",
            Style::new().fg(palette::DANGER),
        ));
    }
    lines.push(Line::styled("The shaded line is where it will fly.", dim()));
    lines.push(Line::default());
    lines.push(Line::styled("Enter fly · Tab next", dim()));
    lines.push(Line::styled("Esc cancel", dim()));
    lines
}

/// Who a faction is and whom it attacks on sight.
fn faction_line(faction: tallow_core::Faction) -> String {
    use tallow_core::Faction;
    let name = |f: Faction| match f {
        Faction::Dreaming => "the Dreaming",
        Faction::Taken => "the Taken",
        Faction::Swarm => "the Swarm",
        Faction::Remnant => "the Remnant",
    };
    let hated: Vec<&str> = Faction::ALL
        .into_iter()
        .filter(|&f| faction.hates(f))
        .map(name)
        .collect();
    let fights = match hated.as_slice() {
        [] => "Wants only you.".to_string(),
        [one] => format!("Attacks {one} on sight."),
        [rest @ .., last] => format!("Attacks {} and {last} on sight.", rest.join(", ")),
    };
    format!("{} · {fights}", capitalize(name(faction)))
}

fn trait_line(t: &Trait) -> String {
    match *t {
        Trait::PackCourage => "flees when no packmate is near.".into(),
        Trait::Pleads => "speaks. It may still be someone.".into(),
        Trait::ShunsLight => "won't cross brazier light.".into(),
        Trait::EatsLight => "its bite snuffs your candle and eats tallow.".into(),
        Trait::Relentless => "always knows where you are.".into(),
        Trait::HeavyBlow {
            damage: (lo, hi), ..
        } => {
            format!("heavy blow, {lo}–{hi}. Aimed a turn ahead at a marked tile.")
        }
        Trait::Blinds { turns } => format!("its hit shrinks your light for {turns} turns."),
        Trait::Chants {
            damage: (lo, hi), ..
        } => format!(
            "chants a rite at you ({lo}–{hi}, and dread). Break its line of sight before the chant ends."
        ),
        Trait::Drags => "its hit drags you toward deep water.".into(),
        Trait::Sings { .. } => "its singing raises your dread while it can see you.".into(),
        Trait::Bursts => "bursts into flies when it dies.".into(),
        Trait::Summons { max, .. } => {
            format!("calls up flies in the dark, up to {max} at a time.")
        }
        Trait::Raises { range, .. } => {
            format!("raises bodies within {range} tiles. Render or burn them first.")
        }
        Trait::Rewrites { .. } => "locks the doors around it, and sets books alight.".into(),
        Trait::Chorus => "several bodies, one life: wound one and all bleed.".into(),
        Trait::Swims => "at home in deep water.".into(),
        Trait::Sweeps => "its blows fall on a cross of marked tiles. Step out of the cross.".into(),
        Trait::Vessel => "a body he wears. Bind or exorcise it to trap him inside.".into(),
        Trait::Unholy => "can't set foot on holy ground.".into(),
        Trait::Gnaws => "chews through shut doors, slowly.".into(),
        Trait::Undying => "cannot be killed.".into(),
    }
}

fn tile_line(tile: Tile) -> &'static str {
    match tile {
        Tile::Floor => "Flagstones, worn smooth by centuries of feet.",
        Tile::Wall => "Old stone, sweating in the cold.",
        Tile::Door => "An open door. Wood: it burns. C closes doors beside you.",
        Tile::DoorClosed => {
            "A shut door. Walk into it to open it. The Taken and the Remnant can open doors; nightmares and vermin can't."
        }
        Tile::DoorSealed => "A door held shut by your Seal. Only you can open it.",
        Tile::ColdBrazier => {
            "A cold brazier. Walk into it with your candle lit to light it: braziers keep the Dreaming off and ease dread."
        }
        Tile::Bookshelf => "Shelves of crumbling books. They would burn well.",
        Tile::Pew => "A wooden pew. You can climb over it. It would burn.",
        Tile::ShallowWater => "Black water, ankle-deep. Slow going. Nothing burns here.",
        Tile::DeepWater => {
            "Deep water. Wading in puts your candle out, and you can't light it again until you're out. Slow."
        }
        Tile::RottenFloor => {
            "Rotten boards. They will give way under you and drop you to the floor below (you'll be bruised, not killed)."
        }
        Tile::Pit => "A hole where the boards gave way. Far below, a floor.",
        Tile::Altar => {
            "The altar of the Low Bell, bare where the Vigil Candle stood. Walk into it to set the candle there."
        }
        Tile::BellRope => {
            "A bell rope. Pull it (walk into it) and the bell rings out: everything within earshot comes to you. The Taken close by cower."
        }
        Tile::StairsDown => "Stairs down. There is no coming back up.",
        Tile::StairsUp => {
            "A stair up. On the way down it is sealed; with the Vigil Candle, it is the way home."
        }
        Tile::Brazier => {
            "A brazier, burning without fuel. Some things in the dark will not cross its light. \
             Rest here with your candle snuffed: dread ebbs and your tallow keeps."
        }
    }
}
