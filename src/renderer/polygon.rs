use std::mem;

use eframe::{
    egui,
    wgpu::{
        self, include_wgsl,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroup, Buffer, BufferAddress, BufferUsages, FragmentState, MultisampleState,
        PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPipeline,
        RenderPipelineDescriptor, VertexAttribute, VertexBufferLayout, VertexState,
    },
};
use egui_wgpu::RenderState;

use super::{
    camera::{Camera, CameraResources},
    texture::Texture,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [VertexAttribute; 2] = wgpu::vertex_attr_array![0  => Float32x2, 1=>Float32x2];

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
pub struct Instance {
    pub position: [f32; 2],
    pub color: u32,
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

pub struct PolygonRenderResources {
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub instance_buffer: Buffer,
    pub diffuse_bind_group: BindGroup,
}

impl PolygonRenderResources {
    pub fn prepare(render_state: &RenderState, camera_resources: &CameraResources) -> Self {
        let device = &render_state.device;
        let shader = device.create_shader_module(include_wgsl!("./shaders/polygon.wgsl"));

        let diffuse_image = image::load_from_memory(include_bytes!("assets/objects.png")).unwrap();
        let diffuse_texture =
            Texture::from_image(device, &render_state.queue, &diffuse_image, "diffuse");

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("polygons"),
            bind_group_layouts: &[
                &texture_bind_group_layout,
                &camera_resources.camera_bind_group_layout,
            ],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("polygons"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_state.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&[
                Vertex {
                    position: [-0.5, -0.5],
                    uv: [0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [-0.5, 0.5],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [0.5, 0.5],
                    uv: [1.0, 0.0],
                },
            ]),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 3]),
            usage: BufferUsages::INDEX,
        });
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice::<Instance, _>(&[]),
            usage: BufferUsages::VERTEX,
        });

        PolygonRenderResources {
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            diffuse_bind_group,
        }
    }
}

pub struct PolygonRenderCallback {
    pub camera: Camera,
    pub instances: Vec<Instance>,
}

impl egui_wgpu::CallbackTrait for PolygonRenderCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        {
            let resources: &CameraResources = callback_resources.get().unwrap();

            queue.write_buffer(
                &resources.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera]),
            );
        }
        {
            let resources: &mut PolygonRenderResources = callback_resources.get_mut().unwrap();

            resources.instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("instance buffer"),
                contents: bytemuck::cast_slice(&self.instances),
                usage: BufferUsages::VERTEX,
            });
        }

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'a>,
        callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
        let camera_resources: &CameraResources = callback_resources.get().unwrap();
        let resources: &PolygonRenderResources = callback_resources.get().unwrap();

        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_bind_group(0, &resources.diffuse_bind_group, &[]);
        render_pass.set_bind_group(1, &camera_resources.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(resources.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..4, 0, 0..self.instances.len() as u32);
        // render_pass.draw(0..3, 0..1);
    }
}
