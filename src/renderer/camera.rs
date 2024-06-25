#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub position: [f32; 2],
    pub rect: [f32; 2],
    pub scale: f32,
    pub _padding: u32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: [0.0, 0.0],
            rect: [320.0, 240.0],
            scale: 16.0,
            _padding: 0,
        }
    }
}
