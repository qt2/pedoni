use glam::{vec2, Vec2};
use ndarray::Array2;
use num_traits::PrimInt;
use ocl::prm::Float2;

/// Index struct for [`ndarray::Array2`]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index {
    pub y: i32,
    pub x: i32,
}

impl Index {
    pub fn new<T: PrimInt>(x: T, y: T) -> Self {
        Index {
            x: x.to_i32().unwrap(),
            y: y.to_i32().unwrap(),
        }
    }

    pub fn add<T: PrimInt>(self, x: T, y: T) -> Self {
        Index {
            x: self.x + x.to_i32().unwrap(),
            y: self.y + y.to_i32().unwrap(),
        }
    }
}

unsafe impl ndarray::NdIndex<ndarray::Ix2> for Index {
    fn index_checked(&self, dim: &ndarray::Ix2, strides: &ndarray::Ix2) -> Option<isize> {
        if self.x.is_negative() || self.y.is_negative() {
            None
        } else {
            (self.y as usize, self.x as usize).index_checked(dim, strides)
        }
    }

    fn index_unchecked(&self, strides: &ndarray::Ix2) -> isize {
        (self.y as usize, self.x as usize).index_unchecked(strides)
    }
}

/// Interpolate grid using bilinear interpolation.
pub fn bilinear(grid: &Array2<f32>, pos: Vec2) -> f32 {
    const FMAX: f32 = 1e12;

    let base = pos.floor();
    let t = pos - base;
    let s = Vec2::ONE - t;
    let ix = Index::new(base.x as i32, base.y as i32);

    let mut y = 0.0;
    y += s.y * s.x * grid.get(ix).cloned().unwrap_or(FMAX);
    y += s.y * t.x * grid.get(ix.add(1, 0)).cloned().unwrap_or(FMAX);
    y += t.y * s.x * grid.get(ix.add(0, 1)).cloned().unwrap_or(FMAX);
    y += t.y * t.x * grid.get(ix.add(1, 1)).cloned().unwrap_or(FMAX);
    y
}

/// Spawn a random integer based on Poisson distribution.
pub fn poisson(lambda: f64, rng: &mut fastrand::Rng) -> i32 {
    let mut y = 0;
    let mut x = rng.f64();
    let exp_lambda = (-lambda).exp();

    while x >= exp_lambda {
        x *= fastrand::f64();
        y += 1;
    }

    y
}

/// Calculate distance from line segment.
pub fn distance_from_line(point: Vec2, line: [Vec2; 2]) -> Vec2 {
    let a = point - line[0];
    let b = line[1] - line[0];
    let b_len2 = b.length_squared();

    if b_len2 == 0.0 {
        a - line[0]
    } else {
        let t = (a.dot(b) / b_len2).max(0.0).min(1.0);
        a - t * b
    }
}

/// Calculate coordinates of vertices of line with given width.
pub fn line_with_width(line: [Vec2; 2], width: f32) -> Vec<Vec2> {
    let a = (line[1] - line[0]).normalize();
    let b = vec2(a.y, -a.x) * 0.5 * width;

    vec![line[0] - b, line[0] + b, line[1] + b, line[1] - b]
}

pub trait ToGlam {
    type T;
    fn to_glam(self) -> Self::T;
}

impl ToGlam for Float2 {
    type T = Vec2;
    fn to_glam(self) -> Vec2 {
        let array: [f32; 2] = self.into();
        Vec2::from(array)
    }
}

pub trait ToOcl {
    type T;
    fn to_ocl(self) -> Self::T;
}

impl ToOcl for Vec2 {
    type T = Float2;
    fn to_ocl(self) -> Float2 {
        Float2::from(self.to_array())
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use glam::vec2;
    use ndarray::array;

    use crate::util::bilinear;

    use super::distance_from_line;

    #[test]
    fn test_distance_from_line() {
        let line = [vec2(1.0, 1.0), vec2(4.0, 1.0)];

        assert_float_absolute_eq!(distance_from_line(vec2(2.0, 3.0), line).length(), 2.0);
        assert_float_absolute_eq!(distance_from_line(vec2(0.0, 0.25), line).length(), 1.25);
    }

    #[test]
    fn test_bilinear() {
        let grid = array![[1.0, 0.0, 4.0], [3.0, 1.0, -1.0],];
        assert_float_absolute_eq!(bilinear(&grid, vec2(0.0, 0.0)), 1.0);
        assert_float_absolute_eq!(bilinear(&grid, vec2(0.5, 0.0)), 0.5);
        assert_float_absolute_eq!(bilinear(&grid, vec2(0.0, 0.25)), 1.5);
        assert_float_absolute_eq!(bilinear(&grid, vec2(0.5, 0.5)), 1.25);
    }
}
