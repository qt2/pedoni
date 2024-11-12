use eframe::wgpu;
use glam::{vec2, Vec2};
use ocl::{prm::Float2, ProQue};

use crate::simulator::Simulator;

use super::PedestrianModel;

const R: f32 = 0.3;

pub struct OptimalStepsModelGpu {
    pedestrians: Vec<super::Pedestrian>,
    pq: ProQue,
}

impl OptimalStepsModelGpu {
    pub fn new() -> Self {
        let source = include_str!("osm_gpu.cl");
        let pq = ProQue::builder().src(source).build().unwrap();

        OptimalStepsModelGpu {
            pedestrians: Vec::new(),
            pq,
        }
    }

    fn calc_next_state_kernel(&self, sim: &Simulator) -> ocl::Result<Vec<Vec2>> {
        let positions: Vec<Float2> = self
            .pedestrians
            .iter()
            .map(|p| p.pos.to_array().into())
            .collect();

        let pq = &self.pq;
        let dim = self.pedestrians.len();

        if dim == 0 {
            return Ok(Vec::new());
        }

        let position_buffer = pq
            .buffer_builder()
            .len(dim)
            .copy_host_slice(&positions)
            .build()?;
        let next_position_buffer = pq.buffer_builder().len(dim).build()?;

        let kernel = pq
            .kernel_builder("calc_next_state")
            .arg(&position_buffer)
            .arg(&next_position_buffer)
            .build()?;

        unsafe {
            kernel.enq()?;
        }

        let mut next_positions = vec![Float2::zero(); self.pedestrians.len()];
        next_position_buffer.read(&mut next_positions).enq()?;

        Ok(next_positions
            .into_iter()
            .map(|pos| Vec2::from(Into::<[f32; 2]>::into(pos)))
            .collect())
    }
}

impl PedestrianModel for OptimalStepsModelGpu {
    fn spawn_pedestrians(&mut self, mut pedestrians: Vec<super::Pedestrian>) {
        self.pedestrians.append(&mut pedestrians);
        self.pq.set_dims(self.pedestrians.len());
    }

    fn calc_next_state(&self, sim: &Simulator) -> Box<dyn std::any::Any> {
        let state = self.calc_next_state_kernel(sim).unwrap();
        Box::new(state)
    }

    fn apply_next_state(&mut self, next_state: Box<dyn std::any::Any>) {
        let next_state = *next_state.downcast::<Vec<Vec2>>().unwrap();

        self.pedestrians
            .iter_mut()
            .filter(|ped| ped.active)
            .zip(next_state)
            .for_each(|(ped, pos)| {
                ped.pos = pos;
                // ped.active = active;
            });
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        self.pedestrians.clone()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.pedestrians.len() as i32
    }
}

#[cfg(test)]
mod tests {}
