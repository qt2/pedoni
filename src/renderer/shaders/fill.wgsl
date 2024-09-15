struct Camera {
    position: vec2<f32>,
    size: vec2<f32>,
    scale: f32,    
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Vertex {
    @location(0) position: vec2<f32>,    
}

struct Instance {
    @location(5) position: vec2<f32>,
    @location(6) scale: f32,
    // @location(6) rect: vec4<f32>,
    // @location(7) color: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // @location(0) color: u32,    
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    vertex: Vertex,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;
    let position = instance.scale * vertex.position + instance.position;
    let relative_position = camera.scale * (position - camera.position) / camera.size;
    out.position = vec4<f32>(relative_position, 0.0, 1.0);
    // out.color = instance.color;
    // out.uv = instance.rect.xy + instance.rect.zw * model.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // let color = unpack4x8unorm(in.color);
    // return textureSample(t_diffuse, s_diffuse, in.uv);
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

 