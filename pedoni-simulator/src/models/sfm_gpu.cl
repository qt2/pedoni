#define Q 16
#define R 0.1f
#define NC 128

inline float calc_potential(int id, float2 x, float dest_id_f,
                            float field_potential_unit,
                            image2d_array_t field_potential_grids,
                            sampler_t field_potential_sampler,
                            float neighbor_grid_unit, int2 neighbor_grid_shape,
                            uint *neighbor_grid_indices, float2 *positions) {
    float4 coord = (float4)(x / field_potential_unit - (float2)(0.5f, 0.5f),
                            dest_id_f, 0.0f);
    float u =
        read_imagef(field_potential_grids, field_potential_sampler, coord).x;

    int2 grid_id = convert_int2((float2)(x / neighbor_grid_unit));

    int y_start = max(grid_id.y - 1, 0);
    int y_end = min(grid_id.y + 1, neighbor_grid_shape.y - 1);
    int x_start = max(grid_id.x - 1, 0);
    int x_end = min(grid_id.x + 1, neighbor_grid_shape.x);

    for (int y = y_start; y <= y_end; y++) {
        int row_id = y * neighbor_grid_shape.x;
        for (int i = neighbor_grid_indices[row_id + x_start];
             i < neighbor_grid_indices[row_id + x_end + 1]; i++) {
            if (i != id) {
                float d = distance(x, positions[i]);
                if (d <= 0.4f) {
                    u += 1000.0f;
                } else if (d <= 1.4f) {
                    u += 0.4f * native_exp(-native_powr(d, 0.2f));
                }
            }
        }
    }

    return u;
}

inline float random(uint x) {
    int id = get_global_id(0);
    x += id << 8;

    // XOR Shift
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;

    return (float)(x & 0xFFFFFF) / 0x1000000; // normalization
}

__kernel void
calc_next_state(uint ped_count, __global float2 *positions,
                __global uint *destinations,
                read_only image2d_array_t field_potential_grids,
                sampler_t field_potential_sampler, float field_potential_unit,
                __global uint *neighbor_grid_indices, int2 neighbor_grid_shape,
                float neighbor_grid_unit, __global float2 *next_positions) {
    int id = get_global_id(0);
    int lid = get_local_id(0);

    if (id >= ped_count) {
        return;
    }

    float2 pos = positions[id];
    float dest_id_f = destinations[id];

    const float r_unit = 2.0 * M_PI_F / Q;
    uint seed = ped_count << 8;

    float2 best_x;
    float best_u = 1e24;

    for (uint i = 0; i <= Q; i++) {
        float2 x;

        if (i != Q) {
            float theta = r_unit * ((float)i + random(i + seed));
            x = pos + (float2){native_cos(theta), native_sin(theta)} * R;
        } else {
            x = pos;
        }

        float u = calc_potential(id, x, dest_id_f, field_potential_unit,
                                 field_potential_grids, field_potential_sampler,
                                 neighbor_grid_unit, neighbor_grid_shape,
                                 neighbor_grid_indices, positions);

        if (u < best_u) {
            best_u = u;
            best_x = x;
        }
    }

    next_positions[id] = best_x;
}
