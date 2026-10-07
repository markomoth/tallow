//! Prints the vi keys that walk from a floor's arrival stair to within `stop`
//! steps of a tile. For driving the real binary in tmux.
//!
//! cargo run -p tallow-core --example route -- <seed> <depth> <x> <y> <stop>

use tallow_core::{Direction, Point, Tile, World, map::path};

fn main() {
    let args: Vec<i64> = std::env::args()
        .skip(1)
        .map(|s| s.parse().expect("numbers"))
        .collect();
    let [seed, depth, x, y, stop] = args[..] else {
        panic!("usage: route <seed> <depth> <x> <y> <stop>");
    };
    let mut world = World::new(seed as u64);
    world.dev_skip_to(depth as u8);
    let dist = path::distances(world.map(), Point::new(x as i32, y as i32));
    let mut p = world.player().pos;
    let mut keys = String::new();
    while let Some(here) = dist.at(p).filter(|&d| d > stop as u32) {
        let d = Direction::ALL
            .into_iter()
            .find(|&d| dist.at(p + d).is_some_and(|n| n < here))
            .expect("a step closer exists");
        let key = match d {
            Direction::N => 'k',
            Direction::S => 'j',
            Direction::E => 'l',
            Direction::W => 'h',
            Direction::NE => 'u',
            Direction::NW => 'y',
            Direction::SE => 'n',
            Direction::SW => 'b',
        };
        // A shut door takes one step to open and another to go through.
        if world.map().tile(p + d) == Tile::DoorClosed {
            keys.push(key);
        }
        keys.push(key);
        p = p + d;
    }
    println!("{keys}");
}
