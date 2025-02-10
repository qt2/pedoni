use macroquad::prelude::*;

use crate::{CONTROL_STATE, SIMULATOR_STATE};

const COLORS: &[Color] = &[RED, BLUE, GREEN, SKYBLUE, MAGENTA, YELLOW];

pub fn run() {
    let conf = Conf {
        window_title: "Pedoni".into(),
        window_width: 800,
        window_height: 600,
        sample_count: 4,
        ..Default::default()
    };

    macroquad::Window::from_config(conf, render());
}

pub async fn render() {
    loop {
        // Handle input.
        {
            let mut state = CONTROL_STATE.lock().unwrap();

            if is_key_pressed(KeyCode::Space) {
                state.paused ^= true;
            }
            // if c.key_just_pressed(KeyCode::S) {
            //     let time = chrono::Local::now()
            //         .format("%Y-%m-%d_%H%M%S_screenshot.png")
            //         .to_string();
            //     c.save_screen_shot(format!("logs/{time}"));
            // }
        }

        // Render.
        clear_background(WHITE);

        let (width, height) = (screen_width(), screen_height());
        set_camera(&Camera2D {
            zoom: vec2(1.0, width / height) / 100.0,
            // target: vec2(field_size.x / 2.0, field_size.y / 2.0),
            ..Default::default()
        });

        {
            let simulator = SIMULATOR_STATE.lock().unwrap();

            // Draw obstacles.
            for obstacle in &simulator.scenario.obstacles {
                draw_line(
                    obstacle.line[0].x,
                    obstacle.line[0].y,
                    obstacle.line[1].x,
                    obstacle.line[1].y,
                    obstacle.width,
                    GRAY,
                );
            }

            // Draw waypoints.
            for waypoint in &simulator.scenario.waypoints {
                draw_line(
                    waypoint.line[0].x,
                    waypoint.line[0].y,
                    waypoint.line[1].x,
                    waypoint.line[1].y,
                    0.25,
                    ORANGE,
                );
            }

            // Draw pedestrians.
            for pedestrian in &simulator.pedestrians {
                draw_circle(
                    pedestrian.pos.x,
                    pedestrian.pos.y,
                    0.2,
                    COLORS[pedestrian.destination as usize % COLORS.len()],
                );
            }
        }

        next_frame().await;
    }
}

// pub struct Renderer {
//     camera: CameraController2d,
// }

// impl Renderer {
//     pub fn new(field_size: Vec2) -> Self {
//         Renderer {
//             camera: CameraController2d {
//                 camera: Camera {
//                     position: (field_size * 0.5, 0.0).into(),
//                     scaling_mode: ScalingMode::AutoMin(field_size),
//                     ..Camera::default_2d()
//                 },
//                 ..Default::default()
//             },
//         }
//     }
// }

// impl PittoreApp for Renderer {
//     fn init(&mut self, c: &mut Context) {
//         println!("Press [Space] to play/pause.");
//         c.set_background_color(Color::WHITE);
//     }

//     fn update(&mut self, c: &mut Context) {
//         // Handle input.
//         {
//             let mut state = CONTROL_STATE.lock().unwrap();
//             if c.key_just_pressed(KeyCode::Space) {
//                 state.paused ^= true;
//             }

//             if c.key_just_pressed(KeyCode::KeyS) {
//                 let time = chrono::Local::now()
//                     .format("%Y-%m-%d_%H%M%S_screenshot.png")
//                     .to_string();
//                 c.save_screen_shot(format!("logs/{time}"));
//             }
//         }

//         // Apply camera transform.
//         self.camera.update_and_apply(c);

//         let simulator = SIMULATOR_STATE.lock().unwrap();

//         // Draw obstacles.
//         let obstacles = simulator
//             .scenario
//             .obstacles
//             .iter()
//             .map(|obs| Line2d::new(obs.line[0], obs.line[1], obs.width, Color::GRAY));
//         c.draw_lines(obstacles);

//         // Draw waypoints.
//         let waypoints = simulator
//             .scenario
//             .waypoints
//             .iter()
//             .map(|wp| Line2d::new(wp.line[0], wp.line[1], 0.25, Color::from_rgb(255, 128, 0)));
//         c.draw_lines(waypoints);

//         // Draw pedestrians.
//         c.draw_circles(simulator.pedestrians.iter().map(|ped| Instance2d {
//             transform: Transform2d::from_translation(ped.pos).with_scale(Vec2::splat(0.2)),
//             color: COLORS[ped.destination % COLORS.len()],
//             ..Default::default()
//         }));
//     }
// }
