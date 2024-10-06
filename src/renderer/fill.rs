use eframe::wgpu;
use glam::{vec2, Mat2, Vec2};

use super::PipelineSet;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: Vec2,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2,];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable)]
pub struct Instance {
    pub matrix2: Mat2,
    pub translation: Vec2,
    pub color: [u8; 4],
}

unsafe impl bytemuck::Pod for Instance {}

impl Instance {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x2, 7 => Unorm8x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

impl Instance {
    pub fn point(position: Vec2, color: [u8; 4]) -> Self {
        Instance {
            matrix2: Mat2::IDENTITY,
            translation: position,
            color,
        }
    }

    pub fn line_segment(points: [Vec2; 2], width: f32, color: [u8; 4]) -> Self {
        let translation = (points[0] + points[1]) / 2.0;
        let y_axis = points[1] - points[0];
        let height = y_axis.length();
        let scale = vec2(width, height);
        let Vec2 { x: ms, y: c } = y_axis / height;

        let matrix2 =
            Mat2::from_cols_array(&[c * scale.x, -ms * scale.x, ms * scale.y, c * scale.y]); // in column major

        Instance {
            matrix2,
            translation,
            color,
        }
    }
}

pub fn setup_fill_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> PipelineSet {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/fill.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&camera_bind_group_layout],
        ..Default::default()
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fill_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc(), Instance::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 4,
            ..Default::default()
        },
        multiview: None,
        cache: None,
    });

    PipelineSet {
        pipeline_layout,
        pipeline,
    }
}

#[cfg(test)]
mod tests {
    use glam::vec2;

    use super::Instance;

    #[test]
    fn test_line_segments() {
        let instance =
            Instance::line_segment([vec2(1.0, 1.0), vec2(5.0, 5.0)], 2.0f32.sqrt(), [255; 4]);

        let p0 = instance.matrix2 * vec2(0.5, 0.5) + instance.translation;
        dbg!(instance, p0);
    }
}
