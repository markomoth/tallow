//! Prints a generated floor as ASCII, monsters included, for eyeballing
//! map generation and spawns.
//!
//! cargo run -p tallow-core --example floor -- [seed] [depth]

use tallow_core::map::generate;
use tallow_core::{Content, MAX_DEPTH, Point, Tile, rng, spawn};

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .map(|a| a.parse::<u64>().expect("numbers only"));
    let seed = args.next().unwrap_or(1);
    let depth = args.next().unwrap_or(1).clamp(1, MAX_DEPTH.into()) as u8;

    let content = Content::bundled();
    let mut rng = rng::floor_rng(seed, depth);
    let layout = generate::floor(&mut rng, depth, depth < MAX_DEPTH);
    let spawns = spawn::populate(&mut rng, content, &layout.map, layout.start, depth);
    let map = &layout.map;
    for y in 0..map.height() {
        let row: String = (0..map.width())
            .map(|x| {
                let p = Point::new(x, y);
                if let Some(&(kind, _)) = spawns.iter().find(|&&(_, at)| at == p) {
                    return content.monster(kind).glyph;
                }
                match map.tile(p) {
                    Tile::Wall => '#',
                    Tile::Floor => '.',
                    Tile::Door => '\'',
                    Tile::DoorClosed | Tile::DoorSealed => '+',
                    Tile::ColdBrazier => 'o',
                    Tile::Bookshelf => 'B',
                    Tile::Pew => '=',
                    Tile::BellRope => '|',
                    Tile::StairsDown => '>',
                    Tile::StairsUp => '<',
                    Tile::Brazier => '&',
                }
            })
            .collect();
        println!("{row}");
    }
    for (kind, at) in &spawns {
        println!("{} at ({}, {})", content.monster(*kind).name, at.x, at.y);
    }
}
