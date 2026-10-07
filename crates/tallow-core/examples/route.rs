use tallow_core::{Direction, Point, World, map::path};
fn main() {
    let a: Vec<i64> = std::env::args()
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect();
    let (seed, depth, tx, ty, stop) = (
        a[0] as u64,
        a[1] as u8,
        a[2] as i32,
        a[3] as i32,
        a[4] as u32,
    );
    let mut w = World::new(seed);
    w.dev_skip_to(depth);
    let goal = Point::new(tx, ty);
    let dist = path::distances(w.map(), goal);
    let mut p = w.player().pos;
    let mut keys = String::new();
    while dist.at(p).unwrap() > stop {
        let here = dist.at(p).unwrap();
        let d = Direction::ALL
            .into_iter()
            .find(|&d| dist.at(p + d).is_some_and(|n| n < here))
            .unwrap();
        keys.push(match d {
            Direction::N => 'k',
            Direction::S => 'j',
            Direction::E => 'l',
            Direction::W => 'h',
            Direction::NE => 'u',
            Direction::NW => 'y',
            Direction::SE => 'n',
            Direction::SW => 'b',
        });
        p = p + d;
    }
    println!("{keys}");
}
