use std::f32::consts::PI;

use glam::{vec2, Vec2};
use ordered_float::NotNan;

pub trait Optimizer {
    fn optimize(&self, f: impl Fn(Vec2) -> f32) -> (Vec2, f32);
}

/// Find minimum value of the function around a circle at the origin with given radius.
pub struct CircleBorder {
    pub radius: f32,
    pub samples: i32,
}

impl Optimizer for CircleBorder {
    fn optimize(&self, f: impl Fn(Vec2) -> f32) -> (Vec2, f32) {
        (0..self.samples)
            .map(|k| {
                let phi = 2.0 * PI / self.samples as f32 * (k as f32 + fastrand::f32());
                let x = self.radius * vec2(phi.cos(), phi.sin());
                let y = f(x);
                (x, y)
            })
            .min_by_key(|t| NotNan::new(t.1).unwrap())
            .unwrap()
    }
}

/// Solve minimum value of function using [Nelder-Mead method](https://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method).
/// If `bound` is set, it searches within the circle of given radius.
pub struct NelderMead {
    pub init: Vec<Vec2>,
    pub bound: Option<f32>,
}

impl Optimizer for NelderMead {
    fn optimize(&self, f: impl Fn(Vec2) -> f32) -> (Vec2, f32) {
        const ALPHA: f32 = 1.0;
        const GAMMA: f32 = 2.0;
        const RHO: f32 = 0.5;
        const SIGMA: f32 = 0.5;

        let clamp = |x: Vec2| match self.bound {
            Some(r) => x.clamp_length_max(r),
            None => x,
        };

        let n = self.init.len();
        let mut xs: Vec<_> = self.init.iter().map(|&x| (f(x), x)).collect();

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

        (xs[0].1, xs[0].0)
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;

    use super::*;

    #[test]
    fn test_nelder_mead() {
        fn rosenbrock(x: Vec2) -> f32 {
            (2.0f32.sqrt() - x.x).powi(2) + 100.0 * (x.y - x.x.powi(2)).powi(2)
        }

        let nelder_mead = NelderMead {
            init: vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
            bound: None,
        };

        let x_opt = nelder_mead.optimize(rosenbrock).0;
        assert_float_absolute_eq!((x_opt - vec2(2.0f32.sqrt(), 2.0)).length(), 0.0, 1e-2);
        dbg!(x_opt);

        let nelder_mead = NelderMead {
            init: vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
            bound: Some(6.0),
        };
        let x_opt = nelder_mead.optimize(rosenbrock).0;
        assert_float_absolute_eq!((x_opt - vec2(2.0f32.sqrt(), 2.0)).length(), 0.0, 1e-2);
        dbg!(x_opt);
    }
}
