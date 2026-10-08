//! The status sidebar: where you are, how you are, what you can see, and
//! the few keys that matter right now. Look draws its own card on the map.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap};
use tallow_core::candle::{CAP, LOW_AT};
use tallow_core::item::LIGHT_LOAD;
use tallow_core::{
    Biome, Burden, CandleState, DreadBand, MAX_DEPTH, Mind, MonsterInfo, Point, Tile, Trait, World,
};

use crate::app::Aim;
use crate::names::item_phrase;

use super::palette::{self, rgb};
use crate::app::{App, Mode};
use crate::log::capitalize;

const BAR_WIDTH: usize = 10;
const MAX_IN_VIEW: usize = 3;
const MAX_HINTS: usize = 4;
/// Columns inside the sidebar's rule and padding.
const SIDEBAR_INNER: usize = 23;

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
        tallow_core::Stage::Church => "The Church",
    }
}

/// The fight at the bottom, or what's behind you on the way up.
fn pursuit(world: &World) -> Line<'static> {
    use tallow_core::Phase;
    if world.following_present() {
        return Line::styled(
            if world.can_flare() {
                "The Following is here · F flares"
            } else {
                "The Following is here"
            },
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
            Line::styled(stage_name(world), text().add_modifier(Modifier::BOLD)),
            Line::styled(
                match world.stage() {
                    tallow_core::Stage::Descent => {
                        format!("floor {depth} of {MAX_DEPTH} · level {}", player.level)
                    }
                    tallow_core::Stage::Ascent(a) => format!(
                        "ascent {a} of {} · level {}",
                        tallow_core::throne::ASCENT_FLOORS,
                        player.level
                    ),
                    tallow_core::Stage::Church => format!("level {}", player.level),
                },
                dim(),
            ),
            Line::default(),
            bar(
                "Health",
                player.health,
                player.max_health,
                palette::HEALTH,
                palette::HEALTH_EMPTY,
            )
            .spans_with_max(player.max_health),
            bar(
                "Candle",
                player.candle.tallow(),
                CAP,
                palette::CANDLE,
                palette::CANDLE_EMPTY,
            ),
            dread_bar(player.dread.value()),
        ];
        lines.extend(mind_line(world));
        lines.push(bar(
            "Load  ",
            world.load() / 10,
            LIGHT_LOAD / 10,
            palette::LOAD,
            palette::LOAD_EMPTY,
        ));
        let conditions = conditions(world);
        if !conditions.spans.is_empty() {
            lines.push(conditions);
        }
        let pursuit = pursuit(world);
        if !pursuit.spans.is_empty() {
            lines.push(pursuit);
        }
        lines.push(Line::default());
        match self.app.mode() {
            Mode::Target { aim, cursor } => {
                lines.extend(target_panel(world, aim, cursor, self.app.aim_path().len()));
            }
            Mode::Dead | Mode::Won => lines.extend(in_view(world)),
            Mode::Title(_)
            | Mode::Play
            | Mode::Look { .. }
            | Mode::Help
            | Mode::Journal(_)
            | Mode::Pack { .. }
            | Mode::Draft
            | Mode::Forget
            | Mode::Shove
            | Mode::Sheet
            | Mode::Corpse
            | Mode::Rites
            | Mode::Leaving(_) => {
                lines.extend(in_view(world));
                lines.extend(hints(world));
            }
        }

        let frayed = player.dread.band() >= DreadBand::Frayed;
        let block = Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::new().fg(if frayed {
                palette::DREAD_RULE
            } else {
                palette::BORDER
            }))
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

/// Lets the health bar show its ceiling: "24/24".
trait WithMax {
    fn spans_with_max(self, max: u32) -> Self;
}

impl WithMax for Line<'static> {
    fn spans_with_max(mut self, max: u32) -> Self {
        if let Some(last) = self.spans.last_mut() {
            last.content = format!("{}/{max}", last.content).into();
        }
        self
    }
}

/// The dread bar: its cells deepen through the bands, and the empty track
/// marks where uneasy (40) and frayed (70) begin.
fn dread_bar(value: u32) -> Line<'static> {
    const MARKS: [usize; 2] = [4, 7];
    let filled = (value as usize * BAR_WIDTH).div_ceil(100).min(BAR_WIDTH);
    let mut spans = vec![Span::styled("Dread  ", Style::new().fg(palette::DREAD))];
    for i in 0..BAR_WIDTH {
        let band = MARKS.iter().filter(|&&m| i >= m).count();
        spans.push(if i < filled {
            Span::styled("█", Style::new().fg(palette::DREAD_BANDS[band]))
        } else if MARKS.contains(&i) {
            Span::styled("╎", Style::new().fg(palette::DREAD_MARK))
        } else {
            Span::styled("·", Style::new().fg(palette::DREAD_EMPTY))
        });
    }
    spans.push(Span::styled(format!(" {value}"), text()));
    Line::from(spans)
}

