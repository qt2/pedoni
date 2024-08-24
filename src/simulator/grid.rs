use std::fmt::Debug;

use glam::DVec2;

pub struct Grid {
    cells: Vec<u8>,
    row: i32,
    column: i32,
    unit: f64,
}

impl Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.cells.chunks(self.row as usize))
            .finish()
    }
}

impl Grid {
    pub fn new(size: DVec2, unit: f64) -> Self {
        let row = (size.x / unit).ceil() as i32;
        let column = (size.y / unit).ceil() as i32;
        let cells = vec![0; (row * column) as usize];

        Grid {
            cells,
            row,
            column,
            unit,
        }
    }

    pub fn point_to_index(&self, point: DVec2) -> (i32, i32) {
        let i = (point.x / self.unit).floor() as i32;
        let j = (point.y / self.unit).floor() as i32;
        (i, j)
    }

    pub fn stroke_line(&mut self, mut a: DVec2, mut b: DVec2, value: u8) -> Vec<(i32, i32)> {
        let mut stroked = Vec::new();

        if a.y > b.y {
            std::mem::swap(&mut a, &mut b);
        }

        let (mut u1, v1) = self.point_to_index(a);
        let (u2, v2) = self.point_to_index(b);

        for v in v1..v2 {
            let y_cross = (v + 1) as f64 * self.unit;
            let t = (y_cross - a.y) / (b.y - a.y);
            let x_cross = a.x + (b.x - a.x) * t;
            let u_cross_f = x_cross / self.unit;
            let u_cross = if a.x < b.x {
                u_cross_f.ceil() as i32 - 1
            } else {
                u_cross_f.floor() as i32
            };

            let u_range = if a.x < b.x {
                u1..=u_cross
            } else {
                u_cross..=u1
            };

            for u in u_range {
                self.cells[(u + v * self.row) as usize] = value;
                stroked.push((u, v));
            }
            u1 = u_cross;

            if u_cross_f.fract() < 1e-16 {
                u1 += if a.x < b.x { 1 } else { -1 };
            }
        }

        let u_range = if a.x < b.x { u1..=u2 } else { u2..=u1 };
        for u in u_range {
            self.cells[(u + v2 * self.row) as usize] = value;
            stroked.push((u, v2));
        }

        stroked
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::*;

    #[test]
    fn test_point_to_index() {
        let grid = Grid::new(dvec2(5.0, 5.0), 1.0);
        assert_eq!(grid.point_to_index(dvec2(0.0, 0.0)), (0, 0));
        assert_eq!(grid.point_to_index(dvec2(4.9, 4.9)), (4, 4));
        assert_eq!(grid.point_to_index(dvec2(2.0, 3.7)), (2, 3));

        let grid = Grid::new(dvec2(5.0, 6.0), 0.5);
        assert_eq!(grid.point_to_index(dvec2(2.0, 3.7)), (4, 7));
    }

    #[test]
    fn test_stroke_line() {
        let mut grid = Grid::new(dvec2(5.0, 5.0), 1.0);
        grid.stroke_line(dvec2(0.5, 3.5), dvec2(2.5, 2.7), 1);

        let cells = grid.cells;
        assert_eq!(cells[2 * 5 + 0], 0);
        assert_eq!(cells[2 * 5 + 1], 1);
        assert_eq!(cells[2 * 5 + 2], 1);

        let mut grid = Grid::new(dvec2(1.5, 2.0), 0.5);
        grid.stroke_line(dvec2(1.5, 0.5) * 0.5, dvec2(0.5, 3.5) * 0.5, 2);
        println!("{grid:?}");

        let cells = grid.cells;
        assert_eq!(cells[1 * 3 + 0], 0);
        assert_eq!(cells[1 * 3 + 1], 2);
        assert_eq!(cells[2 * 3 + 0], 2);
        assert_eq!(cells[2 * 3 + 1], 0);
    }
}
