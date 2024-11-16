#define Q 16
#define R 0.1f
#define NC 128
// #define LGS 8

inline float calc_potential(float2 x, float dest_id_f,
                            float field_potential_unit,
                            image2d_array_t field_potential_grids,
                            sampler_t field_potential_sampler,
                            float neighbor_grid_unit, uint2 neighbor_grid_shape,
                            uint *neighbor_grid_indices, float2 *positions) {
    float4 coord = (float4)(x / field_potential_unit - (float2)(0.5f, 0.5f),
                            dest_id_f, 0.0f);
    float u =
        read_imagef(field_potential_grids, field_potential_sampler, coord).x;

    // for (int j = 0; j < neighbor_count; j++) {
    //     float d = distance(x, neighbors[neighbor_offset + j]);
    //     if (d <= 0.4f) {
    //         u += 1000.0f;
    //     } else if (d <= 1.4f) {
    //         u += 0.4f * native_exp(-native_powr(d, 0.2f));
    //     }
    // }

    int2 grid_id = convert_int2((float2)(x / neighbor_grid_unit));

    int y_start = max(grid_id.y - 1, 0);
    int y_end = min(grid_id.y + 1, (int)neighbor_grid_shape.y - 1);
    for (int y = y_start; y <= y_end; y++) {
        int row_id = y * (int)neighbor_grid_shape.x;
        int x_start = max(grid_id.x - 1, 0);
        int x_end = min(grid_id.x + 1, (int)neighbor_grid_shape.x);
        for (int i = neighbor_grid_indices[row_id + x_start];
             i < neighbor_grid_indices[row_id + x_end + 1]; i++) {
            float d = distance(x, positions[i]);
            if (d <= 0.4f) {
                u += 1000.0f;
            } else if (d <= 1.4f) {
                u += 0.4f * native_exp(-native_powr(d, 0.2f));
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
                // __global uint *neighbor_grid_data,
                __global uint *neighbor_grid_indices, uint2 neighbor_grid_shape,
                float neighbor_grid_unit, __global float2 *next_positions) {
    int id = get_global_id(0);
    int lid = get_local_id(0);

    if (id >= ped_count) {
        return;
    }

    float2 pos = positions[id];
    float dest_id_f = destinations[id];

    // local float2 neighbors[NC * LGS];

    // int neighbor_count = 0;
    // int neighbor_offset = NC * lid;

    // int y_start = max(grid_id.y - 1, 0);
    // int y_end = min(grid_id.y + 1, (int)neighbor_grid_shape.y - 1);
    // for (int y = y_start; y <= y_end; y++) {
    //     int row_id = y * (int)neighbor_grid_shape.x;
    //     int x_start = max(grid_id.x - 1, 0);
    //     int x_end = min(grid_id.x + 1, (int)neighbor_grid_shape.x);
    //     for (int i = neighbor_grid_indices[row_id + x_start];
    //          i < neighbor_grid_indices[row_id + x_end + 1]; i++) {
    //         if (neighbor_count >= NC) {
    //             break;
    //         }
    //         neighbors[neighbor_offset + neighbor_count] = positions[i];
    //         neighbor_count++;
    //     }
    // }

    float r_unit = 2.0 * M_PI_F / Q;
    float2 best_pos = pos;
    float best_u = calc_potential(
        pos, dest_id_f, field_potential_unit, field_potential_grids,
        field_potential_sampler, neighbor_grid_unit, neighbor_grid_shape,
        neighbor_grid_indices, positions);
    uint seed = ped_count << 8;

    for (uint i = 0; i < Q; i++) {
        float theta = r_unit * ((float)i + random(i + seed));
        float2 x = pos + (float2){native_cos(theta), native_sin(theta)} * R;
        float u = calc_potential(x, dest_id_f, field_potential_unit,
                                 field_potential_grids, field_potential_sampler,
                                 neighbor_grid_unit, neighbor_grid_shape,
                                 neighbor_grid_indices, positions);

        if (u < best_u) {
            best_u = u;
            best_pos = x;
        }
    }

    // float r_unit = 2.0 * M_PI_F / Q;
    // float2 xs[Q];
    // float us[Q] = {0.0f};

    // for (int i = 0; i < Q; i++) {
    //     float theta = r_unit * (float)i;
    //     float2 x = pos + (float2){native_cos(theta), native_sin(theta)} * R;
    //     xs[i] = x;
    //     float4 coord = (float4)(x / field_potential_unit, (float)dest_id,
    //     0.0f); us[i] =
    //         read_imagef(field_potential_grids, field_potential_sampler,
    //         coord)
    //             .x;
    // }

    // for (int j = 0; j < neighbor_count; j++) {
    //     float2 neighbor_pos = neighbors[neighbor_offset + j];

    //     for (int i = 0; i < Q; i++) {
    //         float d = distance(xs[i], neighbor_pos);
    //         if (d <= 0.4f) {
    //             us[i] += 1000.0f;
    //         } else if (d <= 1.4f) {
    //             us[i] += 0.4f * native_exp(-native_powr(d, 0.2f));
    //         }
    //     }
    // }

    // float best_u = 1e10f;
    // float2 best_pos;

    // for (int i = 0; i < Q; i++) {
    //     if (us[i] < best_u) {
    //         best_u = us[i];
    //         best_pos = xs[i];
    //     }
    // }

    next_positions[id] = best_pos;
}
