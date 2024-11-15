#define Q 16
#define R 0.03f
#define NC 128
#define LGS 8

__kernel void
calc_next_state(uint ped_count, __global float2 *positions,
                __global uint *destinations, __constant float4 *waypoints,
                read_only image2d_array_t field_potential_grids,
                sampler_t field_potential_sampler, float field_potential_unit,
                __global uint *neighbor_grid_data,
                __global uint *neighbor_grid_indices, uint2 neighbor_grid_shape,
                float neighbor_grid_unit, __global float2 *next_positions) {
    int id = get_global_id(0);
    int lid = get_local_id(0);

    if (id >= ped_count) {
        return;
    }

    float2 pos = positions[id];
    uint dest_id = destinations[id];
    float2 dest = waypoints[dest_id].xy;

    local float2 neighbors[NC * LGS];

    int neighbor_count = 0;
    int neighbor_offset = NC * lid;
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
            neighbors[neighbor_offset + neighbor_count] = positions[oid];
            neighbor_count++;
        }
    }

    float r_unit = 2.0 * M_PI_F / Q;
    float best_u = 1e10f;
    float2 best_pos;
    for (uint i = 0; i < Q; i++) {
        float theta = r_unit * (float)i;
        float2 x = pos + (float2){native_cos(theta), native_sin(theta)} * R;

        float4 coord = (float4)(x / field_potential_unit, (float)dest_id, 0.0f);
        float u =
            read_imagef(field_potential_grids, field_potential_sampler, coord)
                .x;

        for (int j = 0; j < neighbor_count; j++) {
            float d = distance(x, neighbors[neighbor_offset + j]);
            if (d <= 0.4f) {
                u += 1000.0f;
            } else if (d <= 1.4f) {
                u += 0.4f * native_exp(-native_powr(d, 0.2f));
            }
        }

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