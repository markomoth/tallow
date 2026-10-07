//! Hand-drawn maps written as ASCII.
//!
//! Legend: `#` wall, `.` floor, `+` door, `@` player start (on floor).
//! Short rows are padded with wall.

use super::{Map, Tile};
use crate::geom::Point;

/// The first floor under the church, used until procedural generation lands in M1.
pub const UNDERCROFT: &str = include_str!("../../../../assets/prefabs/undercroft.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefabError {
    Empty,
    NoStart,
    MultipleStarts,
    UnknownGlyph { glyph: char, at: Point },
}

impl std::fmt::Display for PrefabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrefabError::Empty => write!(f, "prefab has no rows"),
            PrefabError::NoStart => write!(f, "prefab has no '@' start"),
            PrefabError::MultipleStarts => write!(f, "prefab has more than one '@' start"),
            PrefabError::UnknownGlyph { glyph, at } => {
                write!(f, "unknown glyph {glyph:?} at ({}, {})", at.x, at.y)
            }
        }
    }
}

impl std::error::Error for PrefabError {}

/// Parses an ASCII prefab into a map and the player's start position.
pub fn parse(text: &str) -> Result<(Map, Point), PrefabError> {
    let rows: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let height = rows.len() as i32;
    let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
    if height == 0 || width == 0 {
        return Err(PrefabError::Empty);
    }

    let mut map = Map::filled(width, height);
    let mut start = None;
    for (y, row) in rows.iter().enumerate() {
        for (x, glyph) in row.chars().enumerate() {
            let at = Point::new(x as i32, y as i32);
            let tile = match glyph {
                '#' => Tile::Wall,
                '.' => Tile::Floor,
                '+' => Tile::Door,
                '@' => {
                    if start.replace(at).is_some() {
                        return Err(PrefabError::MultipleStarts);
                    }
                    Tile::Floor
                }
                _ => return Err(PrefabError::UnknownGlyph { glyph, at }),
            };
            map.set(at, tile);
        }
    }
    let start = start.ok_or(PrefabError::NoStart)?;
    Ok((map, start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Direction;
    use std::collections::{HashSet, VecDeque};

    #[test]
    fn parses_a_small_prefab() {
        let (map, start) = parse("####\n#@+#\n#..#\n####").unwrap();
        assert_eq!((map.width(), map.height()), (4, 4));
        assert_eq!(start, Point::new(1, 1));
        assert_eq!(map.tile(Point::new(2, 1)), Tile::Door);
        assert_eq!(map.tile(start), Tile::Floor);
    }

    #[test]
    fn short_rows_are_padded_with_wall() {
        let (map, _) = parse("#####\n#@\n#####").unwrap();
        assert_eq!(map.tile(Point::new(4, 1)), Tile::Wall);
    }

    #[test]
    fn rejects_bad_prefabs() {
        assert_eq!(parse(""), Err(PrefabError::Empty));
        assert_eq!(parse("#.#"), Err(PrefabError::NoStart));
        assert_eq!(parse("@@"), Err(PrefabError::MultipleStarts));
        assert!(matches!(
            parse("@x"),
            Err(PrefabError::UnknownGlyph { glyph: 'x', .. })
        ));
    }

    #[test]
    fn undercroft_is_fully_connected() {
        let (map, start) = parse(UNDERCROFT).expect("undercroft prefab should parse");

        let mut seen = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(p) = queue.pop_front() {
            for dir in Direction::ALL {
                let next = p + dir;
                if map.is_walkable(next) && seen.insert(next) {
                    queue.push_back(next);
                }
            }
        }

        for y in 0..map.height() {
            for x in 0..map.width() {
                let p = Point::new(x, y);
                assert!(
                    !map.is_walkable(p) || seen.contains(&p),
                    "walkable tile ({x}, {y}) is unreachable from the start"
                );
            }
        }
    }
}
