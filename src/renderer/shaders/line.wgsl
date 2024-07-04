struct Camera {
    position: vec2<f32>,
    rect: vec2<f32>,
    scale: f32,    
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(3) pos_a: vec2<f32>,
    @location(4) pos_b: vec2<f32>,
    @location(5) width: f32,
    @location(6) color: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: u32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let diff = pos_b - pos_a;
    let diff_norm = normalize(diff);
    let w_half = 0.5 * width;
    let offset_sign = f32(i32(index & 1) << 1 - 1);

    let position = pos_a * f32(1 - ((index >> 1) & 1))
        + pos_b * f32((index >> 1) & 1)
        + w_half * offset_sign * vec2<f32>(-diff_norm.y, diff_norm.x);
    let relative_position = camera.scale * (position - camera.position) / camera.rect;
    out.clip_position = vec4<f32>(relative_position, 0.0, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = unpack4x8unorm(in.color);
    return color;
}