use glam::{vec2, Affine2, Mat2};
use miniquad::{
    Bindings, BufferId, BufferLayout, BufferSource, BufferType, BufferUsage, EventHandler,
    PassAction, Pipeline, PipelineParams, RenderingBackend, ShaderId, ShaderMeta, ShaderSource,
    UniformBlockLayout, VertexAttribute, VertexFormat, VertexStep,
};

use crate::{CONTROL_STATE, SIMULATOR_STATE};

// const COLORS: &[Color] = &[RED, BLUE, GREEN, SKYBLUE, MAGENTA, YELLOW];

pub struct Renderer {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    mesh_rect: Mesh,
}

impl Renderer {
    pub fn new() -> Self {
        let mut ctx = miniquad::window::new_rendering_backend();

        let mesh_rect = Mesh::triangle_fan(
            &mut ctx,
            &[
                Vertex::new(-0.5, -0.5),
                Vertex::new(0.5, -0.5),
                Vertex::new(0.5, 0.5),
                Vertex::new(-0.5, 0.5),
            ],
        );

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX_SHADER,
                    fragment: FRAGMENT_SHADER,
                },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout { uniforms: vec![] },
                },
            )
            .unwrap();

        let pipeline = ctx.new_pipeline(
            &[
                BufferLayout::default(),
                BufferLayout {
                    step_func: VertexStep::PerInstance,
                    ..Default::default()
                },
            ],
            &[
                VertexAttribute::with_buffer("position", VertexFormat::Float2, 0),
                VertexAttribute::with_buffer("matrix2", VertexFormat::Float4, 1),
                VertexAttribute::with_buffer("translation", VertexFormat::Float2, 1),
            ],
            shader,
            PipelineParams::default(),
        );

        Renderer {
            ctx,
            pipeline,
            mesh_rect,
        }
    }
}

impl EventHandler for Renderer {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let c = &mut self.ctx;

        let instances = vec![
            Instance::new(Affine2::IDENTITY),
            Instance::new(Affine2::from_scale_angle_translation(
                vec2(0.4, 0.8),
                0.2,
                vec2(0.5, 0.5),
            )),
        ];
        let instance_buffer = c.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&instances),
        );

        c.begin_default_pass(PassAction::clear_color(1.0, 1.0, 1.0, 0.0));

        c.apply_pipeline(&self.pipeline);
        c.apply_bindings(&Bindings {
            vertex_buffers: vec![self.mesh_rect.vertex_buffer, instance_buffer],
            index_buffer: self.mesh_rect.index_buffer,
            images: vec![],
        });
        c.draw(0, 6, instances.len() as _);

        c.end_render_pass();

        c.commit_frame();

        // c.delete_buffer(instance_buffer);
    }
}

#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl Vertex {
    pub fn new(x: f32, y: f32) -> Self {
        Vertex { position: [x, y] }
    }
}

#[repr(C)]
pub struct Instance {
    pub matrix2: [f32; 4],
    pub translation: [f32; 2],
}

impl Instance {
    pub fn new(affine: Affine2) -> Self {
        Instance {
            matrix2: affine.matrix2.to_cols_array(),
            translation: affine.translation.to_array(),
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: BufferId,
    pub index_buffer: BufferId,
}

impl Mesh {
    pub fn triangle_fan(ctx: &mut Box<dyn RenderingBackend>, vertices: &[Vertex]) -> Self {
        let indices = (0..vertices.len() as u16 - 2)
            .flat_map(|i| [0, i + 1, i + 2])
            .collect::<Vec<u16>>();
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        Mesh {
            vertex_buffer,
            index_buffer,
        }
    }
}

const VERTEX_SHADER: &str = r#"
    #version 140

    in vec2 position;

    in vec4 matrix2;
    in vec2 translation;

    void main() {
        gl_Position = vec4(translation + mat2(matrix2) * position, 0.0, 1.0);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 140

    out vec4 color;

    void main() {
        color = vec4(1.0, 0.0, 0.0, 1.0);
    }
"#;

pub fn run() {
    let conf = miniquad::conf::Conf {
        window_title: "Pedoni".into(),
        window_width: 800,
        window_height: 600,
        icon: None,
        // sample_count: 4,
        ..Default::default()
    };

    miniquad::start(conf, move || Box::new(Renderer::new()));
}

// pub fn run() {
//     let conf = Conf {
//         window_title: "Pedoni".into(),
//         window_width: 800,
//         window_height: 600,
//         sample_count: 4,
//         ..Default::default()
//     };

//     macroquad::Window::from_config(conf, render());
// }

// pub async fn render() {
//     loop {
//         // Handle input.
//         {
//             let mut state = CONTROL_STATE.lock().unwrap();

//             if is_key_pressed(KeyCode::Space) {
//                 state.paused ^= true;
//             }
//             // if c.key_just_pressed(KeyCode::S) {
//             //     let time = chrono::Local::now()
//             //         .format("%Y-%m-%d_%H%M%S_screenshot.png")
//             //         .to_string();
//             //     c.save_screen_shot(format!("logs/{time}"));
//             // }
//         }

//         // Render.
//         clear_background(WHITE);

//         let (width, height) = (screen_width(), screen_height());
//         set_camera(&Camera2D {
//             zoom: vec2(1.0, width / height) / 100.0,
//             // target: vec2(field_size.x / 2.0, field_size.y / 2.0),
//             ..Default::default()
//         });

//         {
//             let simulator = SIMULATOR_STATE.lock().unwrap();

//             // Draw obstacles.
//             for obstacle in &simulator.scenario.obstacles {
//                 draw_line(
//                     obstacle.line[0].x,
//                     obstacle.line[0].y,
//                     obstacle.line[1].x,
//                     obstacle.line[1].y,
//                     obstacle.width,
//                     GRAY,
//                 );
//             }

//             // Draw waypoints.
//             for waypoint in &simulator.scenario.waypoints {
//                 draw_line(
//                     waypoint.line[0].x,
//                     waypoint.line[0].y,
//                     waypoint.line[1].x,
//                     waypoint.line[1].y,
//                     0.25,
//                     ORANGE,
//                 );
//             }

//             // Draw pedestrians.
//             for pedestrian in &simulator.pedestrians {
//                 draw_circle(
//                     pedestrian.pos.x,
//                     pedestrian.pos.y,
//                     0.2,
//                     COLORS[pedestrian.destination as usize % COLORS.len()],
//                 );
//             }
//         }

//         next_frame().await;
//     }
// }

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
