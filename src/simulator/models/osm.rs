use std::f32::consts::PI;

use glam::{vec2, Vec2};
use ordered_float::NotNan;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::simulator::{
    optim::{NelderMead, Optimizer},
    util, Simulator,
};

use super::PedestrianModel;

const R: f32 = 0.3;

pub struct OptimalStepsModel {
    pedestrians: Vec<super::Pedestrian>,
}

impl Default for OptimalStepsModel {
    fn default() -> Self {
        OptimalStepsModel::new()
    }
}

impl OptimalStepsModel {
    pub fn new() -> Self {
        OptimalStepsModel {
            pedestrians: Vec::new(),
        }
    }

    fn calc_potential(
        &self,
        pos: Vec2,
        sim: &Simulator,
        destination: usize,
        self_id: usize,
    ) -> f32 {
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

        let p_field = 0.2 * sim.field.get_potential(destination, pos);

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

        let p_pedestrians = if let Some(ref grid) = sim.neighbor_grid {
            let ix = sim.neighbor_grid_belong.as_ref().unwrap()[self_id];
            let mut potential = 0.0;

            for j in -1..=1 {
                for i in -1..=1 {
                    let ix = ix.add(i, j);
                    if let Some(neighbors) = grid.get(ix) {
                        for &id in neighbors.iter().filter(|i| **i != self_id as u32) {
                            let delta = (pos - self.pedestrians[id as usize].pos).length();
                            potential += potential_ped_from_distance(delta);
                        }
                    }
                }
            }

            potential
        } else {
            self.pedestrians
                .iter()
                .take(self_id)
                .chain(self.pedestrians.iter().skip(self_id + 1))
                .map(|ped| {
                    let delta = (pos - ped.pos).length();
                    potential_ped_from_distance(delta)
                })
                .sum()
        };
        let p_obstacles: f32 = sim
            .scenario
            .obstacles
            .iter()
            .map(|obs| {
                let delta = util::distance_from_line(pos, obs.line);
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

impl PedestrianModel for OptimalStepsModel {
    fn spawn_pedestrians(&mut self, mut pedestrians: Vec<super::Pedestrian>) {
        self.pedestrians.append(&mut pedestrians);
    }

    fn calc_next_state(&self, sim: &Simulator) -> Box<dyn std::any::Any> {
        let optimizer = NelderMead {
            init: vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
            bound: Some(R),
        };

        let state: Vec<_> = self
            .pedestrians
            .par_iter()
            .enumerate()
            .filter(|(_, ped)| ped.active)
            .map(|(id, ped)| {
                let active = sim.field.get_potential(ped.destination, ped.pos) > 2.0;

                let f = |x: Vec2| self.calc_potential(ped.pos + x, sim, ped.destination, id);
                let position = optimizer.optimize(f).0 + ped.pos;

                (position, active)
            })
            .collect();
        Box::new(state)
    }

    fn apply_next_state(&mut self, next_state: Box<dyn std::any::Any>) {
        let next_state = *next_state.downcast::<Vec<(Vec2, bool)>>().unwrap();

        self.pedestrians
            .iter_mut()
            .filter(|ped| ped.active)
            .zip(next_state)
            .for_each(|(ped, (pos, active))| {
                ped.pos = pos;
                ped.active = active;
            });
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        self.pedestrians.clone()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.pedestrians.len() as i32
    }
}

pub fn optimize_around_circle(f: impl Fn(Vec2) -> f32, r: f32) -> (Vec2, f32) {
    const Q: i32 = 16;

    (0..Q)
        .map(|k| {
            let phi = 2.0 * PI / Q as f32 * (k as f32 + fastrand::f32());
            let x = r * vec2(phi.cos(), phi.sin());
            let y = f(x);
            (x, y)
        })
        .min_by_key(|t| NotNan::new(t.1).unwrap())
        .unwrap()
}
