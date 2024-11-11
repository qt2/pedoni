use eframe::wgpu;

pub struct DPedestrian {}

pub struct DState {}

pub struct OSMKernel {
    pipeline: wgpu::ComputePipeline,
    pedestrian_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl OSMKernel {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("osm.wgsl"));

        let pedestrian_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pedestrian_buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            size: 128 * 65536,
            mapped_at_creation: false,
        });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_buffer"),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            size: 64 * 65536,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pedestrian_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pedestrian_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: pedestrian_buffer.as_entire_binding(),
            }],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pedestrian_pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        OSMKernel {
            pipeline,
            pedestrian_buffer,
            staging_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn prepare(&self, device: &wgpu::Device, queue: &wgpu::Queue) {}

    pub fn execute(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&{ wgpu::CommandEncoderDescriptor::default() });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.insert_debug_marker("compute_pedestrian");
            pass.dispatch_workgroups(128, 1, 1);
        }
        queue.submit([encoder.finish()]);
    }
}
