//! The start menu: the church at night, its windows lit, and what to do next.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use tallow_core::Rgb;

use super::palette;
use crate::app::{App, TitleChoice};

/// The church above the labyrinth. `#` is lit glass, `~` graveyard mist.
const CHURCH: [&str; 24] = [
    "                    *                  |",
    "      ·                              --+--        *             .--.",
    "             ·                         |                       /    \\",
    "                           ·          /^\\                     |      |",
    "    v  v                             /   \\                     \\    /",
    "                                    /  :  \\         ·           `--'",
    "          v                        /   :   \\                              ·",
    "                                 /_____:_____\\           v",
    "                                 |   _/^\\_   |                          ·",
    "   *                             |  | ### |  |      ·",
    "             /\\          ·       |  |_###_|  |                  /\\",
    "            /  \\                 |___________|                 /  \\",
    "           /    \\           ____/             \\____           /    \\",
    "          /______\\         /   /     .'#'.     \\   \\         /______\\",
    "          | /^^\\ |        /   /     ( ### )     \\   \\        | /^^\\ |",
    "          | |##| |_______/  _/       `.#.'       \\_  \\_______| |##| |",
    "          | |##| |  _   _  |#|   /^\\  /^\\  /^\\   |#|  _   _  | |##| |",
    "          | |##| | |#| |#| |#|  |###| |#| |###|  |#| |#| |#| | |##| |",
    "          | |##| | |#| |#| |#|  |###| |#| |###|  |#| |#| |#| | |##| |",
    "          | |##| | |#| |#| |#|  |___| /^\\ |___|  |#| |#| |#| | |##| |",
    "          | |##| | |#| |#| |#|  |___|/###\\|___|  |#| |#| |#| | |##| |",
    "          |_|__|_|_|_|_|_|_|_|__|___|#####|___|__|_|_|_|_|_|_|_|__|_|",
    "    +          n               ~~~ /#######\\ ~~~         +             n     +",
    "  ~~|~~~  n  ~~~  |   ~~~   +    /_________\\  ~~~   n   |~~~   n  ~~~   ~~|~~",
];

/// The first graveyard row.
const GROUND: usize = 22;
/// Rows under the art: title, line, gap, two choices, hint.
const MENU_HEIGHT: u16 = 6;

const STONE: Rgb = [112, 104, 98];
const GLASS: Rgb = [255, 172, 72];
const GLASS_BG: Rgb = [46, 24, 8];
const CROSS: Rgb = [226, 196, 110];
const MOON: Rgb = [214, 208, 184];
const STAR: Rgb = [130, 130, 150];
const CROW: Rgb = [76, 68, 80];
const MIST: Rgb = [78, 86, 104];
const GRAVE: Rgb = [92, 88, 86];

pub fn draw(frame: &mut Frame, area: Rect, app: &App, choice: TitleChoice, time: f32) {
    // The art and menu fit 30 rows exactly; taller terminals get a gap
    // between them, and a short one loses sky first, never the menu.
    let art = CHURCH.len() as u16;
    let gap = u16::from(area.height > art + MENU_HEIGHT);
    let wanted = art + gap + MENU_HEIGHT;
    let skip = usize::from(wanted.saturating_sub(area.height));
    let height = wanted - skip as u16;
    let top = area.y + area.height.saturating_sub(height) / 2;
    let width = CHURCH.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
    let left = area.x + area.width.saturating_sub(width) / 2;

    let buf = frame.buffer_mut();
    for (row, line) in CHURCH.iter().enumerate().skip(skip) {
        let y = top + (row - skip) as u16;
        for (col, ch) in line.chars().enumerate() {
            if ch != ' ' {
                paint(buf, left + col as u16, y, row, col, ch, time);
            }
        }
    }

    let label = if app.resumed() {
        let world = app.world();
        format!(
            "Continue      floor {}, turn {}",
            world.depth(),
            world.turn()
        )
    } else {
        "Play now".to_string()
    };
    let item = |key: &str, text: String, picked: bool| {
        let (marker, style) = if picked {
            (
                "▸ ",
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::new().fg(palette::TEXT_DIM))
        };
        Line::styled(format!("{marker}{key}  {text:<34}"), style)
    };
    let lines = vec![
        Line::styled(
            "T A L L O W",
            Style::new()
                .fg(palette::CANDLE)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "Something under the church has taken the candle.",
            Style::new().fg(palette::TEXT_DIM),
        ),
        Line::default(),
        item("p", label, choice == TitleChoice::Play),
        item("q", "Quit".into(), choice == TitleChoice::Quit),
        Line::styled(
            "↑↓ choose · Enter confirm",
            Style::new().fg(palette::BORDER),
        ),
    ];
    let menu = Rect::new(
        area.x,
        top + (CHURCH.len() - skip) as u16 + gap,
        area.width,
        MENU_HEIGHT,
    )
    .intersection(area);
    frame.render_widget(Paragraph::new(lines).centered(), menu);
}

/// One character of the art, colored by what it is and where it stands.
fn paint(buf: &mut Buffer, x: u16, y: u16, row: usize, col: usize, ch: char, time: f32) {
    let Some(cell) = buf.cell_mut((x, y)) else {
        return;
    };
    // Each window flickers on its own, like a candle behind it.
    let flicker = 0.82 + 0.18 * (time * 3.1 + col as f32 * 0.7 + row as f32 * 1.3).sin();
    let drift = 0.75 + 0.25 * (col as f32 * 0.35 + row as f32 - time * 0.6).sin();
    let (fg, bg) = match ch {
        '#' => (scale(GLASS, flicker), Some(scale(GLASS_BG, flicker))),
        '~' => (scale(MIST, drift), None),
        '|' | '-' | '+' if row <= 2 => (CROSS, None),
        _ if row >= GROUND => (GRAVE, None),
        '·' => (STAR, None),
        '*' => (
            scale(STAR, 1.1 + 0.25 * (time * 1.7 + col as f32).sin()),
            None,
        ),
        'v' => (CROW, None),
        _ if row <= 6 && col >= 60 => (MOON, None),
        // Moonlight from the right.
        _ => (scale(STONE, 0.8 + 0.4 * col as f32 / 80.0), None),
    };
    cell.set_char(ch).set_fg(palette::rgb(fg));
    if let Some(bg) = bg {
        cell.set_bg(palette::rgb(bg));
    }
    cell.modifier = Modifier::empty();
}

fn scale(color: Rgb, by: f32) -> Rgb {
    color.map(|c| (f32::from(c) * by).clamp(0.0, 255.0) as u8)
}
