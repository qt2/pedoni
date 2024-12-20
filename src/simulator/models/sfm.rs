use std::sync::Mutex;

use glam::{IVec2, Vec2};
use rayon::prelude::*;

use crate::simulator::{util::Index, NeighborGrid, Simulator};

use super::PedestrianModel;

#[derive(Default)]
pub struct SocialForceModel {
    pedestrians: Pedestrians,
    neighbor_grid: Option<NeighborGrid>,
    neighbor_grid_indices: Vec<u32>,
    next_state: Mutex<Vec<Vec2>>,
}

#[derive(Debug, Default, Clone)]
pub struct Pedestrians {
    positions: Vec<Vec2>,
    destinations: Vec<u32>,
    velocities: Vec<Vec2>,
    desired_speeds: Vec<f32>,
}

impl Pedestrians {
    pub fn push(&mut self, position: Vec2, destination: u32, velocity: Vec2, desired_speed: f32) {
        self.positions.push(position);
        self.destinations.push(destination);
        self.velocities.push(velocity);
        self.desired_speeds.push(desired_speed);
    }

    pub fn copy(&mut self, other: &Pedestrians, index: usize) {
        self.push(
            other.positions[index],
            other.destinations[index],
            other.velocities[index],
            other.desired_speeds[index],
        );
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }
}

impl PedestrianModel for SocialForceModel {
    fn new(
        args: &crate::args::Args,
        scenario: &crate::simulator::scenario::Scenario,
        _field: &crate::simulator::field::Field,
    ) -> Self {
        let neighbor_grid = (!args.no_grid)
            .then(|| NeighborGrid::new(scenario.field.size, args.neighbor_unit.unwrap_or(1.4)));

        SocialForceModel {
            neighbor_grid,
            ..Default::default()
        }
    }

    fn spawn_pedestrians(&mut self, new_pedestrians: Vec<super::Pedestrian>) {
        for p in new_pedestrians {
            self.pedestrians
                .push(p.pos, p.destination as u32, Vec2::ZERO, 1.34);
        }

        if let Some(neighbor_grid) = &mut self.neighbor_grid {
            neighbor_grid.update(self.pedestrians.positions.iter().cloned());

            let mut sorted_pedestrians = Pedestrians::default();
            self.neighbor_grid_indices = Vec::with_capacity(neighbor_grid.data.len() + 1);
            self.neighbor_grid_indices.push(0);
            let mut index = 0;

            for cell in neighbor_grid.data.iter() {
                for j in 0..cell.len() {
                    sorted_pedestrians.copy(&self.pedestrians, cell[j] as usize);
                }
                index += cell.len();
                self.neighbor_grid_indices.push(index as u32);
            }

            self.pedestrians = sorted_pedestrians;
        }
    }

    fn calc_next_state(&self, sim: &Simulator) {
        let pedestrians = &self.pedestrians;
        let accelerations: Vec<Vec2> = (0..pedestrians.len())
            .into_par_iter()
            .map(|id| {
                let pos = pedestrians.positions[id];
                let vel = pedestrians.velocities[id];
                let destination = pedestrians.destinations[id] as usize;
                let desired_speed = pedestrians.desired_speeds[id];

                let mut acc = Vec2::ZERO;

                // calculate force from the destination.
                let direction = sim.field.get_potential_grad(destination, pos).normalize();
                acc += (direction * desired_speed - vel) / 0.5;

                // calculate force from other pedestrians.
                if let Some(grid) = &self.neighbor_grid {
                    let ix = (pos / grid.unit).as_ivec2() + 1;
                    let ix = Index::new(ix.x, ix.y);

                    let shape = IVec2::new(grid.shape.1 as i32, grid.shape.0 as i32);
                    let y_start = (ix.y - 1).max(0);
                    let y_end = (ix.y + 1).min(shape.y - 1);
                    let x_start = (ix.x - 1).max(0);
                    let x_end = (ix.x + 1).min(shape.x - 1);

                    for y in y_start..=y_end {
                        let offset = y * shape.x;
                        let i_start =
                            self.neighbor_grid_indices[(offset + x_start) as usize] as usize;
                        let i_end =
                            self.neighbor_grid_indices[(offset + x_end + 1) as usize] as usize;

                        for i in i_start..i_end {
                            if i != id {
                                let difference = pos - self.pedestrians.positions[i];
                                let distance_squared = difference.length_squared();
                                if distance_squared > 4.0 {
                                    continue;
                                }

                                let distance = distance_squared.sqrt();
                                let direction = difference.normalize();

                                if distance <= 0.4 {
                                    acc += 1000.0 * direction;
                                    continue;
                                }

                                let vel_i = pedestrians.velocities[i];
                                let t1 = difference - vel_i * 0.1;
                                let t1_length = t1.length();
                                let t2 = distance + t1_length;
                                let b = (t2.powi(2) - (vel_i.length() * 0.1).powi(2)).sqrt() * 0.5;

                                let nabla_b = t2 * (direction + t1 / t1_length) / (4.0 * b);
                                let force = 2.1 / 0.3 * (-b / 0.3).exp() * nabla_b;

                                acc += force;
                            }
                        }
                    }
                }

                // calculate force from obstacles.
                let distance = sim.field.get_obstacle_distance(pos);
                let direction = -sim.field.get_obstacle_distance_grad(pos).normalize();
                let force = if distance >= 0.4 {
                    10.0 * 0.2 * (-distance / 0.2).exp() * direction
                } else {
                    10000.0 * direction
                };
                acc += force;

                acc
            })
            .collect();

        *self.next_state.lock().unwrap() = accelerations;
    }

    fn apply_next_state(&mut self) {
        let accelerations = self.next_state.lock().unwrap();
        let pedestrians = &mut self.pedestrians;

        for i in 0..pedestrians.len() {
            let pos = &mut pedestrians.positions[i];
            let vel = &mut pedestrians.velocities[i];
            let desired_speed = pedestrians.desired_speeds[i];

            let vel_prev = *vel;
            *vel += accelerations[i] * 0.1;
            *vel = vel.clamp_length_max(desired_speed);
            *pos += (*vel + vel_prev) * 0.05;
        }
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        (0..self.pedestrians.len())
            .map(|i| super::Pedestrian {
                active: true,
                pos: self.pedestrians.positions[i],
                destination: self.pedestrians.destinations[i] as usize,
            })
            .collect()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.pedestrians.len() as i32
    }
}
