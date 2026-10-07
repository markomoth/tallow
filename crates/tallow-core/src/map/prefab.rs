//! Hand-drawn maps written as ASCII. Used for tests now and for vaults later.
//!
//! Legend: `#` wall, `.` floor, `+` door, `>` stairs down, `<` stairs up,
//! `&` brazier, `@` player start (on floor). Short rows are padded with wall.

use super::{Map, Tile};
use crate::geom::Point;

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
                'D' => Tile::DoorClosed,
                'B' => Tile::Bookshelf,
                '=' => Tile::Pew,
                '|' => Tile::BellRope,
                'o' => Tile::ColdBrazier,
                '~' => Tile::ShallowWater,
                'W' => Tile::DeepWater,
                ',' => Tile::RottenFloor,
                'A' => Tile::Altar,
                '>' => Tile::StairsDown,
                '<' => Tile::StairsUp,
                '&' => Tile::Brazier,
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

    #[test]
    fn parses_a_small_prefab() {
        let (map, start) = parse("#####\n#@+>#\n#.&<#\n#####").unwrap();
        assert_eq!((map.width(), map.height()), (5, 4));
        assert_eq!(start, Point::new(1, 1));
        assert_eq!(map.tile(Point::new(2, 1)), Tile::Door);
        assert_eq!(map.tile(Point::new(3, 1)), Tile::StairsDown);
        assert_eq!(map.tile(Point::new(2, 2)), Tile::Brazier);
        assert_eq!(map.tile(Point::new(3, 2)), Tile::StairsUp);
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
}
