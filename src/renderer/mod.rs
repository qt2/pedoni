mod callback;
pub mod camera;
pub mod fill;

use callback::{DrawCommand, RenderCallback, RenderResources};
use camera::{Camera, View};
use eframe::{egui, wgpu};
use fill::Instance;

use crate::SIMULATOR;

pub struct Renderer {
    camera: Camera,
}

impl eframe::App for Renderer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                for name in ["File", "Edit", "View"] {
                    if ui.selectable_label(false, name).clicked() {}
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.draw_canvas(ui, ctx);
            });
        });

        egui::Window::new("controller")
            .open(&mut true)
            .show(ctx, |ui| {
                ui.heading("Controller");
                if ui.button("Pause").clicked() {}
                ui.add(egui::DragValue::new(&mut 30));
            });

        ctx.request_repaint();
    }
}

impl Renderer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let render_resources = RenderResources::new(render_state);
        let resoureces = &mut render_state.renderer.write().callback_resources;
        resoureces.insert(render_resources);

        Renderer {
            camera: Camera::default(),
        }
    }

    pub fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());

        let camera = &mut self.camera;
        camera.size = glam::vec2(size.x, size.y);

        let delta_wheel_y = ctx.input(|i| i.smooth_scroll_delta).y;
        camera.scale *= 2.0_f32.powf(delta_wheel_y * 0.01);

        let delta_drag = 1.0 * response.drag_delta() / camera.scale;
        camera.position.x -= delta_drag.x;
        camera.position.y += delta_drag.y;

        // let instances = (0..3)
        //     .map(|i| fill::Instance {
        //         position: glam::vec2(i as f32 * 100.0, 0.0),
        //         scale: 24.0,
        //         // rect: [0.0, 0.0, 0.125, 0.125],
        //         color: [(i as u8 * 64), 255, 255, 255],
        //     })
        //     .collect();

        let instances = {
            let simulator = SIMULATOR.read().unwrap();
            simulator
                .pedestrians
                .iter()
                .map(|ped| Instance {
                    position: ped.pos,
                    scale: 1.0,
                    color: [255; 4],
                })
                .collect()
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RenderCallback {
                view: View::from(&self.camera),
                commands: vec![DrawCommand {
                    mesh_id: 4,
                    instances,
                }],
            },
        ));
    }
}

pub struct PipelineSet {
    pub pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
}
