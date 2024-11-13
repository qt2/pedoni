#define Q 16
#define R 0.03f
#define NC 256

__kernel void
calc_next_state(__global float2 *positions, __global uint *destinations,
                __global float4 *waypoints, __global uint *neighbor_grid_data,
                __global uint *neighbor_grid_indices, uint2 neighbor_grid_shape,
                float neighbor_grid_unit, __global float2 *next_positions) {
    uint id = get_global_id(0);
    float2 pos = positions[id];
    uint dest_id = destinations[id];
    float2 dest = waypoints[dest_id].xy;

    local float2 neighbors[NC];
    int neighbor_count = 0;
    int2 grid_id = convert_int2((float2)(pos / neighbor_grid_unit));

    int y_start = max(grid_id.y - 1, 0);
    int y_end = min(grid_id.y + 1, (int)neighbor_grid_shape.y - 1);
    for (int y = y_start; y <= y_end; y++) {
        int row_id = y * (int)neighbor_grid_shape.x;
        int x_start = max(grid_id.x - 1, 0);
        int x_end = min(grid_id.x + 1, (int)neighbor_grid_shape.x);
        for (int i = neighbor_grid_indices[row_id + x_start];
             i < neighbor_grid_indices[row_id + x_end + 1]; i++) {
            if (neighbor_count >= NC) {
                break;
            }
            uint oid = neighbor_grid_data[i];
            neighbors[neighbor_count] = positions[oid];
            neighbor_count++;
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