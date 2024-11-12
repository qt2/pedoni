struct Pedestrian {
    pos: vec2f,
    destination: u32,
    is_active: u32,
}

struct State {
    pos: vec2f,
    // is_active: bool,
    // _padding: u32
}

@group(0) @binding(0) var<storage, read> pedestrians: array<Pedestrian>;
@group(0) @binding(1) var<storage, read_write> states: array<State>;

@compute @workgroup_size(64)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(global_invocation_id) global_invocation_id: vec3u,
) {
    let id = global_invocation_id.x;
    let pos = pedestrians[id];

    var state: State;
    state.pos = vec2(0.0, 0.0);

    states[id] = state;
}