//! Prints a generated floor as ASCII, for eyeballing map generation.
//!
//! cargo run -p tallow-core --example floor -- [seed] [depth]

use tallow_core::map::generate;
use tallow_core::{MAX_DEPTH, Tile, rng};

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .map(|a| a.parse::<u64>().expect("numbers only"));
    let seed = args.next().unwrap_or(1);
    let depth = args.next().unwrap_or(1).clamp(1, MAX_DEPTH.into()) as u8;

    let layout = generate::crypt(&mut rng::floor_rng(seed, depth), depth < MAX_DEPTH);
    let map = &layout.map;
    for y in 0..map.height() {
        let row: String = (0..map.width())
            .map(|x| match map.tile(tallow_core::Point::new(x, y)) {
                Tile::Wall => '#',
                Tile::Floor => '.',
                Tile::Door => '+',
                Tile::StairsDown => '>',
                Tile::StairsUp => '<',
                Tile::Brazier => '&',
            })
            .collect();
        println!("{row}");
    }
}
