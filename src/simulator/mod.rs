pub mod environment;
pub mod models;
pub mod scenario;

use std::f32::consts::PI;

use crate::renderer::{fill::Instance, DrawCommand};
use environment::Environment;
use glam::{vec2, Vec2};
use ordered_float::NotNan;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use scenario::Scenario;

const DELTA_T: f32 = 0.1;
const TAU_A: f32 = 0.5;
const INV_TAU_A: f32 = 1.0 / TAU_A;

/// Simulator instance
#[derive(Default)]
pub struct Simulator {
    pub scenario: Scenario,
    pub environment: Environment,
    pub pedestrians: Vec<Pedestrian>,
    pub static_draw_commands: Vec<DrawCommand>,
}

impl Simulator {
    /// Create new simulator instance with scenario
    pub fn with_scenario(scenario: Scenario) -> Self {
        let environment = Environment::from_scenario(&scenario);

        let obs_instances = scenario
            .obstacles
            .iter()
            .map(|obstacle| Instance::line_segment(obstacle.line, 1.0, [255, 255, 255, 64]))
            .collect();
        let wp_instances = scenario
            .waypoints
            .iter()
            .map(|wp| Instance::line_segment(wp.line, 1.0, [255, 255, 0, 255]))
            .collect();

        let static_draw_commands = vec![
            DrawCommand {
                mesh_id: 4,
                instances: obs_instances,
            },
            DrawCommand {
                mesh_id: 4,
                instances: wp_instances,
            },
        ];

        Simulator {
            scenario,
            environment,
            pedestrians: vec![],
            static_draw_commands,
        }
    }

    pub fn tick(&mut self) {
        self.spawn_pedestrians();
    }

    pub fn spawn_pedestrians(&mut self) {
        for pedestrian in self.scenario.pedestrians.iter() {
            let [p_1, p_2] = self.scenario.waypoints[pedestrian.origin].line;
            let count = poisson(pedestrian.spawn.frequency / 10.0);

            for _ in 0..count {
                let pos = p_1.lerp(p_2, fastrand::f32());
                self.pedestrians.push(Pedestrian {
                    pos,
                    destination: pedestrian.destination,
                    ..Default::default()
                })
            }
        }
    }

    pub fn calc_next_state(&self) -> Vec<(Vec2, bool)> {
        const Q: i32 = 8;
        const R: f32 = 1.0;

        self.pedestrians
            .par_iter()
            .filter(|ped| ped.active)
            .map(|ped| {
                let active = self.environment.get_potential(ped.destination, ped.pos) > 2.0;

                let position = (0..Q)
                    .map(|k| {
                        let phi = 2.0 * PI / Q as f32 * (k as f32 + fastrand::f32());
                        let x_k = ped.pos + R * vec2(phi.cos(), phi.sin());
                        let p = self.environment.get_potential(ped.destination, x_k);
                        (NotNan::new(p).unwrap(), x_k)
                    })
                    .min_by_key(|t| t.0)
                    .unwrap()
                    .1;

                (position, active)
            })
            .collect()
    }

    pub fn apply_next_state(&mut self, state: Vec<(Vec2, bool)>) {
        self.pedestrians
            .iter_mut()
            .filter(|ped| ped.active)
            .zip(state)
            .for_each(|(ped, (pos, active))| {
                ped.pos = pos;
                ped.active = active;
            });
    }

    // pub fn calc_acceleration(&self) -> Vec<Vec2> {
    //     let es: Vec<_> = self
    //         .pedestrians
    //         .iter()
    //         .map(|p| {
    //             (p.destination - p.pos)
    //                 .try_normalize()
    //                 .unwrap_or(vec2(1.0, 0.0))
    //         })
    //         .collect();

    //     let mut accels = vec![Vec2::ZERO; self.pedestrians.len()];

    //     accels.par_iter_mut().enumerate().for_each(|(i, acc)| {
    //         let a = &self.pedestrians[i];
    //         let e_a = es[i];

    //         // Acceleration term
    //         let f0_a = INV_TAU_A * (a.v0 * e_a - a.vel);
    //         *acc += f0_a;

    //         // Repulsive force from other pedestrians
    //         for (j, b) in self.pedestrians.iter().enumerate() {
    //             if i == j {
    //                 continue;
    //             }
    //             let r_ab = a.pos - b.pos;
    //             let r_ab_mag = r_ab.length();
    //             let move_b = b.vel * 2.0;
    //             let r_ab_mv = r_ab - move_b;
    //             let r_ab_mv_mag = r_ab_mv.length();

    //             let b = 0.5 * ((r_ab_mag + r_ab_mv_mag).powi(2) - move_b.length_squared()).sqrt();
    //             let grad_b =
    //                 (r_ab_mag + r_ab_mv_mag) * (r_ab / r_ab_mag + r_ab_mv / r_ab_mv_mag) * 0.25 / b;
    //             let f_ab = 2.1 / 0.3 * (-b / 0.3).exp() * grad_b;
    //             *acc += f_ab;
    //         }
    //     });

    //     accels
    // }

    // /// Tick and update environment
    // pub fn tick(&mut self, accels: Vec<Vec2>) {
    //     self.pedestrians
    //         .par_iter_mut()
    //         .zip(&accels)
    //         .for_each(|(a, acc)| {
    //             a.vel_prefered += *acc * DELTA_T;
    //             a.vel = a.vel_prefered.clamp_length_max(a.vmax);
    //             a.pos += a.vel * DELTA_T;
    //         });
    // }
}

fn poisson(lambda: f64) -> i32 {
    let mut y = 0;
    let mut x = fastrand::f64();
    let exp_lambda = (-lambda).exp();

    while x >= exp_lambda {
        x *= fastrand::f64();
        y += 1;
    }

    y
}

/// Pedestrian instance
#[derive(Debug)]
pub struct Pedestrian {
    pub active: bool,
    pub pos: Vec2,
    pub vel: Vec2,
    pub vel_prefered: Vec2,
    // pub destination: Vec2,
    pub destination: usize,
    pub v0: f32,
    pub vmax: f32,
}

impl Default for Pedestrian {
    fn default() -> Self {
        // default parameters from https://arxiv.org/abs/cond-mat/9805244

        let v0 = fastrand_contrib::f32_normal_approx(1.34, 0.26);

        Pedestrian {
            active: true,
            pos: Vec2::default(),
            vel: Vec2::default(),
            vel_prefered: Vec2::default(),
            // destination: Vec2::default(),
            destination: 0,
            v0,
            vmax: v0 * 1.3,
        }
    }
}
