pub mod camera;
pub mod pedestrian;
pub mod polygon;
pub mod texture;

use camera::CameraResources;
use eframe::egui;
use polygon::{PolygonRenderCallback, PolygonRenderResources};

use self::camera::Camera;

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

        // let pedestrians = vec![Instance {
        //     position: [0.0, 0.0],
        //     color: Color::RED.into(),
        // }];

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            PolygonRenderCallback {
                camera: self.camera.clone(),
                polygons: vec![],
            },
        ));
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
