use eframe::wgpu;

use super::{camera::Camera, fill, PipelineSet};

pub struct RenderResources {
    camera_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    fill_pipeline: PipelineSet,
}

pub struct RenderCallback {
    pub camera: Camera,
    pub instances: Vec<fill::Instance>,
}

impl egui_wgpu::CallbackTrait for RenderCallback {
    fn prepare(
        &self,
        device: &eframe::wgpu::Device,
        queue: &eframe::wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut eframe::wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<eframe::wgpu::CommandBuffer> {
        let resources: &RenderResources = callback_resources.get().unwrap();

        egui_encoder.

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        info: eframe::egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'a>,
        callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
    }
}
