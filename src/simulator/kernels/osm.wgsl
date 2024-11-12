struct Pedestrian {

}

struct State {
    
}

@group(0) @binding(0) var<storage, read> pedestrians: array<Pedestrian>;
@group(0) @binding(1) var<storage, write> states: array<State>;

@compute @workgroup_size(64)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(global_invocation_id) global_invocation_id: vec3u,
) {

}