/// What your dread is doing to you, named. A Nightmare's coming is told, but
/// never when.
fn mind_line(world: &World) -> Option<Line<'static>> {
    let band = world.player().dread.band();
    let (word, what, danger) = match band {
        DreadBand::Calm => return None,
        DreadBand::Uneasy => ("uneasy", "whispers", false),
        DreadBand::Frayed if world.nightmare_coming() => ("frayed", "it is coming", true),
        DreadBand::Frayed => ("frayed", "phantoms walk", false),
        DreadBand::Manifest => ("hunted", "your fear walks", true),
    };
    Some(Line::from(vec![
        Span::styled(
            word,
            Style::new()
                .fg(palette::DREAD_BANDS[2])
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", dim()),
        Span::styled(
            what,
            if danger {
                Style::new()
                    .fg(palette::DANGER)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim()
            },
        ),
    ]))
}

/// A section heading with a rule running to the edge.
fn heading(name: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{name} "), dim()),
        Span::styled(
            "─".repeat(SIDEBAR_INNER.saturating_sub(name.len() + 1)),
            Style::new().fg(palette::BORDER),
        ),
    ])
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
    Line::from(spans)
}

fn in_view(world: &World) -> Vec<Line<'static>> {
    let ids = world.floor().visible_monsters(world.player().pos);
    if ids.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![heading("in view")];
    for info in ids
        .iter()
        .filter_map(|&id| world.inspect(id))
        .take(MAX_IN_VIEW)
    {
        let def = world.content().monster(info.kind);
        // What it's doing, on the right; how hurt it is, in its name's color.
        let (state, color) = if info.winding_up.is_some() {
            ("strikes!", palette::DANGER)
        } else if info.compelled > 0 {
            ("yours", palette::GOOD)
        } else if info.terrified > 0 || info.mind == Mind::Fleeing {
            ("fleeing", palette::TEXT_DIM)
        } else if info.unseeing > 0 {
            ("blind", palette::GOOD)
        } else if matches!(info.mind, Mind::Hunting { .. }) {
            ("hunting", palette::DANGER)
        } else {
            ("unaware", palette::TEXT_DIM)
        };
        let name_color = match wound_word(&info) {
            None | Some("hurt") => palette::TEXT,
            Some("wounded") => palette::ACCENT,
            Some(_) => palette::DANGER,
        };
        let room = SIDEBAR_INNER.saturating_sub(2 + 1 + state.len());
        let name: String = if def.name.chars().count() > room {
            def.name
                .chars()
                .take(room.saturating_sub(1))
                .collect::<String>()
                + "…"
        } else {
            def.name.clone()
        };
        let pad = SIDEBAR_INNER.saturating_sub(2 + name.chars().count() + state.len());
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", def.glyph),
                Style::new().fg(rgb(def.color)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(name, Style::new().fg(name_color)),
            Span::raw(" ".repeat(pad)),
            Span::styled(
                state,
                Style::new()
                    .fg(color)
                    .add_modifier(if info.winding_up.is_some() {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]));
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

/// The few keys that matter right now, most pressing first. The full list
/// is one key away.
fn hints(world: &World) -> Vec<Line<'static>> {
    let player = world.player();
    let here = player.pos;
    let floor = world.floor();
    let hostile: Vec<MonsterInfo> = floor
        .visible_monsters(here)
        .iter()
        .filter_map(|&id| world.inspect(id))
        .filter(|i| !i.phantom && i.compelled == 0 && matches!(i.mind, Mind::Hunting { .. }))
        .collect();
    let beside = hostile
        .iter()
        .any(|i| (i.pos.x - here.x).abs().max((i.pos.y - here.y).abs()) == 1);

    let mut hints: Vec<(&str, String)> = Vec::new();
    if world.can_flare() && world.following_present() {
        hints.push(("F", "flare: drive it back".into()));
    }
    if world.corpse_here().is_some() {
        hints.push(("s", "study or render it".into()));
    }
    if floor.items_at(here).next().is_some() || floor.leavings().iter().any(|&(at, _)| at == here) {
        hints.push(("g", "pick it up".into()));
    }
    match floor.map().tile(here) {
        Tile::StairsDown if world.stage() == tallow_core::Stage::Descent => {
            hints.push((">", "go down".into()));
        }
        Tile::StairsUp if player.vigil => hints.push(("<", "go up".into())),
        _ => {}
    }
    if !hostile.is_empty() {
        hints.push(("x", "look at it".into()));
    }
    if beside {
        hints.push(("G", "guard and wait".into()));
        hints.push(("S", "shove it back".into()));
    }
    let ready = world
        .known_rites()
        .iter()
        .filter(|&&r| world.rite_recharge(r) == 0 && player.dread.value() >= world.rite_cost(r))
        .count();
    if ready > 0 {
        hints.push((
            "z",
            if ready == 1 {
                "1 rite ready".to_string()
            } else {
                format!("{ready} rites ready")
            },
        ));
    }
    match player.candle.state() {
        CandleState::Lit | CandleState::Guttering if !player.vigil => {
            hints.push(("c", "snuff the candle".into()));
        }
        CandleState::Snuffed => hints.push(("c", "light the candle".into())),
        _ => {}
    }
    if hostile.is_empty() {
        if player.health < player.max_health {
            hints.push(("R", "rest until healed".into()));
        }
        hints.push(("o", "explore".into()));
    }
    hints.truncate(MAX_HINTS);

    let mut lines = vec![heading("now")];
    for (key, what) in hints {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{key:<3}"),
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(what, dim()),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled(
            "?  ",
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("every key", dim()),
    ]));
    lines
}

