pub mod camera;
pub mod polygon;
pub mod sprite;
pub mod texture;

use camera::{Camera, CameraResources};
use eframe::egui::{self, pos2, vec2, Color32, Vec2};
use polygon::{PolygonRenderCallback, PolygonRenderResources};

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

        let camera_resources = CameraResources::prepare(render_state);
        let polygon_render_resources =
            PolygonRenderResources::prepare(render_state, &camera_resources);

        let resources = &mut render_state.renderer.write().callback_resources;

        resources.insert(polygon_render_resources);
        resources.insert(camera_resources);

        Renderer {
            camera: Camera::default(),
        }
    }

    pub fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
        ui.set_clip_rect(rect);

        let delta_wheel_y = ctx.input(|i| i.smooth_scroll_delta).y;
        self.camera.scale *= 2.0_f32.powf(delta_wheel_y * 0.01);

        let delta_drag = 1.0 * response.drag_delta() / self.camera.scale;
        self.camera.position[0] -= delta_drag.x;
        self.camera.position[1] += delta_drag.y;

        let size = rect.size();
        self.camera.rect = [size.x, size.y];

        let camera_pos = Vec2::from(&self.camera.position);
        let camera_scale = self.camera.scale;

        let simulator = SIMULATOR.read().unwrap();

        simulator.pedestrians.iter().for_each(|p| {
            let pos = camera_scale * (vec2(p.pos.x, p.pos.y) - camera_pos);
            ui.painter()
                .circle_filled(pos.to_pos2(), 0.125 * camera_scale, Color32::BLUE);
        });

        simulator.pedestrians.iter().for_each(|p| {
            let pos = camera_scale * (vec2(p.pos.x, p.pos.y) - camera_pos) * 2.0;
            ui.painter()
                .circle_filled(pos.to_pos2(), 0.125 * camera_scale, Color32::RED);
        });
    }
}
