pub mod camera;
pub mod pedestrian;

use std::mem;

use camera::CameraResources;
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
use pedestrian::PedestrianRenderResources;

use self::camera::Camera;

pub struct Renderer {
    camera: Camera,
}

impl eframe::App for Renderer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Pedoni");
            // ui.label(format!(
            //     "Number of pedestrians: {}",
            //     self.simulator.pedestrians.len(),
            // ));
            // ui.label(format!(
            //     "Calculation time per frame: {:.4}s",
            //     self.simulate_time.as_secs_f64()
            // ));
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.draw_canvas(ui, ctx);
            });
        });

        ctx.request_repaint();
    }
}

impl Renderer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;

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

        let resources = &mut render_state.renderer.write().callback_resources;

        resources.insert(CameraResources {
            camera_bind_group,
            camera_buffer,
        });
        resources.insert(PedestrianRenderResources::new(
            render_state,
            &camera_bind_group_layout,
        ));

        Renderer { camera }
    }

    pub fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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

        // let pedestrians = Vec::new();

        let pedestrians = vec![Instance {
            position: [0.0, 0.0],
            color: Color::RED.into(),
        }];

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
