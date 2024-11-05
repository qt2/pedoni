use eframe::egui::{self, Modifiers, RichText};
use egui_extras::Column;
use log::{error, info};
use pittore_egui::prelude::*;

use crate::{load_scenario, DIAGNOSTIC, SIMULATOR, STATE};

const COLORS: &[[u8; 4]] = &[
    [255, 0, 0, 255],
    [0, 0, 255, 255],
    [0, 255, 0, 255],
    [255, 0, 255, 255],
    [255, 255, 0, 255],
    [0, 255, 255, 255],
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
    fn apply_camera_transform(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        size: egui::Vec2,
    ) {
        let camera = &mut self.camera;

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
        let delta_drag = response.drag_delta() / camera.scale; // double it to follow wgpu's coordinate system

        camera.position.x -= delta_drag.x;
        camera.position.y += delta_drag.y;
    }
}

impl PittoreApp for FieldCanvas {
    fn update(&mut self, ctx: &mut pittore_egui::prelude::Context) {
        ctx.add_layer(|c| {
            c.add_camera(Camera::new_2d(self.camera.position, self.camera.scale));

            {
                let simulator = SIMULATOR.read().unwrap();

                let pedestrians = simulator
                    .pedestrians
                    .iter()
                    .filter(|ped| ped.active)
                    .map(|ped| Rect {
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
            let open_scenario_shortcut =
                egui::KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::O);

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(
                            egui::Button::new("Open scenario")
                                .shortcut_text(ui.ctx().format_shortcut(&open_scenario_shortcut)),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            info!("Loading scenario: {:?}", &path);
                            if let Err(err) = load_scenario(&path) {
                                error!("Failed to load scenario: {path:?}\n{err:?}");
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("Save control state")).clicked() {
                        crate::save_state();
                        ui.close_menu();
                    }
                });

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

        egui::SidePanel::right("right-panel").show(ctx, |ui| {
            egui::ScrollArea::new([false, true])
                .auto_shrink(false)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("Pedestrians")
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut table = egui_extras::TableBuilder::new(ui)
                                .column(Column::auto())
                                .column(Column::remainder());
                            table = table.sense(egui::Sense::click());

                            {
                                let pedestrians = &SIMULATOR.read().unwrap().scenario.pedestrians;
                                table.body(|body| {
                                    body.rows(20.0, pedestrians.len(), |mut row| {
                                        let index = row.index();
                                        let ped = &pedestrians[index];
                                        row.col(|ui| {
                                            ui.strong("🚶");
                                        });
                                        row.col(|ui| {
                                            ui.strong(format!(
                                                "#{} ({} -> {})",
                                                index, ped.origin, ped.destination
                                            ));
                                        });
                                    });
                                })
                            }
                        });
                });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.0))
            .show(ctx, |ui| {
                egui::Frame::canvas(ui.style())
                    .rounding(0.0)
                    .show(ui, |ui| {
                        let size = ui.available_size();
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());

                        let canvas = &mut self.field_canvas;
                        canvas.apply_camera_transform(ctx, &response, size);
                        canvas.show(ui, rect);
                    });
            });

        {
            let diagnostic = DIAGNOSTIC.lock().unwrap();
            let last_metrics = diagnostic.last();

            egui::Window::new("diagnostic")
                .open(&mut self.show_diagnostic)
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
                                    .range(0.1..=100.0),
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
                                    .range(0.1..=100.0),
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
