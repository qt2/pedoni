use std::time::{Duration, Instant};

use ocl::{
    core::{ImageChannelDataType, ImageChannelOrder, MemObjectType, ProfilingInfo},
    prm::{Float2, Int2},
    Event, Image, MemFlags, ProQue,
};
use soa_derive::StructOfArray;

use crate::{
    field::Field,
    neighbor_grid::NeighborGrid,
    scenario::Scenario,
    util::{ToGlam, ToOcl},
    SimulatorOptions,
};

use super::PedestrianModel;

pub struct SocialForceModelGpu {
    pedestrians: PedestrianVec,
    neighbor_grid: NeighborGrid,
    neighbor_grid_indices: Vec<u32>,

    pq: ProQue,
    local_work_size: usize,

    potential_map_buffer: Image<f32>,
    distance_map_buffer: Image<f32>,
}

#[derive(Debug, Clone, StructOfArray)]
#[soa_derive(Debug, Default)]
pub struct Pedestrian {
    position: Float2,
    destination: u32,
    velocity: Float2,
    desired_speed: f32,
}

macro_rules! parallel {
    ($task:expr) => {
        $task()
    };
    ($task1:expr, $task2:expr) => {
        rayon::join($task1, $task2)
    };
    ($task1:expr, $task2:expr, $($rest:expr),+) => {{
        rayon::join($task1, || parallel!($task2, $($rest),*));
        ()
    }};
}

impl PedestrianModel for SocialForceModelGpu {
    fn new(options: &SimulatorOptions, scenario: &Scenario, field: &Field) -> Self {
        let neighbor_grid = NeighborGrid::new(scenario.field.size, options.neighbor_grid_unit);

        let source = include_str!("sfm_gpu.cl");
        let pq = ProQue::builder()
            .src(source)
            .queue_properties(ocl::core::QUEUE_PROFILING_ENABLE)
            .dims(1)
            .build()
            .unwrap();

        let potential_map_data: Vec<f32> = field
            .potential_maps
            .iter()
            .flat_map(|grid| grid.iter().cloned())
            .collect();
        let distance_map_data: Vec<f32> = field.distance_map.iter().cloned().collect();

        let potential_map_buffer = Image::builder()
            .channel_data_type(ImageChannelDataType::Float)
            .channel_order(ImageChannelOrder::R)
            .image_type(MemObjectType::Image2dArray)
            .dims((field.shape.1, field.shape.0, field.potential_maps.len()))
            .array_size(field.potential_maps.len())
            .copy_host_slice(&potential_map_data)
            .queue(pq.queue().clone())
            .build()
            .unwrap();

        let distance_map_buffer = Image::builder()
            .channel_data_type(ImageChannelDataType::Float)
            .channel_order(ImageChannelOrder::R)
            .image_type(MemObjectType::Image2d)
            .dims((field.shape.1, field.shape.0, 1))
            .copy_host_slice(&distance_map_data)
            .queue(pq.queue().clone())
            .build()
            .unwrap();

        SocialForceModelGpu {
            pedestrians: Default::default(),
            neighbor_grid,
            neighbor_grid_indices: Vec::default(),
            pq,
            local_work_size: options.gpu_work_size,
            potential_map_buffer,
            distance_map_buffer,
        }
    }

