use std::{sync::Mutex, time::Duration};

use ocl::{
    core::{
        AddressingMode, FilterMode, ImageChannelDataType, ImageChannelOrder, MemObjectType,
        ProfilingInfo,
    },
    prm::{Float2, Float4, Uint2},
    Event, Image, MemFlags, ProQue, Sampler,
};

use super::PedestrianModel;
use crate::simulator::{field::Field, util::ToGlam, Simulator};

const LOCAL_WORK_SIZE: usize = 8;

pub struct OptimalStepsModelGpu {
    positions: Vec<Float2>,
    destinations: Vec<u32>,
    pq: ProQue,
    field_potential_grids_buffer: Image<f32>,
    field_potential_sampler: Sampler,
    next_state: Mutex<Vec<Float2>>,
}

impl PedestrianModel for OptimalStepsModelGpu {
    fn new(
        _args: &crate::args::Args,
        _scenario: &crate::simulator::scenario::Scenario,
        field: &Field,
    ) -> Self {
        let source = include_str!("osm_gpu.cl");
        let pq = ProQue::builder()
            .src(source)
            .queue_properties(ocl::core::QUEUE_PROFILING_ENABLE)
            .dims(1)
            .build()
            .unwrap();

        let field_potential_grids_data: Vec<f32> = field
            .potentials
            .iter()
            .flat_map(|grid| grid.iter().cloned())
            .collect();

        let field_potential_grids_buffer = Image::builder()
            .channel_data_type(ImageChannelDataType::Float)
            .channel_order(ImageChannelOrder::R)
            .image_type(MemObjectType::Image2dArray)
            .dims((field.shape.1, field.shape.0, field.potentials.len()))
            .array_size(field.potentials.len())
            .copy_host_slice(&field_potential_grids_data)
            .queue(pq.queue().clone())
            .build()
            .unwrap();
        let field_potential_sampler = Sampler::new(
            &pq.context(),
            false,
            AddressingMode::Clamp,
            FilterMode::Linear,
        )
        .unwrap();

        OptimalStepsModelGpu {
            positions: Vec::new(),
            destinations: Vec::new(),
            pq,
            field_potential_grids_buffer,
            field_potential_sampler,
            next_state: Mutex::new(Vec::new()),
        }
    }

    fn spawn_pedestrians(&mut self, new_pedestrians: Vec<super::Pedestrian>) {
        for p in new_pedestrians {
            self.positions.push(p.pos.to_array().into());
            self.destinations.push(p.destination as u32);
        }
    }

    fn calc_next_state(&self, sim: &Simulator) {
        let state = self.calc_next_state_kernel(sim).unwrap();
        *self.next_state.lock().unwrap() = state;
    }

    fn apply_next_state(&mut self) {
        let mut next_state = self.next_state.lock().unwrap();
        std::mem::swap(&mut self.positions, &mut *next_state);
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        (0..self.positions.len())
            .map(|i| super::Pedestrian {
                active: true,
                pos: self.positions[i].to_glam(),
                destination: self.destinations[i] as usize,
            })
            .collect()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.positions.len() as i32
    }
}

impl OptimalStepsModelGpu {
    fn calc_next_state_kernel(&self, sim: &Simulator) -> ocl::Result<Vec<Float2>> {
        let ped_count = self.positions.len();
        if ped_count == 0 {
            return Ok(Vec::new());
        }

        let waypoints: Vec<Float4> = sim
            .scenario
            .waypoints
            .iter()
            .map(|wp| Float4::new(wp.line[0].x, wp.line[0].y, wp.line[1].x, wp.line[1].y))
            .collect();

        let field_potential_unit = sim.field.unit;

        let neighbor_grid = sim.neighbor_grid.as_ref().unwrap();
        let mut neighbor_grid_data: Vec<u32> = Vec::with_capacity(self.positions.len());
        let mut neighbor_grid_indices: Vec<u32> = Vec::with_capacity(neighbor_grid.data.len() + 1);
        neighbor_grid_indices.push(0);

        let mut index = 0;
        for cell in neighbor_grid.data.iter() {
            index += cell.len() as u32;
            neighbor_grid_indices.push(index);
            neighbor_grid_data.append(&mut cell.to_vec());
        }

        let neighbor_grid_shape = neighbor_grid.shape;
        let neighbor_grid_shape =
            Uint2::new(neighbor_grid_shape.0 as u32, neighbor_grid_shape.1 as u32);

        let pq = &self.pq;
        let global_work_size =
            (ped_count + LOCAL_WORK_SIZE - 1) / LOCAL_WORK_SIZE * LOCAL_WORK_SIZE;

        let position_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.positions)
            .build()?;
        let destination_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.destinations)
            .build()?;
        let waypoint_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(waypoints.len())
            .copy_host_slice(&waypoints)
            .build()?;
        let neighbor_grid_data_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(neighbor_grid_data.len())
            .copy_host_slice(&neighbor_grid_data)
            .build()?;
        let neighbor_grid_indices_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(neighbor_grid_indices.len())
            .copy_host_slice(&neighbor_grid_indices)
            .build()?;
        let next_position_buffer = pq
            .buffer_builder()
            .flags(MemFlags::WRITE_ONLY)
            .len(ped_count)
            .build()?;

        let kernel = pq
            .kernel_builder("calc_next_state")
            .arg(&(ped_count as u32))
            .arg(&position_buffer)
            .arg(&destination_buffer)
            .arg(&waypoint_buffer)
            .arg(&self.field_potential_grids_buffer)
            .arg_sampler(&self.field_potential_sampler)
            .arg(&field_potential_unit)
            .arg(&neighbor_grid_data_buffer)
            .arg(&neighbor_grid_indices_buffer)
            .arg(&neighbor_grid_shape)
            .arg(&neighbor_grid.unit)
            .arg(&next_position_buffer)
            .global_work_size(global_work_size)
            .local_work_size(LOCAL_WORK_SIZE)
            .build()?;

        let mut event = Event::empty();
        unsafe {
            kernel.cmd().enew(&mut event).enq()?;
        }
        event.wait_for()?;
        let start = event.profiling_info(ProfilingInfo::Start)?.time()?;
        let end = event.profiling_info(ProfilingInfo::End)?.time()?;
        let time_kernel = Duration::from_nanos(end - start);

        {
            let mut step_metrics = sim.step_metrics.lock().unwrap();
            step_metrics.time_calc_state_kernel = Some(time_kernel.as_secs_f64());
        }

        let mut next_positions = vec![Float2::zero(); ped_count];
        next_position_buffer.read(&mut next_positions).enq()?;

        Ok(next_positions)
    }
}

#[cfg(test)]
mod tests {}
