mod state;

use glam::{vec2, Affine2, Mat2, Vec2};
use miniquad::{EventHandler, KeyCode};
use state::{Color, Instance, RenderState};

use crate::{CONTROL_STATE, SIMULATOR_STATE};

const COLORS: &[Color] = &[
    Color::RED,
    Color::BLUE,
    Color::GREEN,
    Color::CYAN,
    Color::MAGENTA,
    Color::YELLOW,
];

pub struct Renderer {
    state: RenderState,
    view_target: Vec2,
    view_scale: f32,
    prev_cursor_pos: Vec2,
    cursor_pos: Vec2,
    mouse_left_down: bool,
    mouse_center_down: bool,
    wheel_delta: f32,
}

impl Renderer {
    pub fn new() -> Self {
        let size = SIMULATOR_STATE.lock().unwrap().scenario.field.size;
        let view_target = size * 0.5;
        let view_scale = size.x.max(size.y).recip();

        Renderer {
            state: RenderState::new(),
            view_target,
            view_scale,
            prev_cursor_pos: Vec2::ZERO,
            cursor_pos: Vec2::ZERO,
            mouse_left_down: false,
            mouse_center_down: false,
            wheel_delta: 0.0,
        }
    }
}

impl EventHandler for Renderer {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let (width, height) = miniquad::window::screen_size();

        // Handle camera movement.
        self.view_scale *= 2.0_f32.powf(self.wheel_delta / 512.0);
        self.wheel_delta = 0.0;

        let mut cursor_delta = self.cursor_pos - self.prev_cursor_pos;
        cursor_delta.y = -cursor_delta.y;
        self.prev_cursor_pos = self.cursor_pos;

        if self.mouse_center_down || self.mouse_left_down {
            self.view_target -= cursor_delta * 2.0 / (self.view_scale * width);
        }

        // Render.
        let state = &mut self.state;

        state.begin_pass();
        state.set_view(
            self.view_target,
            vec2(1.0, width / height) * self.view_scale,
        );

        {
            let simulator = SIMULATOR_STATE.lock().unwrap();

            // Draw obstacles.
            state.draw_rectangles(
                &simulator
                    .scenario
                    .obstacles
                    .iter()
                    .map(|obs| {
                        Instance::from_line(obs.line[0], obs.line[1], obs.width, Color::GRAY)
                    })
                    .collect::<Vec<_>>(),
            );

            // Draw waypoints.
            state.draw_rectangles(
                &simulator
                    .scenario
                    .waypoints
                    .iter()
                    .map(|wp| Instance::from_line(wp.line[0], wp.line[1], 0.25, Color::ORANGE))
                    .collect::<Vec<_>>(),
            );

            // Draw pedestrians.
            state.draw_circles(
                &simulator
                    .pedestrians
                    .iter()
                    .map(|ped| {
                        Instance::new(
                            Affine2::from_mat2_translation(
                                Mat2::from_diagonal(Vec2::splat(0.2)),
                                ped.pos,
                            ),
                            COLORS[ped.destination as usize % COLORS.len()],
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }

        state.end_pass();
    }

    fn key_down_event(
        &mut self,
        keycode: miniquad::KeyCode,
        _keymods: miniquad::KeyMods,
        repeat: bool,
    ) {
        if !repeat {
            match keycode {
                KeyCode::Space => {
                    let mut state = CONTROL_STATE.lock().unwrap();
                    state.paused ^= true;
                }
                _ => {}
            }
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.wheel_delta += y;
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.cursor_pos = vec2(x, y);
    }

    fn mouse_button_down_event(&mut self, button: miniquad::MouseButton, _x: f32, _y: f32) {
        match button {
            miniquad::MouseButton::Left => {
                self.mouse_left_down = true;
            }
            miniquad::MouseButton::Middle => {
                self.mouse_center_down = true;
            }
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, button: miniquad::MouseButton, _x: f32, _y: f32) {
        match button {
            miniquad::MouseButton::Left => {
                self.mouse_left_down = false;
            }
            miniquad::MouseButton::Middle => {
                self.mouse_center_down = false;
            }
            _ => {}
        }
    }
}

pub fn run() {
    let conf = miniquad::conf::Conf {
        window_title: "Pedoni".into(),
        window_width: 800,
        window_height: 600,
        icon: None,
        sample_count: 4,
        ..Default::default()
    };

    miniquad::start(conf, move || Box::new(Renderer::new()));
}
