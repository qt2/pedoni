mod camera;

use std::mem;

use eframe::{
    egui,
    wgpu::{
        self, include_wgsl,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferBindingType, BufferUsages,
        FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline,
        RenderPipelineDescriptor, ShaderStages, VertexAttribute, VertexBufferLayout, VertexState,
    },
};

use crate::simulator::Simulator;

use self::camera::Camera;

pub struct Renderer {
    camera: Camera,
}

impl Renderer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;
        let shader = device.create_shader_module(include_wgsl!("./shader.wgsl"));

        let camera = Camera::default();
        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera"),
            });
        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera"),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pedestrians"),
            bind_group_layouts: &[&camera_bind_group_layout],
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

        let instances: Vec<Instance> = Vec::new();
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: BufferUsages::VERTEX,
        });

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(PedestrianRenderResources {
                pipeline,
                vertex_buffer,
                index_buffer,
                instance_buffer,
                camera_buffer,
                camera_bind_group,
            });

        Renderer { camera }
    }

    pub fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, simulator: &Simulator) {
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());

        let delta_wheel_y = ctx.input(|i| i.smooth_scroll_delta).y;
        self.camera.scale *= 2.0_f32.powf(delta_wheel_y * 0.01);

        let delta_drag = 1.0 * response.drag_delta() / self.camera.scale;
        self.camera.position[0] -= delta_drag.x;
        self.camera.position[1] += delta_drag.y;

        let size = rect.size();
        self.camera.rect = [size.x, size.y];

        // let pedestrians: Vec<_> = simulator
        //     .pedestrians
        //     .iter()
        //     .filter(|p| !p.has_arrived_goal)
        //     .map(|p| {
        //         let color = Color::pick(p.trip_id);
        //         Instance {
        //             position: p.position.into(),
        //             color: color.into(),
        //         }
        //     })
        //     .collect();

        let pedestrians = Vec::new();

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            CustomCallback {
                camera: self.camera.clone(),
                pedestrians,
            },
        ));
    }
}

struct CustomCallback {
    camera: Camera,
    pedestrians: Vec<Instance>,
}

impl egui_wgpu::CallbackTrait for CustomCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &mut PedestrianRenderResources = callback_resources.get_mut().unwrap();

        queue.write_buffer(
            &resources.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera]),
        );

        resources.instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&self.pedestrians),
            usage: BufferUsages::VERTEX,
        });

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'a>,
        callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
        let resources: &PedestrianRenderResources = callback_resources.get().unwrap();

        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_bind_group(0, &resources.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(resources.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..self.pedestrians.len() as u32);
        // render_pass.draw(0..3, 0..1);
    }
}

struct PedestrianRenderResources {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instance_buffer: Buffer,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
}

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

#[derive(Debug, Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub const WHITE: Color = Color::new(255, 255, 255, 255);
    pub const MAGENTA: Color = Color::new(255, 0, 255, 255);
    pub const YELLOW: Color = Color::new(255, 255, 0, 255);
    pub const CYAN: Color = Color::new(0, 255, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0, 255);
    pub const GREEN: Color = Color::new(0, 255, 0, 255);
    pub const BLUE: Color = Color::new(0, 0, 255, 255);
    pub const BLACK: Color = Color::new(0, 0, 0, 255);

    pub const PALLET: [Color; 6] = [
        Color::MAGENTA,
        Color::CYAN,
        Color::YELLOW,
        Color::RED,
        Color::BLUE,
        Color::GREEN,
    ];

    const fn pick(value: usize) -> Self {
        Color::PALLET[value % Color::PALLET.len()]
    }
}

impl From<Color> for u32 {
    fn from(c: Color) -> Self {
        u32::from_le_bytes([c.r, c.g, c.b, c.a])
    }
}
