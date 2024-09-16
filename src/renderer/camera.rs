use glam::{vec2, Affine2, Mat2, Vec2};

#[derive(Debug, Clone)]
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Zeroable)]
pub struct View {
    pub matrix2: Mat2,
    pub translation: Vec2,
}

unsafe impl bytemuck::Pod for View {}

impl From<&Camera> for View {
    fn from(camera: &Camera) -> Self {
        let Affine2 {
            matrix2,
            translation,
        } = Affine2::from_scale(camera.scale / camera.size)
            * Affine2::from_translation(-camera.position);

        View {
            matrix2,
            translation,
        }
    }
}
