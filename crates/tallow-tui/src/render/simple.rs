//! Simple mode (`tallow --simple`): the terminal's own background and its 16
//! standard colors, no animation. For terminals and setups where truecolor
//! and a painted background don't work well.
//!
//! The screen is drawn as usual, then every cell is mapped: backgrounds become
//! the terminal's own except a few that carry meaning (a raised blow, an aim
//! line, holy ground), and every color becomes its nearest standard one.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// Maps a finished frame to plain terminal colors.
pub fn simplify(buf: &mut Buffer) {
    for cell in buf.content.iter_mut() {
        cell.fg = foreground(cell.fg);
        cell.bg = background(cell.bg);
    }
}

fn foreground(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => nearest(r, g, b, true),
        other => other,
    }
}

/// Dark or grey backgrounds (floors, walls, popups, memory) disappear; the
/// strong colored ones stay, as plain dark colors.
fn background(color: Color) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    let (luma, saturation) = measure(r, g, b);
    if luma < 45.0 || saturation < 0.3 {
        Color::Reset
    } else {
        nearest(r, g, b, false)
    }
}

fn measure(r: u8, g: u8, b: u8) -> (f32, f32) {
    let (r, g, b) = (f32::from(r), f32::from(g), f32::from(b));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let luma = 0.30 * r + 0.59 * g + 0.11 * b;
    let saturation = if max == 0.0 { 0.0 } else { (max - min) / max };
    (luma, saturation)
}

/// The nearest of the 16 standard colors. `light` allows the bright variants.
fn nearest(r: u8, g: u8, b: u8, light: bool) -> Color {
    let (luma, saturation) = measure(r, g, b);
    if saturation < 0.25 {
        return match luma {
            l if l < 90.0 => Color::DarkGray,
            l if l < 175.0 || !light => Color::Gray,
            _ => Color::White,
        };
    }
    // Dim colors (far from the light, or remembered) read best as dark grey.
    if luma < 70.0 && light {
        return Color::DarkGray;
    }
    let bright = light && r.max(g).max(b) > 185;
    let (rf, gf, bf) = (f32::from(r), f32::from(g), f32::from(b));
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let hue = if max == rf {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if max == gf {
        60.0 * ((bf - rf) / delta + 2.0)
    } else {
        60.0 * ((rf - gf) / delta + 4.0)
    };
    match (hue, bright) {
        (h, false) if !(20.0..330.0).contains(&h) => Color::Red,
        (h, true) if !(20.0..330.0).contains(&h) => Color::LightRed,
        (h, false) if h < 65.0 => Color::Yellow,
        (h, true) if h < 65.0 => Color::LightYellow,
        (h, false) if h < 160.0 => Color::Green,
        (h, true) if h < 160.0 => Color::LightGreen,
        (h, false) if h < 200.0 => Color::Cyan,
        (h, true) if h < 200.0 => Color::LightCyan,
        (h, false) if h < 260.0 => Color::Blue,
        (h, true) if h < 260.0 => Color::LightBlue,
        (_, false) => Color::Magenta,
        (_, true) => Color::LightMagenta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::palette;

    #[test]
    fn dark_and_grey_backgrounds_become_the_terminals() {
        assert_eq!(background(palette::VOID), Color::Reset);
        assert_eq!(background(palette::rgb(palette::FLOOR_BG)), Color::Reset);
        assert_eq!(background(palette::rgb(palette::WALL_BG)), Color::Reset);
    }

    #[test]
    fn meaningful_backgrounds_stay_visible() {
        assert_eq!(background(palette::rgb(palette::TELEGRAPH)), Color::Red);
        assert_ne!(background(palette::rgb(palette::AIM)), Color::Reset);
        assert_ne!(background(palette::rgb(palette::HOLY_BG)), Color::Reset);
    }

    #[test]
    fn foregrounds_keep_their_character() {
        assert_eq!(foreground(palette::PLAYER), Color::LightYellow);
        assert_eq!(foreground(palette::DANGER), Color::LightRed);
        assert_eq!(foreground(palette::rgb([40, 44, 60])), Color::DarkGray);
        assert_eq!(foreground(palette::TEXT), Color::White);
    }
}
