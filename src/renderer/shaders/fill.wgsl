struct View {
    matrix2: mat2x2<f32>,
    translation: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> view: View;

struct Vertex {
    @location(0) position: vec2<f32>,    
}

struct Instance {
    @location(5) position: vec2<f32>,
    @location(6) scale: f32,    
    @location(7) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    vertex: Vertex,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;
    let position = instance.scale * vertex.position + instance.position;    
    let relative_position = view.matrix2 * position + view.translation;
    out.position = vec4<f32>(relative_position, 0.0, 1.0);
    out.color = instance.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

 