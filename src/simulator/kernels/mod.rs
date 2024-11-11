use eframe::wgpu;

fn setup(device: &wgpu::Device) {
    let pedestrian_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pedestrian_buffer"),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        size: 128 * 65536,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("pedestrian_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: 
            }
        ]
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pedestrian_bind_group"),
        layout: 
    });
}