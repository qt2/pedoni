use pittore::{prelude::*, util::CameraController2d};

use crate::{SIMULATOR, STATE};

const COLORS: &[Color] = &[
    Color::RED,
    Color::BLUE,
    Color::GREEN,
    Color::CYAN,
    Color::MAGENTA,
    Color::YELLOW,
];

pub struct Renderer {
    camera: CameraController2d,
}

impl Renderer {
    pub fn new(field_size: Vec2) -> Self {
        Renderer {
            camera: CameraController2d {
                camera: Camera {
                    position: (field_size * 0.5, 0.0).into(),
                    scaling_mode: ScalingMode::AutoMin(field_size),
                    ..Camera::default_2d()
                },
                ..Default::default()
            },
        }
    }
}

impl PittoreApp for Renderer {
    fn init(&mut self, _c: &mut Context) {
        println!("Press [Space] to play/pause.")
    }

    fn update(&mut self, c: &mut Context) {
        // Handle input.
        {
            let mut state = STATE.lock().unwrap();
            if c.key_just_pressed(KeyCode::Space) {
                state.paused ^= true;
            }
        }

        // Apply camera transform.
        self.camera.update_and_apply(c);

        let simulator = SIMULATOR.read().unwrap();

        // Draw obstacles.
        let obstacles = simulator
            .scenario
            .obstacles
            .iter()
            .map(|obs| Line2d::new(obs.line[0], obs.line[1], obs.width, Color::GRAY));
        c.draw_lines(obstacles);

        // Draw waypoints.
        let waypoints = simulator
            .scenario
            .waypoints
            .iter()
            .map(|wp| Line2d::new(wp.line[0], wp.line[1], 0.25, Color::YELLOW));
        c.draw_lines(waypoints);

        // Draw pedestrians.
        c.draw_circles(simulator.list_pedestrians().iter().map(|ped| Instance2d {
            transform: Transform2d::from_translation(ped.pos).with_scale(Vec2::splat(0.2)),
            color: COLORS[ped.destination % COLORS.len()],
            ..Default::default()
        }));
    }
}