pub(super) fn wound_word(info: &MonsterInfo) -> Option<&'static str> {
    let left = info.health * 4 / info.max_health.max(1);
    match left {
        _ if info.health == info.max_health => None,
        3.. => Some("hurt"),
        2 => Some("wounded"),
        1 => Some("badly hurt"),
        0 => Some("near death"),
    }
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
pub(super) fn faction_line(faction: tallow_core::Faction) -> String {
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
    let way = match faction {
        Faction::Swarm => "Flanks you; quicker in the dark.",
        Faction::Taken => "Holds back in light; fiercer in the dark.",
        Faction::Remnant => "Keeps to its post.",
        Faction::Dreaming => "Slowed by light; half-there in the dark.",
    };
    format!("{} · {fights} {way}", capitalize(name(faction)))
}

pub(super) fn trait_line(t: &Trait) -> String {
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
        Trait::Nightmare => "made of your fear: it comes apart when you're calm.".into(),
        Trait::FearsLight => "scatters from candlelight; bold again in the dark.".into(),
        Trait::Mends => "badly hurt, it runs off to mend, and comes back.".into(),
        Trait::Shoots {
            damage: (lo, hi),
            range,
        } => format!("strikes from up to {range} tiles ({lo}–{hi}) and keeps its distance."),
        Trait::Lunges { bonus } => format!(
            "from two tiles off in a line it marks your tile, then leaps (+{bonus}). Step aside."
        ),
        Trait::Grabs { turns } => {
            format!("its hit holds you for {turns} turns: you can't step away (shove it off).")
        }
        Trait::Feeds { dread } => format!("each blow it lands adds {dread} dread."),
        Trait::Snuffs { range, cooldown } => format!(
            "puts out your candle when it comes within {range} tiles; not again for {cooldown} of its actions."
        ),
        Trait::Doubles { max, .. } => {
            format!("sends false copies of itself at you, up to {max} at a time.")
        }
    }
}

pub(super) fn tile_line(tile: Tile) -> &'static str {
    match tile {
        Tile::Floor => "Flagstones, worn smooth by centuries of feet.",
        Tile::Wall => "Old stone, sweating in the cold.",
        Tile::Door => "An open door. Wood: it burns. C closes doors beside you.",
        Tile::DoorClosed => {
            "A shut door. Walk into it to open it. The Taken and the Remnant can open doors; the Dreaming and vermin can't (but Nightmares made of your fear can)."
        }
        Tile::DoorSealed => "A door held shut by your Seal. Only you can open it.",
        Tile::ColdBrazier => {
            "A cold brazier. Walk into it with your candle lit to light it: braziers keep the Dreaming off, and take dread for health."
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
             Walk into it to offer it your dread: it mends you for it. Its light breeds no dread, and your tallow keeps."
        }
    }
}
