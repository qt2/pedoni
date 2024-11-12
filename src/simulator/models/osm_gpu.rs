use eframe::wgpu;
use glam::{vec2, Vec2};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::simulator::{
    optim::{NelderMead, Optimizer},
    util, Simulator, WgpuResources,
};

use super::PedestrianModel;

const R: f32 = 0.3;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DPedestrian {
    pos: Vec2,
    destination: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DState {
    pos: Vec2,
    active: i32,
}

pub struct OptimalStepsModelGpu {
    pedestrians: Vec<super::Pedestrian>,
    wgpu_resources: WgpuResources,
    pipeline: wgpu::ComputePipeline,
    pedestrian_buffer: wgpu::Buffer,
    state_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl OptimalStepsModelGpu {
    pub fn new(wgpu_resources: WgpuResources) -> Self {
        let device = &wgpu_resources.device;

        let shader = device.create_shader_module(wgpu::include_wgsl!("osm_gpu.wgsl"));

        let pedestrian_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pedestrian_buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            size: size_of::<DPedestrian>() as u64 * 65536,
            mapped_at_creation: false,
        });
        let state_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("state_buffer"),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            size: size_of::<DState>() as u64 * 65536,
            mapped_at_creation: false,
        });
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_buffer"),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            size: state_buffer.size(),
            mapped_at_creation: false,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pedestrian_pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pedestrian_bind_group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pedestrian_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: state_buffer.as_entire_binding(),
                },
            ],
        });

        OptimalStepsModelGpu {
            pedestrians: Vec::new(),
            pipeline,
            pedestrian_buffer,
            state_buffer,
            output_buffer,
            bind_group,
            wgpu_resources,
        }
    }

    pub fn prepare(&self, queue: &wgpu::Queue) {
        // let pedestrians: Vec<_> = self.pedestrians.iter().map(|p| DPedestrian {

        // }).collect();
        // queue.write_buffer(&self.pedestrian_buffer, 0, bytemuck::cast_slice(&[&self.pedestrians]));
    }

    pub async fn execute(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&{ wgpu::CommandEncoderDescriptor::default() });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.insert_debug_marker("compute_pedestrian");
            pass.dispatch_workgroups(128, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &self.state_buffer,
            0,
            &self.output_buffer,
            0,
            self.state_buffer.size(),
        );

        queue.submit(Some(encoder.finish()));

        let output_slice = self.output_buffer.slice(..);
        let (tx, rx) = flume::bounded(1);
        output_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());

        device.poll(wgpu::Maintain::wait()).panic_on_timeout();

        if let Ok(Ok(())) = rx.recv_async().await {
            let data = output_slice.get_mapped_range();
            let states: Vec<DState> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            self.output_buffer.unmap();
        }
    }
}

impl PedestrianModel for OptimalStepsModelGpu {
    fn spawn_pedestrians(&mut self, mut pedestrians: Vec<super::Pedestrian>) {
        self.pedestrians.append(&mut pedestrians);
    }

    fn calc_next_state(&self, sim: &Simulator) -> Box<dyn std::any::Any> {
        // let state: Vec<_> = self.execute(device, queue)
        let state = Vec::<bool>::new();
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
