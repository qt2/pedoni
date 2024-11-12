__kernel void calc_next_state(__global float2 *positions,
                              __global float2 *next_positions) {
    uint id = get_global_id(0);
    float2 pos = positions[id];
    pos[0] += 0.01;
    next_positions[id] = pos;
}