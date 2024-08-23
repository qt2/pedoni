use glam::DVec2;

pub struct Grid {
    cells: Vec<u8>,
    row: usize,
    column: usize,
    unit: f64,
}

impl Grid {
    pub fn new(size: DVec2, unit: f64) -> Self {
        let row = (size.y / unit).ceil() as usize;
        let column = (size.x / unit).ceil() as usize;
        let cells = vec![0; row * column];

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

    pub fn stroke_line(&mut self, a: DVec2, b: DVec2, value: u8) {
        let (x1, y1) = self.point_to_index(a);
        let (x2, y2) = self.point_to_index(b);

        let (mut x, mut y) = (x1, y1);
        let step_y = if y2 >= y1 { 1 } else { -1 };

        loop {
            if y == y2 {
                break;
            }
            y += step_y;
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::*;

    #[test]
    fn test_stroke_line() {
        let mut grid = Grid::new(dvec2(5.0, 5.0), 1.0);
        grid.stroke_line(dvec2(0.5, 3.5), dvec2(3.5, 2.7), 1);

        let cells = grid.cells;
        assert_eq!(cells[2 * 5 + 1], 0);
        assert_eq!(cells[2 * 5 + 2], 1);
        assert_eq!(cells[2 * 5 + 3], 1);
    }
}
