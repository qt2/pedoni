pub mod scenario;

use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use self::scenario::Scenario;
use crate::Vec2;

const DELTA_T: f32 = 0.1;
const TAU_A: f32 = 0.5;
const INV_TAU_A: f32 = 1.0 / TAU_A;

/// Simulator instance
#[derive(Debug, Default)]
pub struct Simulator {
    pub scenario: Scenario,
    pub walls: Vec<Wall>,
    pub pedestrians: Vec<Pedestrian>,
}

impl Simulator {
    /// Create new simulator instance with scenario
    pub fn with_scenario(scenario: Scenario) -> Self {
        let walls = scenario
            .walls
            .iter()
            .flat_map(|c| {
                c.polygon.windows(2).map(|polygon| Wall {
                    polygon: polygon.try_into().unwrap(),
                })
            })
            .collect();

        Simulator {
            scenario,
            walls,
            pedestrians: vec![],
        }
    }

    pub fn with_random() -> Self {
        let pedestrians = (0..100)
            .map(|_| Pedestrian {
                pos: Vec2::new(fastrand::f32(), fastrand::f32()) * 25.,
                destination: Vec2::new(fastrand::f32(), fastrand::f32()) * 25.,
                ..Default::default()
            })
            .collect();

        Simulator {
            pedestrians,
            ..Default::default()
        }
    }

    pub fn calc_acceleration(&self) -> Vec<Vec2> {
        let es: Vec<_> = self
            .pedestrians
            .iter()
            .map(|p| (p.destination - p.pos).normalize())
            .collect();

        let mut accels = vec![Vec2::zeros(); self.pedestrians.len()];

        accels.par_iter_mut().enumerate().for_each(|(i, acc)| {
            let a = &self.pedestrians[i];
            let e_a = es[i];

            // Acceleration term
            let f0_a = INV_TAU_A * (a.v0 * e_a - a.vel);
            *acc += f0_a;

            // Repulsive force from other pedestrians
            for (j, b) in self.pedestrians.iter().enumerate() {
                if i == j {
                    continue;
                }
                let r_ab = a.pos - b.pos;
                let r_ab_mag = r_ab.magnitude();
                let move_b = b.vel * 2.0;
                let r_ab_mv = r_ab - move_b;
                let r_ab_mv_mag = r_ab_mv.magnitude();

                let b =
                    0.5 * ((r_ab_mag + r_ab_mv_mag).powi(2) - move_b.magnitude_squared()).sqrt();
                let grad_b =
                    (r_ab_mag + r_ab_mv_mag) * (r_ab / r_ab_mag + r_ab_mv / r_ab_mv_mag) * 0.25 / b;
                let f_ab = 2.1 / 0.3 * (-b / 0.3).exp() * grad_b;
                *acc += f_ab;
            }
        });

        accels
    }

    /// Tick and update environment
    pub fn tick(&mut self, accels: Vec<Vec2>) {
        self.pedestrians
            .par_iter_mut()
            .zip(&accels)
            .for_each(|(a, acc)| {
                a.vel_prefered += acc * DELTA_T;
                a.vel = a.vel_prefered.cap_magnitude(a.vmax);
                a.pos += a.vel * DELTA_T;
            });
    }
}

/// Wall instance
#[derive(Debug)]
pub struct Wall {
    pub polygon: [Vec2; 2],
}

/// Pedestrian instance
#[derive(Debug)]
pub struct Pedestrian {
    pub pos: Vec2,
    pub vel: Vec2,
    pub vel_prefered: Vec2,
    pub destination: Vec2,
    pub v0: f32,
    pub vmax: f32,
}

impl Default for Pedestrian {
    fn default() -> Self {
        // default parameters from https://arxiv.org/abs/cond-mat/9805244

        let v0 = fastrand_contrib::f32_normal_approx(1.34, 0.26);

        Pedestrian {
            pos: Vec2::default(),
            vel: Vec2::default(),
            vel_prefered: Vec2::default(),
            destination: Vec2::default(),
            v0,
            vmax: v0 * 1.3,
        }
    }
}
