use std::sync::Mutex;

use glam::{IVec2, Vec2};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::simulator::{
    optim::{CircleBorder, Optimizer},
    util::Index,
    NeighborGrid, Simulator,
};

use super::PedestrianModel;

const R: f32 = 0.1;

pub struct OptimalStepsModel {
    pedestrians: Vec<super::Pedestrian>,
    neighbor_grid: Option<NeighborGrid>,
    neighbor_grid_indices: Vec<u32>,
    next_state: Mutex<Vec<(Vec2, bool)>>,
}

impl PedestrianModel for OptimalStepsModel {
    fn new(
        args: &crate::args::Args,
        scenario: &crate::simulator::scenario::Scenario,
        _field: &crate::simulator::field::Field,
    ) -> Self {
        let neighbor_grid = (!args.no_grid)
            .then(|| NeighborGrid::new(scenario.field.size, args.neighbor_unit.unwrap_or(1.4)));

        OptimalStepsModel {
            pedestrians: Vec::new(),
            neighbor_grid,
            neighbor_grid_indices: Vec::new(),
            next_state: Mutex::new(Vec::new()),
        }
    }

    fn spawn_pedestrians(&mut self, mut new_pedestrians: Vec<super::Pedestrian>) {
        self.pedestrians.append(&mut new_pedestrians);

        if let Some(neighbor_grid) = &mut self.neighbor_grid {
            neighbor_grid.update(self.pedestrians.iter().map(|p| p.pos));

            let mut sorted_pedestrians = Vec::with_capacity(self.pedestrians.len());
            self.neighbor_grid_indices = Vec::with_capacity(neighbor_grid.data.len() + 1);
            self.neighbor_grid_indices.push(0);

            let mut index = 0;
            for cell in neighbor_grid.data.iter() {
                for j in 0..cell.len() {
                    let prev = cell[j] as usize;
                    sorted_pedestrians.push(self.pedestrians[prev].clone());
                }
                index += cell.len();
                self.neighbor_grid_indices.push(index as u32);
            }

            self.pedestrians = sorted_pedestrians;
        }
    }

    fn calc_next_state(&self, sim: &Simulator) {
        // let optimizer = NelderMead {
        //     init: vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
        //     bound: Some(R),
        // };
        let optimizer = CircleBorder {
            radius: R,
            samples: 16,
        };

        let state: Vec<_> = self
            .pedestrians
            .par_iter()
            .enumerate()
            .filter(|(_, ped)| ped.active)
            .map(|(id, ped)| {
                let active = sim.field.get_field_potential(ped.destination, ped.pos) > 2.0;

                let f = |x: Vec2| self.calc_potential(ped.pos + x, sim, ped.destination, id);
                let position = optimizer.optimize(f).0 + ped.pos;

                (position, active)
            })
            .collect();

        *self.next_state.lock().unwrap() = state;
    }

    fn apply_next_state(&mut self) {
        let next_state = self.next_state.lock().unwrap();

        self.pedestrians
            .iter_mut()
            .filter(|ped| ped.active)
            .zip(next_state.iter().cloned())
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

impl OptimalStepsModel {
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

        let p_field = sim.field.get_field_potential(destination, pos);

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

        let p_pedestrians = if let Some(grid) = &self.neighbor_grid {
            let ix = (pos / grid.unit).as_ivec2() + 1;
            let ix = Index::new(ix.x, ix.y);
            let mut potential = 0.0;

            let shape = IVec2::new(grid.shape.1 as i32, grid.shape.0 as i32);
            let y_start = (ix.y - 1).max(0);
            let y_end = (ix.y + 1).min(shape.y);
            let x_start = (ix.x - 1).max(0);
            let x_end = (ix.x + 1).min(shape.x);

            for y in y_start..=y_end {
                let offset = y * shape.x;
                let i_start = self.neighbor_grid_indices[(offset + x_start) as usize];
                let i_end = self.neighbor_grid_indices[(offset + x_end + 1) as usize];

                for i in i_start..i_end {
                    if i != self_id as u32 {
                        let distance = (pos - self.pedestrians[i as usize].pos).length();
                        potential += potential_ped_from_distance(distance);
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

        // let p_obstacles: f32 = sim
        //     .scenario
        //     .obstacles
        //     .iter()
        //     .map(|obs| {
        //         let delta = util::distance_from_line(pos, obs.line);
        //         if delta > H_O {
        //             0.0
        //         } else if delta < G_P_HALF {
        //             MU_O
        //         } else {
        //             NU_O * (-A_O * delta.powf(B_O)).exp()
        //         }
        //     })
        //     .sum();

        let p_obstacles = {
            let delta = sim.field.get_distance_from_obstacles(pos);

            if delta > H_O {
                0.0
            } else if delta < G_P_HALF {
                MU_O
            } else {
                NU_O * (-A_O * delta.powf(B_O)).exp()
            }
        };

        // p_field + p_pedestrians
        p_field + p_pedestrians + p_obstacles
    }
}
