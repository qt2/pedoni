use glam::{Affine2, Mat2, Vec2};
use miniquad::{
    BufferId, BufferLayout, BufferSource, BufferType, BufferUsage, Pipeline, PipelineParams,
    RenderingBackend, ShaderMeta, ShaderSource, UniformBlockLayout, UniformDesc, UniformType,
    UniformsSource, VertexAttribute, VertexFormat, VertexStep,
};

pub struct RenderState {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    mesh_rectangle: Mesh,
    mesh_circle: Mesh,

    commands: Vec<Command>,
}

impl RenderState {
    pub fn new() -> Self {
        let mut ctx = miniquad::window::new_rendering_backend();

        let mesh_rectangle = Mesh::triangle_fan(
            &mut ctx,
            &[
                Vertex::new(-0.5, -0.5),
                Vertex::new(0.5, -0.5),
                Vertex::new(0.5, 0.5),
                Vertex::new(-0.5, 0.5),
            ],
        );
        let mesh_circle = Mesh::triangle_fan(
            &mut ctx,
            &(0..20)
                .map(|i| {
                    let angle = i as f32 / 20.0 * std::f32::consts::PI * 2.0;
                    Vertex::new(angle.cos(), angle.sin())
                })
                .collect::<Vec<_>>(),
        );

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX_SHADER,
                    fragment: FRAGMENT_SHADER,
                },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("view_translation", UniformType::Float2),
                            UniformDesc::new("view_scale", UniformType::Float2),
                        ],
                    },
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
                VertexAttribute::with_buffer("color", VertexFormat::Float4, 1),
            ],
            shader,
            PipelineParams::default(),
        );

        RenderState {
            ctx,
            pipeline,
            mesh_rectangle,
            mesh_circle,

            commands: Vec::new(),
        }
    }

    pub fn begin_pass(&mut self) {
        self.ctx
            .begin_default_pass(miniquad::PassAction::clear_color(1.0, 1.0, 1.0, 0.0));
    }

    pub fn end_pass(&mut self) {
        self.ctx.apply_pipeline(&self.pipeline);

        for command in &self.commands {
            match command {
                Command::SetView { target, scale } => {
                    self.ctx
                        .apply_uniforms(UniformsSource::table(&Uniform::new(*target, *scale)));
                }
                Command::Draw {
                    mesh,
                    instance_buffer,
                    num_instances,
                } => {
                    self.ctx.apply_bindings(&miniquad::Bindings {
                        vertex_buffers: vec![mesh.vertex_buffer, *instance_buffer],
                        index_buffer: mesh.index_buffer,
                        images: vec![],
                    });
                    self.ctx.draw(0, mesh.num_indices, *num_instances);
                }
            }
        }

        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        for command in &self.commands {
            if let Command::Draw {
                instance_buffer, ..
            } = command
            {
                self.ctx.delete_buffer(*instance_buffer);
            }
        }

        self.commands.clear();
    }

    pub fn set_view(&mut self, target: Vec2, scale: Vec2) {
        self.commands.push(Command::SetView { target, scale });
    }

    pub fn draw_rectangles(&mut self, instances: &[Instance]) {
        let instance_buffer = self.ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(instances),
        );

        self.commands.push(Command::Draw {
            mesh: self.mesh_rectangle.clone(),
            instance_buffer,
            num_instances: instances.len() as _,
        });
    }

    pub fn draw_circles(&mut self, instances: &[Instance]) {
        let instance_buffer = self.ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(instances),
        );

        self.commands.push(Command::Draw {
            mesh: self.mesh_circle.clone(),
            instance_buffer,
            num_instances: instances.len() as _,
        });
    }
}

pub enum Command {
    Draw {
        mesh: Mesh,
        instance_buffer: BufferId,
        num_instances: i32,
    },
    SetView {
        target: Vec2,
        scale: Vec2,
    },
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
    pub color: [f32; 4],
}

impl Instance {
    pub fn new(affine: Affine2, color: Color) -> Self {
        Instance {
            matrix2: affine.matrix2.to_cols_array(),
            translation: affine.translation.to_array(),
            color: color.0,
        }
    }

    pub fn from_line(start: Vec2, end: Vec2, width: f32, color: Color) -> Self {
        let diff = end - start;
        let Vec2 { x: cos, y: sin } = diff.normalize_or(Vec2::X);
        let affine = Affine2 {
            matrix2: Mat2::from_cols_array(&[cos, sin, -sin, cos])
                * Mat2::from_diagonal(Vec2::new(diff.length(), width)),
            translation: start + diff * 0.5,
        };
        Instance::new(affine, color)
    }
}

#[repr(C)]
pub struct Uniform {
    pub view_translation: [f32; 2],
    pub view_scale: [f32; 2],
}

impl Uniform {
    pub fn new(view_target: Vec2, view_scale: Vec2) -> Self {
        Uniform {
            view_translation: (-view_target).to_array(),
            view_scale: view_scale.to_array(),
        }
    }
}

#[derive(Clone)]
pub struct Mesh {
    pub vertex_buffer: BufferId,
    pub index_buffer: BufferId,
    pub num_indices: i32,
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
            num_indices: indices.len() as _,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Color(pub [f32; 4]);

#[allow(unused)]
impl Color {
    pub const RED: Self = Color::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Color::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Color::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Color::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Color::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Color::rgb(1.0, 0.0, 1.0);
    pub const ORANGE: Self = Color::rgb(1.0, 0.5, 0.0);
    pub const WHITE: Self = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Color::rgb(0.0, 0.0, 0.0);
    pub const GRAY: Self = Color::rgb(0.5, 0.5, 0.5);

    #[inline]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color([r, g, b, a])
    }

    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Color([r, g, b, 1.0])
    }
}

const VERTEX_SHADER: &str = r#"
    #version 140

    in vec2 position;

    in vec4 matrix2;
    in vec2 translation;
    in vec4 color;

    uniform vec2 view_translation;
    uniform vec2 view_scale;

    flat out vec4 v_color;    

    void main() {
        vec2 pos = translation + mat2(matrix2) * position;
        gl_Position = vec4((pos + view_translation) * view_scale, 0.0, 1.0);
        v_color = color;
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 140

    flat in vec4 v_color;
    out vec4 color;

    void main() {
        color = v_color;
    }
"#;
