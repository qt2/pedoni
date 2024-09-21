use std::ops::Range;

use eframe::wgpu::{self, util::DeviceExt};
use egui_wgpu::RenderState;
use glam::vec2;
use rustc_hash::FxHashMap;

use crate::renderer::fill::{Instance, Vertex};

use super::{
    camera::{Camera, View},
    fill, PipelineSet,
};

pub struct MeshRegistry {
    mapping: FxHashMap<u64, (Range<u64>, Range<u64>)>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    vertex_len: u64,
    index_len: u64,
}

impl MeshRegistry {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: 64 * 512,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer"),
            size: 4 * 128,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        MeshRegistry {
            mapping: FxHashMap::default(),
            vertex_buffer,
            index_buffer,
            vertex_len: 0,
            index_len: 0,
        }
    }

    pub fn contains_key(&self, id: u64) -> bool {
        self.mapping.contains_key(&id)
    }

    pub fn get(&self, id: u64) -> Option<(Range<u64>, Range<u64>)> {
        self.mapping.get(&id).cloned()
    }

    pub fn insert(
        &mut self,
        queue: &wgpu::Queue,
        id: u64,
        vertices: &[impl bytemuck::NoUninit],
        indices: &[u16],
    ) {
        let vertex_len = self.vertex_len;
        let index_len = self.index_len;
        let vertices = bytemuck::cast_slice(vertices);
        let indices = bytemuck::cast_slice(indices);

        queue.write_buffer(&self.vertex_buffer, vertex_len, vertices);
        queue.write_buffer(&self.index_buffer, index_len, indices);

        self.vertex_len += vertices.len() as u64;
        self.index_len += indices.len() as u64;

        let ranges = (vertex_len..self.vertex_len, index_len..self.index_len);
        self.mapping.insert(id, ranges);
    }

    pub fn add(
        &mut self,
        queue: &wgpu::Queue,
        vertices: &[impl bytemuck::NoUninit],
        indices: &[u16],
    ) -> u64 {
        let mut id = fastrand::u64(..);

        while self.contains_key(id) {
            id = fastrand::u64(..);
        }

        self.insert(queue, id, vertices, indices);

        id
    }
}

pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
    len: u64,
    segments: Vec<Range<u64>>,
}

impl InstanceBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: 256 * 2048,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        InstanceBuffer {
            buffer,
            len: 0,
            segments: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.segments.clear();
    }

    pub fn push(&mut self, queue: &wgpu::Queue, instances: &[impl bytemuck::NoUninit]) {
        let data = bytemuck::cast_slice(instances);
        let len = self.len;
        queue.write_buffer(&self.buffer, len, data);

        self.len += data.len() as u64;
        self.segments.push(len..self.len);
    }

    pub fn segment(&self, index: usize) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(self.segments[index].clone())
    }
}

pub struct RenderResources {
    fill_pipeline: PipelineSet,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instance_buffer: InstanceBuffer,
    mesh_registry: MeshRegistry,
}

impl RenderResources {
    pub fn new(render_state: &RenderState) -> Self {
        let RenderState {
            device,
            queue,
            target_format,
            ..
        } = &render_state;

        let camera = Camera::default();
        let view = View::from(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[view]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let fill_pipeline =
            fill::setup_fill_pipeline(device, *target_format, &camera_bind_group_layout);

        let mut mesh_registry = MeshRegistry::new(device);

        mesh_registry.insert(
            queue,
            4,
            &[
                Vertex {
                    position: vec2(-0.5, 0.5),
                },
                Vertex {
                    position: vec2(-0.5, -0.5),
                },
                Vertex {
                    position: vec2(0.5, -0.5),
                },
                Vertex {
                    position: vec2(0.5, 0.5),
                },
            ],
            &[0, 1, 2, 0, 2, 3],
        );

        let instance_buffer = InstanceBuffer::new(device);

        RenderResources {
            fill_pipeline,
            camera_buffer,
            camera_bind_group,
            mesh_registry,
            instance_buffer,
        }
    }
}

pub struct RenderCallback {
    pub view: View,
    pub commands: Vec<DrawCommand>,
}

impl egui_wgpu::CallbackTrait for RenderCallback {
    fn prepare(
        &self,
        _device: &eframe::wgpu::Device,
        queue: &eframe::wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut eframe::wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<eframe::wgpu::CommandBuffer> {
        let resources: &mut RenderResources = callback_resources.get_mut().unwrap();

        queue.write_buffer(
            &resources.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.view]),
        );

        resources.instance_buffer.clear();

        for command in &self.commands {
            if command.instances.is_empty() {
                continue;
            }
            resources.instance_buffer.push(queue, &command.instances);
        }

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: eframe::egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'a>,
        callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
        let resources: &RenderResources = callback_resources.get().unwrap();

        render_pass.set_pipeline(&resources.fill_pipeline.pipeline);
        render_pass.set_bind_group(0, &resources.camera_bind_group, &[]);

        for (i, command) in self.commands.iter().enumerate() {
            if command.instances.is_empty() {
                continue;
            }

            let (vertex_range, index_range) = resources.mesh_registry.get(command.mesh_id).unwrap();
            let indices = 0..index_range.end as u32 / 2;

            render_pass
                .set_vertex_buffer(0, resources.mesh_registry.vertex_buffer.slice(vertex_range));
            render_pass.set_vertex_buffer(1, resources.instance_buffer.segment(i));
            render_pass.set_index_buffer(
                resources.mesh_registry.index_buffer.slice(index_range),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(indices, 0, 0..command.instances.len() as _)
        }
    }
}

pub struct DrawCommand {
    pub mesh_id: u64,
    pub instances: Vec<Instance>,
}
