//! Wrapping styled lines to a width, with hanging indents: a wrapped line
//! continues under its text, not back at the left edge.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// The widest key column a hanging indent follows ("hjkl yubn     ").
const MAX_HANG: usize = 16;

/// Where a wrapped line's continuations start. A line led by a short key
/// span ("a  ", "Riposte: ", a padded key column) hangs after it; otherwise
/// continuations line up with the line's own leading spaces.
pub fn hang(line: &Line) -> usize {
    let lead = line
        .spans
        .iter()
        .flat_map(|s| s.content.chars())
        .take_while(|&c| c == ' ')
        .count();
    if let [first, _, ..] = line.spans.as_slice() {
        let width = first.content.chars().count();
        if first.content.ends_with(' ') && width <= MAX_HANG && width > lead {
            return width;
        }
    }
    lead
}

/// Wraps `line` to `width` columns, continuing at `hang`.
pub fn wrap_hanging(line: &Line<'static>, width: usize, hang: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let hang = hang.min(width / 2);
    let style = line.style;
    let chars: Vec<(char, Style)> = line
        .spans
        .iter()
        .flat_map(|s| s.content.chars().map(move |c| (c, style.patch(s.style))))
        .collect();
    if chars.len() <= width {
        return vec![line.clone()];
    }

    let mut rows: Vec<Vec<(char, Style)>> = vec![Vec::new()];
    let mut i = 0;
    while i < chars.len() {
        let space = chars[i].0 == ' ';
        let end = (i..chars.len())
            .find(|&j| (chars[j].0 == ' ') != space)
            .unwrap_or(chars.len());
        let run = &chars[i..end];
        let continuing = rows.len() > 1;
        let row = rows.last_mut().expect("never empty");
        let fresh = continuing && row.len() == hang;
        if space {
            // Spaces at a break are dropped; elsewhere they are kept.
            if !fresh && row.len() + run.len() <= width {
                row.extend_from_slice(run);
            } else if !fresh {
                rows.push(vec![(' ', Style::default()); hang]);
            }
        } else if row.len() + run.len() <= width {
            row.extend_from_slice(run);
        } else if !fresh && row.iter().any(|&(c, _)| c != ' ') && hang + run.len() <= width {
            while rows
                .last()
                .is_some_and(|r| r.last().is_some_and(|&(c, _)| c == ' '))
            {
                rows.last_mut().expect("checked").pop();
            }
            let mut next = vec![(' ', Style::default()); hang];
            next.extend_from_slice(run);
            rows.push(next);
        } else {
            // Too long for any line: split it.
            for &cell in run {
                if rows.last().expect("never empty").len() >= width {
                    rows.push(vec![(' ', Style::default()); hang]);
                }
                rows.last_mut().expect("never empty").push(cell);
            }
        }
        i = end;
    }
    rows.into_iter().map(|row| to_line(&row)).collect()
}

/// Wraps every line, each with its own hanging indent.
pub fn wrap_all(lines: &[Line<'static>], width: usize) -> Vec<Line<'static>> {
    lines
        .iter()
        .flat_map(|l| wrap_hanging(l, width, hang(l)))
        .collect()
}

fn to_line(cells: &[(char, Style)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut text = String::new();
    let mut current = None;
    for &(c, style) in cells {
        if current != Some(style) {
            if let Some(s) = current {
                spans.push(Span::styled(std::mem::take(&mut text), s));
            }
            current = Some(style);
        }
        text.push(c);
    }
    if let Some(s) = current {
        spans.push(Span::styled(text, s));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn continuations_hang_under_the_text() {
        let line = Line::from(vec![
            Span::raw("a  "),
            Span::raw("Compel the creature to fight for you"),
        ]);
        assert_eq!(hang(&line), 3);
        assert_eq!(
            plain(&wrap_hanging(&line, 20, hang(&line))),
            vec!["a  Compel the", "   creature to fight", "   for you"]
        );
    }

    #[test]
    fn leading_spaces_set_the_hang() {
        let line = Line::raw("   one two three four");
        assert_eq!(hang(&line), 3);
        assert_eq!(
            plain(&wrap_hanging(&line, 12, 3)),
            vec!["   one two", "   three", "   four"]
        );
    }

    #[test]
    fn short_lines_and_long_words_survive() {
        assert_eq!(plain(&wrap_all(&[Line::raw("short")], 10)), vec!["short"]);
        assert_eq!(
            plain(&wrap_hanging(&Line::raw("abcdefghij"), 4, 0)),
            vec!["abcd", "efgh", "ij"]
        );
    }
}
