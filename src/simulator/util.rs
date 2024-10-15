use glam::{vec2, Vec2};
use ndarray::Array2;
use num_traits::PrimInt;

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
pub fn poisson(lambda: f64) -> i32 {
    let mut y = 0;
    let mut x = fastrand::f64();
    let exp_lambda = (-lambda).exp();

    while x >= exp_lambda {
        x *= fastrand::f64();
        y += 1;
    }

    y
}

/// Calculate distance from line segment.
pub fn distance_from_line(point: Vec2, line: [Vec2; 2]) -> f32 {
    let a = point - line[0];
    let b = line[1] - line[0];
    let b_len2 = b.length_squared();

    if b_len2 == 0.0 {
        (a - line[0]).length()
    } else {
        let t = (a.dot(b) / b_len2).max(0.0).min(1.0);
        (t * b - a).length()
    }
}

/// Calculate coordinates of vertices of line with given width.
pub fn line_with_width(line: [Vec2; 2], width: f32) -> Vec<Vec2> {
    let a = (line[1] - line[0]).normalize();
    let b = vec2(a.y, -a.x) * 0.5 * width;

    vec![line[0] - b, line[0] + b, line[1] + b, line[1] - b]
}

/// Solve minimum value of function using [Nelder-Mead method](https://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method).
/// If `bound` is set, it searches within the circle of given radius.
pub fn nelder_mead(f: impl Fn(Vec2) -> f32, init: Vec<Vec2>, bound: Option<f32>) -> Vec2 {
    const ALPHA: f32 = 1.0;
    const GAMMA: f32 = 2.0;
    const RHO: f32 = 0.5;
    const SIGMA: f32 = 0.5;

    let clamp = |x: Vec2| match bound {
        Some(r) => x.clamp_length_max(r),
        None => x,
    };

    let n = init.len();
    let mut xs: Vec<_> = init.into_iter().map(|x| (f(x), x)).collect();

    for _it in 0..200 {
        xs.sort_by(|&a, &b| a.0.partial_cmp(&b.0).unwrap());
        let x_g = xs[..n - 1]
            .iter()
            .map(|(_, x)| x)
            .fold(Vec2::ZERO, |sum, x| sum + *x)
            / (n - 1) as f32;

        if (x_g - xs[n - 1].1).length_squared() < 1e-6 {
            break;
        }

        let x_r = clamp(x_g + ALPHA * (x_g - xs[n - 1].1));
        let y_r = f(x_r);

        if y_r < xs[0].0 {
            let x_e = clamp(x_g + GAMMA * (x_r - x_g));
            let y_e = f(x_e);
            if y_e < y_r {
                xs[n - 1] = (y_e, x_e);
            } else {
                xs[n - 1] = (y_r, x_r);
            }
        } else if y_r < xs[n - 2].0 {
            xs[n - 1] = (y_r, x_r);
        } else {
            let x_c = clamp(x_g + RHO * (xs[n - 1].1 - x_g));
            let y_c = f(x_c);
            if y_c < xs[n - 1].0 {
                xs[n - 1] = (y_c, x_c);
            } else {
                for i in 1..n {
                    let x_i = xs[0].1 + SIGMA * (xs[i].1 - xs[0].1);
                    let y_i = f(x_i);
                    xs[i] = (y_i, x_i);
                }
            }
        }
    }

    xs[0].1
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use glam::{vec2, Vec2};
    use ndarray::array;

    use crate::simulator::util::{bilinear, nelder_mead};

    use super::distance_from_line;

    #[test]
    fn test_distance_from_line() {
        let line = [vec2(1.0, 1.0), vec2(4.0, 1.0)];

        assert_float_absolute_eq!(distance_from_line(vec2(2.0, 3.0), line), 2.0);
        assert_float_absolute_eq!(distance_from_line(vec2(0.0, 0.25), line), 1.25);
    }

    #[test]
    fn test_nelder_mead() {
        fn rosenbrock(x: Vec2) -> f32 {
            (2.0f32.sqrt() - x.x).powi(2) + 100.0 * (x.y - x.x.powi(2)).powi(2)
        }

        let x_opt = nelder_mead(
            rosenbrock,
            vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
            None,
        );
        assert_float_absolute_eq!((x_opt - vec2(2.0f32.sqrt(), 2.0)).length(), 0.0, 1e-2);
        dbg!(x_opt);

        let x_opt = nelder_mead(
            rosenbrock,
            vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
            Some(6.0f32.sqrt()),
        );
        assert_float_absolute_eq!((x_opt - vec2(2.0f32.sqrt(), 2.0)).length(), 0.0, 1e-2);
        dbg!(x_opt);
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
