pub mod diagnostic;
pub mod field;
pub mod scenario;
pub mod util;

use std::f32::consts::PI;

use crate::renderer::{fill::Instance, DrawCommand};
use field::Field;
use glam::{vec2, Vec2};
use ordered_float::NotNan;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use scenario::Scenario;

/// Simulator instance
#[derive(Default)]
pub struct Simulator {
    pub scenario: Scenario,
    pub field: Field,
    pub pedestrians: Vec<Pedestrian>,
    pub static_draw_commands: Vec<DrawCommand>,
}

impl Simulator {
    /// Create new simulator instance with scenario
    pub fn with_scenario(scenario: Scenario) -> Self {
        let field = Field::from_scenario(&scenario);
        let static_draw_commands = Self::create_static_draw_commands(&scenario);

        Simulator {
            scenario,
            field,
            pedestrians: vec![],
            static_draw_commands,
        }
    }

    fn create_static_draw_commands(scenario: &Scenario) -> Vec<DrawCommand> {
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

        vec![
            DrawCommand {
                mesh_id: 4,
                instances: obs_instances,
            },
            DrawCommand {
                mesh_id: 4,
                instances: wp_instances,
            },
        ]
    }

    pub fn spawn_pedestrians(&mut self) {
        for pedestrian in self.scenario.pedestrians.iter() {
            let [p_1, p_2] = self.scenario.waypoints[pedestrian.origin].line;
            let count = util::poisson(pedestrian.spawn.frequency / 10.0);

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
        const Q: i32 = 16;
        const R: f32 = 0.5;

        self.pedestrians
            .par_iter()
            .enumerate()
            .filter(|(_, ped)| ped.active)
            .map(|(i, ped)| {
                let active = self.field.get_potential(ped.destination, ped.pos) > 2.0;

                let position = (0..Q)
                    .map(|k| {
                        let phi = 2.0 * PI / Q as f32 * (k as f32 + fastrand::f32());
                        let x_k = ped.pos + R * vec2(phi.cos(), phi.sin());

                        let p = self.get_potential(i, ped.destination, x_k);

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

    fn get_potential(&self, pedestrian_id: usize, waypoint_id: usize, position: Vec2) -> f32 {
        /// Pedestrian torso
        const G_P: f32 = 0.4;
        const G_P_HALF: f32 = G_P / 2.0;

        // Parameters on pedestrians
        const MU_P: f32 = 1000.0;
        const NU_P: f32 = 0.4;
        const A_P: f32 = 1.0;
        const B_P: f32 = 0.2;
        const H_P: f32 = 1.0;

        // Parameters on obstacles
        const MU_O: f32 = 10000.0;
        const NU_O: f32 = 0.2;
        const A_O: f32 = 3.0;
        const B_O: f32 = 2.0;
        const H_O: f32 = 6.0;

        let p_field = 0.2 * self.field.get_potential(waypoint_id, position);

        if p_field < 4.0 {
            return p_field;
        }

        let p_pedestrians: f32 = self
            .pedestrians
            .iter()
            .take(pedestrian_id)
            .chain(self.pedestrians.iter().skip(pedestrian_id + 1))
            .map(|ped| {
                let delta = (position - ped.pos).length();
                if delta > G_P + H_P {
                    0.0
                } else if delta <= G_P {
                    MU_P
                } else {
                    NU_P * (-A_P * delta.powf(B_P)).exp()
                }
            })
            .sum();

        let p_obstacles: f32 = self
            .scenario
            .obstacles
            .iter()
            .map(|obs| {
                let delta = util::distance_from_line(position, obs.line);
                if delta > H_O {
                    0.0
                } else if delta < G_P_HALF {
                    MU_O
                } else {
                    NU_O * (-A_O * delta.powf(B_O)).exp()
                }
            })
            .sum();

        p_field + p_pedestrians + p_obstacles
    }
}

/// Pedestrian instance
#[derive(Debug)]
pub struct Pedestrian {
    pub active: bool,
    pub pos: Vec2,
    pub destination: usize,
}

impl Default for Pedestrian {
    fn default() -> Self {
        // default parameters from https://arxiv.org/abs/cond-mat/9805244

        // let v0 = fastrand_contrib::f32_normal_approx(1.34, 0.26);

        Pedestrian {
            active: true,
            pos: Vec2::default(),
            destination: 0,
        }
    }
}
