use eframe::egui::{self, vec2, Color32, Vec2};

use crate::SIMULATOR;

#[derive(Debug)]
pub struct Camera {
    pub position: Vec2,
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec2::ZERO,
            scale: 16.0,
        }
    }
}

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
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Renderer {
            camera: Camera::default(),
        }
    }

    pub fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
        ui.set_clip_rect(rect);

        let camera = &mut self.camera;

        let delta_wheel_y = ctx.input(|i| i.smooth_scroll_delta).y;
        camera.scale *= 2.0_f32.powf(delta_wheel_y * 0.01);

        let delta_drag = 1.0 * response.drag_delta() / camera.scale;
        camera.position.x -= delta_drag.x;
        camera.position.y += delta_drag.y;

        let simulator = SIMULATOR.read().unwrap();

        simulator.pedestrians.iter().for_each(|p| {
            let pos = camera.scale * (vec2(p.pos.x, p.pos.y) - camera.position);
            ui.painter()
                .circle_filled(pos.to_pos2(), 0.125 * camera.scale, Color32::BLUE);
        });

        simulator.pedestrians.iter().for_each(|p| {
            let pos = camera.scale * (vec2(p.pos.x, p.pos.y) - camera.position) * 2.0;
            ui.painter()
                .circle_filled(pos.to_pos2(), 0.125 * camera.scale, Color32::RED);
        });
    }
}
