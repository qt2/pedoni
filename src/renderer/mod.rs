use eframe::egui::{self, RichText};
use pittore::prelude::*;

use crate::{SIMULATOR, STATE};

const COLORS: &[Color] = &[
    Color::RED,
    Color::BLUE,
    Color::GREEN,
    Color::YELLOW,
    Color::from_rgb(255, 0, 255),
    Color::from_rgb(255, 255, 0),
];

struct Camera2d {
    position: Vec2,
    scale: f32,
}

impl Default for Camera2d {
    fn default() -> Self {
        Camera2d {
            position: Vec2::ZERO,
            scale: 8.0,
        }
    }
}

#[derive(Default)]
pub struct FieldCanvas {
    camera: Camera2d,
}

impl FieldCanvas {
    fn apply_camera_transform(&mut self, ctx: &egui::Context, response: &egui::Response) {
        let camera = &mut self.camera;
        let size = response.intrinsic_size.unwrap();

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

        let delta_drag = response.drag_delta() / camera.scale; // double it to follow wgpu's coordinate system

        camera.position.x -= delta_drag.x;
        camera.position.y += delta_drag.y;
    }
}

impl PittoreApp for FieldCanvas {
    fn update(&mut self, ctx: &mut pittore::Context) {
        self.apply_camera_transform(ctx.egui_context, &ctx.egui_response);

        ctx.add_layer(|c| {
            c.add_camera(Camera::new_2d(self.camera.position, self.camera.scale));

            {
                let simulator = SIMULATOR.read().unwrap();

                let obstacles = simulator
                    .scenario
                    .obstacles
                    .iter()
                    .map(|obs| Line {
                        start: obs.line[0],
                        end: obs.line[1],
                        width: obs.width,
                        color: Color::GRAY,
                    })
                    .collect();
                c.draw_lines(obstacles);

                let waypoints = simulator
                    .scenario
                    .waypoints
                    .iter()
                    .map(|wp| Line {
                        start: wp.line[0],
                        end: wp.line[1],
                        width: 0.25,
                        color: Color::YELLOW,
                    })
                    .collect();
                c.draw_lines(waypoints);

                let pedestrians = simulator
                    .list_pedestrians()
                    .iter()
                    .filter(|ped| ped.active)
                    .map(|ped| Object {
                        position: ped.pos,
                        scale: Vec2::splat(0.4),
                        color: COLORS[ped.destination % COLORS.len()],
                        ..Default::default()
                    })
                    .collect();
                c.draw_rects(pedestrians);
            }
        });
    }
}

pub struct Renderer {
    field_canvas: PittoreCanvas<FieldCanvas>,
    show_controller: bool,
    show_diagnostic: bool,
}

impl Renderer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let field_canvas = PittoreCanvas::new(cc, FieldCanvas::default());

        Renderer {
            field_canvas,
            show_controller: true,
            show_diagnostic: true,
        }
    }
}

impl eframe::App for Renderer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("View", |ui| {
                    let mut show_button = |text: &str, value: &mut bool| {
                        if ui
                            .add(egui::Button::new(format!(
                                "{} {text}",
                                if *value { "✔" } else { "    " }
                            )))
                            .clicked()
                        {
                            *value ^= true;
                            ui.close_menu();
                        }
                    };

                    show_button("Controller", &mut self.show_controller);
                    show_button("Diagnostic", &mut self.show_diagnostic);
                })
            })
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.0))
            .show(ctx, |ui| {
                egui::Frame::canvas(ui.style())
                    .rounding(0.0)
                    .show(ui, |ui| {
                        self.field_canvas.show(ctx, ui);
                    });
            });

        {
            let simulator = SIMULATOR.read().unwrap();
            let metrics = simulator.step_metrics.lock().unwrap();

            egui::Window::new("diagnostic")
                .open(&mut self.show_diagnostic)
                .show(ctx, |ui| {
                    egui::Grid::new("diagnostic-grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Active Pedestrians");
                            ui.label(RichText::new(metrics.active_ped_count.to_string()).strong());
                            ui.end_row();

                            ui.label("Calc State Time");
                            ui.label(
                                RichText::new(format!("{:.4}s", metrics.time_calc_state)).strong(),
                            );
                            ui.end_row();

                            ui.label("Apply State Time");
                            ui.label(
                                RichText::new(format!("{:.4}s", metrics.time_apply_state)).strong(),
                            );
                            ui.end_row();
                        });
                    ui.end_row();
                });
        }

        {
            let mut state = STATE.lock().unwrap();
            egui::Window::new("controller")
                .open(&mut self.show_controller)
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
                                    .range(0.1..=1000.0),
                            );
                            ui.end_row();

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
