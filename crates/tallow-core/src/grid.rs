//! A generic rectangular grid, used for tiles, visibility, memory and light.

use crate::geom::Point;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<T> {
    width: i32,
    height: i32,
    cells: Vec<T>,
}

impl<T: Clone> Grid<T> {
    pub fn new(width: i32, height: i32, fill: T) -> Self {
        assert!(width > 0 && height > 0, "grid must have a positive size");
        Self {
            width,
            height,
            cells: vec![fill; (width * height) as usize],
        }
    }

    /// Resets every cell to `value`.
    pub fn fill(&mut self, value: T) {
        self.cells.fill(value);
    }
}

impl<T> Grid<T> {
    pub const fn width(&self) -> i32 {
        self.width
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    pub const fn in_bounds(&self, p: Point) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    pub fn get(&self, p: Point) -> Option<&T> {
        self.index(p).map(|i| &self.cells[i])
    }

    pub fn get_mut(&mut self, p: Point) -> Option<&mut T> {
        self.index(p).map(|i| &mut self.cells[i])
    }

    /// Sets a cell. Writes outside the bounds are ignored.
    pub fn set(&mut self, p: Point, value: T) {
        if let Some(cell) = self.get_mut(p) {
            *cell = value;
        }
    }

    /// Every point in the grid, row by row.
    pub fn points(&self) -> impl Iterator<Item = Point> + use<T> {
        let (w, h) = (self.width, self.height);
        (0..h).flat_map(move |y| (0..w).map(move |x| Point::new(x, y)))
    }

    fn index(&self, p: Point) -> Option<usize> {
        self.in_bounds(p).then(|| (p.y * self.width + p.x) as usize)
    }
}

impl<T: Copy + Default> Grid<T> {
    /// The value at `p`, or the default outside the bounds.
    pub fn at(&self, p: Point) -> T {
        self.get(p).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_bounds() {
        let mut g = Grid::new(3, 2, 0u8);
        g.set(Point::new(2, 1), 7);
        g.set(Point::new(9, 9), 1);
        assert_eq!(g.at(Point::new(2, 1)), 7);
        assert_eq!(g.at(Point::new(-1, 0)), 0);
        assert_eq!(g.points().count(), 6);
    }
}
