use glam::{vec2, Vec2};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub position: Vec2,
    pub size: Vec2,
    pub scale: f32,
    pub _padding: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec2::ZERO,
            size: vec2(640.0, 360.0),
            scale: 16.0,
            _padding: 0.0,
        }
    }
}
