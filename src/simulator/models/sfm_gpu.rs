use std::{sync::Mutex, time::Duration};

use ocl::{
    core::{
        AddressingMode, FilterMode, ImageChannelDataType, ImageChannelOrder, MemObjectType,
        ProfilingInfo,
    },
    prm::{Float2, Int2},
    Event, Image, MemFlags, ProQue, Sampler,
};

use crate::simulator::{
    util::{ToGlam, ToOcl},
    NeighborGrid, Simulator,
};

use super::PedestrianModel;

pub struct SocialForceModelGpu {
    pedestrians: Pedestrians,
    neighbor_grid: Option<NeighborGrid>,
    neighbor_grid_indices: Vec<u32>,
    next_state: Mutex<Vec<Float2>>,

    pq: ProQue,
    local_work_size: usize,
    field_potential_grids_buffer: Image<f32>,
    field_potential_sampler: Sampler,
}

#[derive(Debug, Default, Clone)]
pub struct Pedestrians {
    positions: Vec<Float2>,
    destinations: Vec<u32>,
    velocities: Vec<Float2>,
    desired_speeds: Vec<f32>,
}

impl Pedestrians {
    pub fn push(
        &mut self,
        position: Float2,
        destination: u32,
        velocity: Float2,
        desired_speed: f32,
    ) {
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

impl PedestrianModel for SocialForceModelGpu {
    fn new(
        args: &crate::args::Args,
        scenario: &crate::simulator::scenario::Scenario,
        field: &crate::simulator::field::Field,
    ) -> Self {
        let neighbor_grid = (!args.no_grid)
            .then(|| NeighborGrid::new(scenario.field.size, args.neighbor_unit.unwrap_or(1.4)));

        let source = include_str!("sfm_gpu.cl");
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
            AddressingMode::ClampToEdge,
            FilterMode::Linear,
        )
        .unwrap();

        SocialForceModelGpu {
            pedestrians: Default::default(),
            neighbor_grid,
            neighbor_grid_indices: Vec::default(),
            next_state: Default::default(),

            pq,
            local_work_size: args.work_size.unwrap_or(64),
            field_potential_grids_buffer,
            field_potential_sampler,
        }
    }

    fn spawn_pedestrians(&mut self, new_pedestrians: Vec<super::Pedestrian>) {
        for p in new_pedestrians {
            self.pedestrians
                .push(p.pos.to_ocl(), p.destination as u32, Float2::zero(), 1.34);
        }

        if let Some(neighbor_grid) = &mut self.neighbor_grid {
            neighbor_grid.update(self.pedestrians.positions.iter().map(|p| p.to_glam()));

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
        self.calc_next_state_kernel(sim).unwrap();
    }

    fn apply_next_state(&mut self) {
        // let accelerations = self.next_state.lock().unwrap();
        let pedestrians = &mut self.pedestrians;

        let next_state = self.next_state.lock().unwrap();

        for i in 0..pedestrians.len() {
            pedestrians.positions[i] = next_state[i];
        }
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        (0..self.pedestrians.len())
            .map(|i| super::Pedestrian {
                active: true,
                pos: self.pedestrians.positions[i].to_glam(),
                destination: self.pedestrians.destinations[i] as usize,
            })
            .collect()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.pedestrians.len() as i32
    }
}

impl SocialForceModelGpu {
    fn calc_next_state_kernel(&self, sim: &Simulator) -> ocl::Result<()> {
        let ped_count = self.pedestrians.len();
        if ped_count == 0 {
            return Ok(());
        }

        let field_potential_unit = sim.field.unit;

        let neighbor_grid = self.neighbor_grid.as_ref().unwrap();
        let neighbor_grid_shape =
            Int2::new(neighbor_grid.shape.0 as i32, neighbor_grid.shape.1 as i32);

        let pq = &self.pq;
        let global_work_size =
            (ped_count + self.local_work_size - 1) / self.local_work_size * self.local_work_size;

        let position_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.pedestrians.positions)
            .build()?;
        let destination_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.pedestrians.destinations)
            .build()?;
        let neighbor_grid_indices_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(self.neighbor_grid_indices.len())
            .copy_host_slice(&self.neighbor_grid_indices)
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
            .arg(&self.field_potential_grids_buffer)
            .arg_sampler(&self.field_potential_sampler)
            .arg(&field_potential_unit)
            // .arg(&neighbor_grid_data_buffer)
            .arg(&neighbor_grid_indices_buffer)
            .arg(&neighbor_grid_shape)
            .arg(&neighbor_grid.unit)
            .arg(&next_position_buffer)
            .global_work_size(global_work_size)
            .local_work_size(self.local_work_size)
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

        *self.next_state.lock().unwrap() = next_positions;

        Ok(())
    }
}
