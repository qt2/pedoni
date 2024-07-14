use std::{mem, sync::Arc};

use eframe::{
    egui,
    wgpu::{
        self, include_wgsl,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferBindingType, BufferUsages,
        Device, FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
        RenderPipeline, RenderPipelineDescriptor, ShaderStages, VertexAttribute,
        VertexBufferLayout, VertexState,
    },
};
use egui_wgpu::RenderState;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [VertexAttribute; 1] = wgpu::vertex_attr_array![0  => Float32x2];

    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    position: [f32; 2],
    color: u32,
}

impl Instance {
    const ATTRIBS: [VertexAttribute; 2] = wgpu::vertex_attr_array![3  => Float32x2, 4 => Uint32];

    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct PedestrianRenderResources {
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub instance_buffer: Buffer,
    // camera_buffer: Buffer,
    // camera_bind_group: BindGroup,
}

impl PedestrianRenderResources {
    pub fn new(render_state: &RenderState, camera_bind_group_layout: &BindGroupLayout) -> Self {
        let device = &render_state.device;
        let shader = device.create_shader_module(include_wgsl!("./shaders/point.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pedestrians"),
            bind_group_layouts: &[camera_bind_group_layout],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pedestrians"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(render_state.target_format.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&[
                Vertex {
                    position: [-0.5, -0.5],
                },
                Vertex {
                    position: [0.5, -0.5],
                },
                Vertex {
                    position: [-0.5, 0.5],
                },
                Vertex {
                    position: [0.5, 0.5],
                },
            ]),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 2, 1, 3]),
            usage: BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice::<Instance, _>(&[]),
            usage: BufferUsages::VERTEX,
        });

        PedestrianRenderResources {
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
        }
    }
}
