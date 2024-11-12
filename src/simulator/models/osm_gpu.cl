#define Q 16
#define R 0.03f

__kernel void
calc_next_state(__global float2 *positions, __global uint *destinations,
                __global float4 *waypoints, __global uint16 *neighbor_grid,
                uint2 neighbor_grid_shape, float neighbor_grid_unit,
                __global float2 *next_positions) {
    uint id = get_global_id(0);
    float2 pos = positions[id];
    uint dest_id = destinations[id];
    float2 dest = waypoints[dest_id].xy;

    local float2 neighbors[16 * 9];
    int neighbor_count = 0;
    int2 grid_id = convert_int2((float2)(pos / neighbor_grid_unit));
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            int2 t = grid_id + (int2)(x, y);
            if (t.x >= 0 && t.x < neighbor_grid_shape.y && t.y >= 0 &&
                t.y < neighbor_grid_shape.x) {
                uint16 others =
                    neighbor_grid[t.x + t.y * neighbor_grid_shape.x];
                for (int i = 0; i < 16; i++) {
                    uint oid = others[i];
                    if (oid != 0) {
                        neighbors[neighbor_count] = positions[oid];
                        neighbor_count++;
                    }
                }
            }
        }
    }

    float r_unit = 2.0 * M_PI_F / Q;
    float best_u = 1e10;
    float2 best_pos;
    for (uint i = 0; i < Q; i++) {
        float theta = r_unit * (float)i;
        float2 x = pos + (float2){cos(theta), sin(theta)} * R;

        float u_field = distance(x, dest);

        float u_ped = 0.0;
        for (int j = 0; j < neighbor_count; j++) {
            float d = distance(x, neighbors[j]);
            float u_j;
            if (d > 0.4 + 1.0) {
                u_j = 0.0;
            } else if (d <= 0.4) {
                u_j = 1000.0;
            } else {
                u_j = 0.4 * exp(-1.0 * powr(d, 0.2f));
            }
            u_ped += u_j;
        }

        float u = u_field + u_ped;
        if (u < best_u) {
            best_u = u;
            best_pos = x;
        }
    }

    next_positions[id] = best_pos;
}