    fn spawn_pedestrians(&mut self, field: &Field, new_pedestrians: Vec<super::Pedestrian>) {
        for p in new_pedestrians {
            self.pedestrians.push(Pedestrian {
                position: p.pos.to_ocl(),
                destination: p.destination as u32,
                velocity: Float2::zero(),
                desired_speed: fastrand_contrib::f32_normal_approx(1.34, 0.26),
            });
        }

        dbg!("::");

        let instant = Instant::now();
        // self.neighbor_grid.update_only_active(
        //     self.pedestrians.position.iter().map(|p| p.to_glam()),
        //     self.pedestrians
        //         .iter()
        //         .map(|p| field.get_potential(*p.destination as usize, p.position.to_glam()) > 0.25),
        // );

        self.neighbor_grid
            .update(self.pedestrians.position.iter().map(|p| p.to_glam()));
        dbg!(instant.elapsed());

        let mut sorted = PedestrianVec::default();
        self.neighbor_grid_indices = Vec::with_capacity(self.neighbor_grid.data.len() + 1);
        self.neighbor_grid_indices.push(0);

        let instant = Instant::now();
        parallel! {
            || {
                for cell in self.neighbor_grid.data.iter() {
                    for f in cell.iter() {
                        sorted.position.push(self.pedestrians.position[*f as usize])
                    }
                }
            },
            || {
                for cell in self.neighbor_grid.data.iter() {
                    for f in cell.iter() {
                        sorted
                            .destination
                            .push(self.pedestrians.destination[*f as usize])
                    }
                }
            },
            || {
                for cell in self.neighbor_grid.data.iter() {
                    for f in cell.iter() {
                        sorted.velocity.push(self.pedestrians.velocity[*f as usize])
                    }
                }
            },
            || {
                for cell in self.neighbor_grid.data.iter() {
                    for f in cell.iter() {
                        sorted.desired_speed.push(self.pedestrians.desired_speed[*f as usize])
                    }
                }
            },
            || {
                for cell in self.neighbor_grid.data.iter() {
                    self.neighbor_grid_indices.push(cell.len() as u32);
                }
            }
        }
        dbg!(instant.elapsed());

        self.pedestrians = sorted;
    }

    fn update_states(&mut self, _scenario: &Scenario, field: &Field) {
        let accelerations = self.calc_next_state_kernel(field).unwrap();

        for i in 0..self.pedestrians.len() {
            let pos = &mut self.pedestrians.position[i];
            let vel = &mut self.pedestrians.velocity[i];
            let desired_speed = self.pedestrians.desired_speed[i];

            let vel_prev = vel.to_glam();
            let mut v = vel_prev + accelerations[i].to_glam() * 0.1;
            v = v.clamp_length_max(desired_speed * 1.3);
            let p = pos.to_glam() + (v + vel_prev) * 0.05;

            *vel = v.to_ocl();
            *pos = p.to_ocl();
        }
    }

    fn list_pedestrians(&self) -> Vec<super::Pedestrian> {
        self.pedestrians
            .iter()
            .map(|p| super::Pedestrian {
                pos: p.position.to_glam(),
                destination: *p.destination as usize,
            })
            .collect()
    }

    fn get_pedestrian_count(&self) -> i32 {
        self.pedestrians.len() as i32
    }
}

impl SocialForceModelGpu {
    fn calc_next_state_kernel(&self, field: &Field) -> ocl::Result<Vec<Float2>> {
        let ped_count = self.pedestrians.len();
        if ped_count == 0 {
            return Ok(Vec::new());
        }

        let neighbor_grid_shape = Int2::new(
            self.neighbor_grid.shape.0 as i32,
            self.neighbor_grid.shape.1 as i32,
        );

        let pq = &self.pq;
        let global_work_size =
            (ped_count + self.local_work_size - 1) / self.local_work_size * self.local_work_size;

        let position_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.pedestrians.position)
            .build()?;
        let velocity_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.pedestrians.velocity)
            .build()?;
        let disired_speed_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.pedestrians.desired_speed)
            .build()?;
        let destination_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(ped_count)
            .copy_host_slice(&self.pedestrians.destination)
            .build()?;
        let neighbor_grid_indices_buffer = pq
            .buffer_builder()
            .flags(MemFlags::READ_ONLY)
            .len(self.neighbor_grid_indices.len())
            .copy_host_slice(&self.neighbor_grid_indices)
            .build()?;
        let acceleration_buffer = pq
            .buffer_builder()
            .flags(MemFlags::WRITE_ONLY)
            .len(ped_count)
            .build()?;

        let kernel = pq
            .kernel_builder("calc_next_state")
            .arg(&(ped_count as u32))
            .arg(&position_buffer)
            .arg(&velocity_buffer)
            .arg(&disired_speed_buffer)
            .arg(&destination_buffer)
            .arg(&self.potential_map_buffer)
            .arg(&self.distance_map_buffer)
            .arg(&field.unit)
            .arg(&neighbor_grid_indices_buffer)
            .arg(&neighbor_grid_shape)
            .arg(&self.neighbor_grid.unit)
            .arg(&acceleration_buffer)
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
        let _time_kernel = Duration::from_nanos(end - start);

        let mut accelerations = vec![Float2::zero(); ped_count];
        acceleration_buffer.read(&mut accelerations).enq()?;

        Ok(accelerations)
    }
}
