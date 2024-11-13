use glam::{vec2, Vec2};
use ocl::{
    prm::{Float16, Float2, Float4, Uint16, Uint2},
    ProQue,
};

use crate::simulator::Simulator;

use super::PedestrianModel;

const R: f32 = 0.3;

pub struct OptimalStepsModelGpu {
    positions: Vec<Float2>,
    destinations: Vec<u32>,
    pq: ProQue,
}

impl OptimalStepsModelGpu {
    pub fn new() -> Self {
        let source = include_str!("osm_gpu.cl");
        let pq = ProQue::builder().src(source).build().unwrap();

        OptimalStepsModelGpu {
            positions: Vec::new(),
            destinations: Vec::new(),
            pq,
        }
    }

    fn calc_next_state_kernel(&self, sim: &Simulator) -> ocl::Result<Vec<Float2>> {
        let waypoints: Vec<Float4> = sim
            .scenario
            .waypoints
            .iter()
            .map(|wp| Float4::new(wp.line[0].x, wp.line[0].y, wp.line[1].x, wp.line[1].y))
            .collect();

        let neighbor_grid = sim.neighbor_grid.as_ref().unwrap();
        let mut neighbor_grid_data: Vec<u32> = Vec::with_capacity(self.positions.len());
        let mut neighbor_grid_indices: Vec<u32> = Vec::with_capacity(neighbor_grid.len() + 1);
        neighbor_grid_indices.push(0);

        let mut index = 0;
        for cell in neighbor_grid.iter() {
            index += cell.len() as u32;
            neighbor_grid_indices.push(index);
            neighbor_grid_data.append(&mut cell.to_vec());
        }
        // let neighbor_grid: Vec<Uint16> = sim
        //     .neighbor_grid
        //     .as_ref()
        //     .unwrap()
        //     .iter()
        //     .map(|neighbors| {
        //         let mut item = [0u32; 16];
        //         for i in 0..16.min(neighbors.len()) {
        //             item[i] = neighbors[i];
        //         }
        //         item.into()
        //     })
        //     .collect();
        let neighbor_grid_shape = sim.neighbor_grid.as_ref().unwrap().shape();
        let neighbor_grid_shape =
            Uint2::new(neighbor_grid_shape[0] as u32, neighbor_grid_shape[1] as u32);
        let neighbor_grid_unit = sim.neighbor_grid_unit.unwrap();

        let pq = &self.pq;
        let dim = self.positions.len();

        if dim == 0 {
            return Ok(Vec::new());
        }

        let position_buffer = pq
            .buffer_builder()
            .copy_host_slice(&self.positions)
            .build()?;
        let destination_buffer = pq
            .buffer_builder()
            .copy_host_slice(&self.destinations)
            .build()?;
        let waypoint_buffer = pq
            .buffer_builder()
            .len(waypoints.len())
            .copy_host_slice(&waypoints)
            .build()?;
        let neighbor_grid_data_buffer = pq
            .buffer_builder()
            .len(neighbor_grid_data.len())
            .copy_host_slice(&neighbor_grid_data)
            .build()?;
        let neighbor_grid_indices_buffer = pq
            .buffer_builder()
            .len(neighbor_grid_indices.len())
            .copy_host_slice(&neighbor_grid_indices)
            .build()?;
        let next_position_buffer = pq.buffer_builder().build()?;

        let kernel = pq
            .kernel_builder("calc_next_state")
            .arg(&position_buffer)
            .arg(&destination_buffer)
            .arg(&waypoint_buffer)
            .arg(&neighbor_grid_data_buffer)
            .arg(&neighbor_grid_indices_buffer)
            .arg(&neighbor_grid_shape)
            .arg(&neighbor_grid_unit)
            .arg(&next_position_buffer)
            .build()?;

        unsafe {
            kernel.enq()?;
        }

        let mut next_positions = vec![Float2::zero(); dim];
        next_position_buffer.read(&mut next_positions).enq()?;

        Ok(next_positions)
    }
}

impl PedestrianModel for OptimalStepsModelGpu {
    fn spawn_pedestrians(&mut self, pedestrians: Vec<super::Pedestrian>) {
        for p in pedestrians {
            self.positions.push(p.pos.to_array().into());
            self.destinations.push(p.destination as u32);
        }
        self.pq.set_dims(self.positions.len());
    }

    fn calc_next_state(&self, sim: &Simulator) -> Box<dyn std::any::Any> {
        let state = self.calc_next_state_kernel(sim).unwrap();
        Box::new(state)
    }

    fn apply_next_state(&mut self, next_state: Box<dyn std::any::Any>) {
        let next_state = *next_state.downcast::<Vec<Float2>>().unwrap();
        self.positions = next_state;
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        (0..self.positions.len())
            .map(|i| super::Pedestrian {
                active: true,
                pos: self.positions[i].as_vec2(),
                destination: self.destinations[i] as usize,
            })
            .collect()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.positions.len() as i32
    }
}

pub trait AsVec2 {
    fn as_vec2(self) -> Vec2;
}

impl AsVec2 for Float2 {
    fn as_vec2(self) -> Vec2 {
        let array: [f32; 2] = self.into();
        array.into()
    }
}

#[cfg(test)]
mod tests {}
