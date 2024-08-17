@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct Camera {
    position: vec2<f32>,
    rect: vec2<f32>,
    scale: f32,    
}

@group(1) @binding(0)
var<uniform> camera: Camera;

struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct Instance {
    @location(3) position: vec2<f32>,
    @location(4) rect: vec4<f32>,
    @location(5) color: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,    
    @location(0) uv: vec2<f32>,
    @location(1) color: u32,
}

@vertex
fn vs_main(    
    vertex: Vertex,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;
    
    let position = model.position + instance.position;
    let relative_position = camera.scale * (position - camera.position) / camera.rect;
    
    out.position = vec4<f32>(relative_position, 0.0, 1.0);
    out.uv = instance.rect.xy + instance.rect.zw * model.uv;
    out.color = instance.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = unpack4x8unorm(in.color);
    return color * textureSample(t_diffuse, s_diffuse, in.uv);
    // return color;
}