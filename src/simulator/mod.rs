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
            .map(|c| Wall {
                pos: c.vertice.clone(),
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
        let mut accels = vec![Vec2::zeros(); self.pedestrians.len()];

        accels.par_iter_mut().enumerate().for_each(|(i, acc)| {
            let a = &self.pedestrians[i];
            let e_a = (a.destination - a.pos).normalize();
            let f0_a = INV_TAU_A * (a.v0 * e_a - a.vel);
            let f_a = f0_a;
            *acc = f_a;
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
    pub pos: Vec<Vec2>,
}

/// Pedestrian instance
#[derive(Debug)]
pub struct Pedestrian {
    pub pos: Vec2,
    pub vel: Vec2,
    pub destination: Vec2,
    pub v0: f32,
    pub vel_prefered: Vec2,
    pub vmax: f32,
}

impl Default for Pedestrian {
    fn default() -> Self {
        Pedestrian {
            pos: Vec2::default(),
            vel: Vec2::default(),
            destination: Vec2::default(),
            v0: 1.34,
            vel_prefered: Vec2::default(),
            vmax: 1.3 * 1.34,
        }
    }
}
