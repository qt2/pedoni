mod callback;
pub mod camera;
pub mod fill;

pub use callback::{DrawCommand, RenderCallback, RenderResources};
use camera::{Camera, View};
use eframe::{
    egui::{self, RichText},
    wgpu,
};
use fill::Instance;
use glam::vec2;

use crate::{DIAGNOSTIC, SIMULATOR, STATE};

const COLORS: &[[u8; 4]] = &[
    [255, 0, 0, 255],
    [0, 0, 255, 255],
    [0, 255, 0, 255],
    [255, 0, 255, 255],
    [255, 255, 0, 255],
    [0, 255, 255, 255],
];

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

        {
            let diagnostic = DIAGNOSTIC.lock().unwrap();
            let last_metrics = diagnostic.last();

            egui::Window::new("diagnostic")
                .open(&mut true)
                .show(ctx, |ui| {
                    egui::Grid::new("diagnostic-grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Active Pedestrians");
                            ui.label(
                                RichText::new(last_metrics.active_ped_count.to_string()).strong(),
                            );
                            ui.end_row();

                            ui.label("Calc State Time");
                            ui.label(
                                RichText::new(format!("{:.4}s", last_metrics.time_calc_state))
                                    .strong(),
                            );
                            ui.end_row();

                            ui.label("Apply State Time");
                            ui.label(
                                RichText::new(format!("{:.4}s", last_metrics.time_apply_state))
                                    .strong(),
                            );
                            ui.end_row();
                        });
                    ui.end_row();
                });
        }

        {
            let mut state = STATE.lock().unwrap();
            egui::Window::new("controller")
                .open(&mut true)
                .show(ctx, |ui| {
                    egui::Grid::new("controller-grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Play/Pause");
                            if ui
                                .button(if state.paused { "Play" } else { "Pause" })
                                .clicked()
                            {
                                state.paused ^= true;
                            }
                            ui.end_row();

                            ui.label("Playback Speed");
                            ui.add(
                                egui::DragValue::new(&mut state.playback_speed)
                                    .suffix("x")
                                    .speed(0.1)
                                    .clamp_range(0.1..=100.0),
                            );
                            ui.end_row();

                            ui.label("Use Neighbor Grid");
                            ui.checkbox(&mut state.use_neighbor_grid, "");
                            ui.end_row();

                            ui.label("Neighbor Grid Unit");
                            ui.add(
                                egui::DragValue::new(&mut state.neighbor_grid_unit)
                                    .suffix("m")
                                    .speed(0.1)
                                    .clamp_range(0.1..=100.0),
                            );
                            ui.end_row();

                            // ui.label("Use Neighbor Grid");
                            // ui.checkbox(&mut state.use_neighbor_grid, "Use Neighbor Grid")

                            ui.label("Theme");
                            ui.horizontal(|ui| {
                                if ui.button("toggle").clicked() {
                                    ctx.set_visuals(if ui.visuals().dark_mode {
                                        egui::Visuals::light()
                                    } else {
                                        egui::Visuals::dark()
                                    });
                                }
                            });
                            ui.end_row();
                        });
                    ui.end_row();
                });

            if !state.paused {
                ctx.request_repaint();
            }
        }
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

        if let Some(mouse_pos) = response.hover_pos() {
            let delta_wheel_y = ctx.input(|i| i.smooth_scroll_delta).y;

            if delta_wheel_y != 0.0 {
                let d = mouse_pos - size / 2.0;
                let d = vec2(d.x, -d.y) / camera.scale;
                let scale_mul = 2.0_f32.powf(delta_wheel_y * 0.01);
                camera.scale *= scale_mul;
                camera.position += d * (1.0 - scale_mul.recip());
            }
        }

        // let dpi = ctx.native_pixels_per_point().unwrap_or_default();
        let delta_drag = response.drag_delta() / camera.scale; // double it for matching wgpu's coordinate system

        camera.position.x -= delta_drag.x;
        camera.position.y += delta_drag.y;

        let commands = {
            let simulator = SIMULATOR.read().unwrap();

            let mut commands = simulator.static_draw_commands.clone();

            let instances = simulator
                .pedestrians
                .iter()
                .filter(|ped| ped.active)
                .map(|ped| Instance::point(ped.pos, COLORS[ped.destination % COLORS.len()]))
                .collect();
            commands.push(DrawCommand {
                mesh_id: 4,
                instances,
            });

            commands
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RenderCallback {
                view: View::from(&self.camera),
                commands,
            },
        ));
    }
}

pub struct PipelineSet {
    pub pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
}
