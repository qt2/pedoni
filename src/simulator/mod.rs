pub mod diagnostic;
pub mod field;
mod kernels;
pub mod scenario;
pub mod util;

use crate::{
    // renderer::{fill::Instance, DrawCommand},
    State,
    STATE,
};
use field::Field;
use glam::{vec2, Vec2};
use ndarray::Array2;

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use scenario::Scenario;
use thin_vec::ThinVec;
use util::Index;

/// Simulator instance
#[derive(Default)]
pub struct Simulator {
    pub scenario: Scenario,
    pub field: Field,
    pub pedestrians: Vec<Pedestrian>,
    // pub static_draw_commands: Vec<DrawCommand>,
    pub neighbor_grid: Option<Array2<ThinVec<u32>>>,
    pub neighbor_grid_belong: Option<Vec<Index>>,
}

impl Simulator {
    /// Create new simulator instance with scenario
    pub fn with_scenario(scenario: Scenario) -> Self {
        let field = Field::from_scenario(&scenario);
        // let static_draw_commands = Self::create_static_draw_commands(&scenario);

        Simulator {
            scenario,
            field,
            pedestrians: vec![],
            // static_draw_commands,
            neighbor_grid: None,
            neighbor_grid_belong: None,
        }
    }

    // fn create_static_draw_commands(scenario: &Scenario) -> Vec<DrawCommand> {
    //     let obs_instances = scenario
    //         .obstacles
    //         .iter()
    //         .map(|obstacle| {
    //             Instance::line_segment(obstacle.line, obstacle.width, [255, 255, 255, 64])
    //         })
    //         .collect();
    //     let wp_instances = scenario
    //         .waypoints
    //         .iter()
    //         .map(|wp| Instance::line_segment(wp.line, wp.width, [255, 255, 0, 255]))
    //         .collect();

    //     vec![
    //         DrawCommand {
    //             mesh_id: 4,
    //             instances: obs_instances,
    //         },
    //         DrawCommand {
    //             mesh_id: 4,
    //             instances: wp_instances,
    //         },
    //     ]
    // }

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

        let State {
            use_neighbor_grid,
            neighbor_grid_unit,
            ..
        } = *STATE.lock().unwrap();

        (self.neighbor_grid, self.neighbor_grid_belong) = if use_neighbor_grid {
            let shape = (self.scenario.field.size / neighbor_grid_unit).ceil();
            let shape = (shape.y as usize, shape.x as usize);
            let mut grid = Array2::from_elem(shape, ThinVec::new());
            let mut belong = vec![Index::default(); self.pedestrians.len()];

            for (i, pedestrian) in self
                .pedestrians
                .iter()
                .enumerate()
                .filter(|(_, ped)| ped.active)
            {
                let ix = (pedestrian.pos / neighbor_grid_unit).ceil().as_ivec2();
                let ix = Index::new(ix.x, ix.y);
                if let Some(neighbors) = grid.get_mut(ix) {
                    neighbors.push(i as u32);
                    belong[i] = ix;
                }
            }

            (Some(grid), Some(belong))
        } else {
            (None, None)
        };
    }

    pub fn calc_next_state(&self) -> Vec<(Vec2, bool)> {
        const R: f32 = 0.3;

        self.pedestrians
            .par_iter()
            .enumerate()
            .filter(|(_, ped)| ped.active)
            .map(|(i, ped)| {
                let active = self.field.get_potential(ped.destination, ped.pos) > 2.0;

                // const Q: i32 = 16;

                // let (potential, position) = (0..Q)
                //     .map(|k| {
                //         let phi = 2.0 * PI / Q as f32 * (k as f32 + fastrand::f32());
                //         let x_k = ped.pos + R * vec2(phi.cos(), phi.sin());

                //         let p = self.get_potential(i, ped.destination, x_k);

                //         (NotNan::new(p).unwrap(), x_k)
                //     })
                //     .min_by_key(|t| t.0)
                //     .unwrap();

                let f = |x: Vec2| self.get_potential(i, ped.destination, ped.pos + x);

                let position = util::nelder_mead(
                    f,
                    vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
                    Some(R),
                ) + ped.pos;

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

        if p_field < 1.0 {
            return p_field;
        }

        let potential_ped_from_distance = |delta: f32| {
            if delta > G_P + H_P {
                0.0
            } else if delta <= G_P {
                MU_P
            } else {
                NU_P * (-A_P * delta.powf(B_P)).exp()
            }
        };

        let p_pedestrians = if let Some(ref grid) = self.neighbor_grid {
            let ix = self.neighbor_grid_belong.as_ref().unwrap()[pedestrian_id];
            let mut potential = 0.0;

            for j in -1..=1 {
                for i in -1..=1 {
                    let ix = ix.add(i, j);
                    if let Some(neighbors) = grid.get(ix) {
                        for &id in neighbors.iter().filter(|i| **i != pedestrian_id as u32) {
                            let delta = (position - self.pedestrians[id as usize].pos).length();
                            potential += potential_ped_from_distance(delta);
                        }
                    }
                }
            }

            potential
        } else {
            self.pedestrians
                .iter()
                .take(pedestrian_id)
                .chain(self.pedestrians.iter().skip(pedestrian_id + 1))
                .map(|ped| {
                    let delta = (position - ped.pos).length();
                    potential_ped_from_distance(delta)
                })
                .sum()
        };
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
