
#define COS_PHI -0.17364817766693036f

const sampler_t SAMP =
    CLK_NORMALIZED_COORDS_FALSE | CLK_ADDRESS_CLAMP_TO_EDGE | CLK_FILTER_LINEAR;

inline float2 sobel(image2d_t grids, float2 coord) {
    float u00 = read_imagef(grids, SAMP, coord + (float2)(-1.0f, -1.0f)).x;
    float u01 = read_imagef(grids, SAMP, coord + (float2)(0.0f, -1.0f)).x;
    float u02 = read_imagef(grids, SAMP, coord + (float2)(1.0f, -1.0f)).x;
    float u10 = read_imagef(grids, SAMP, coord + (float2)(-1.0f, 0.0f)).x;
    float u12 = read_imagef(grids, SAMP, coord + (float2)(1.0f, 0.0f)).x;
    float u20 = read_imagef(grids, SAMP, coord + (float2)(-1.0f, 1.0f)).x;
    float u21 = read_imagef(grids, SAMP, coord + (float2)(0.0f, 1.0f)).x;
    float u22 = read_imagef(grids, SAMP, coord + (float2)(1.0f, 1.0f)).x;

    return (float2)(u00 + u10 + u10 + u20 - u02 - u12 - u12 - u22,
                    u00 + u01 + u01 + u02 - u20 - u21 - u21 - u22);
}

inline float2 sobel_array(image2d_array_t grids, float4 coord) {
    float u00 =
        read_imagef(grids, SAMP, coord + (float4)(-1.0f, -1.0f, 0.0f, 0.0f)).x;
    float u01 =
        read_imagef(grids, SAMP, coord + (float4)(0.0f, -1.0f, 0.0f, 0.0f)).x;
    float u02 =
        read_imagef(grids, SAMP, coord + (float4)(1.0f, -1.0f, 0.0f, 0.0f)).x;
    float u10 =
        read_imagef(grids, SAMP, coord + (float4)(-1.0f, 0.0f, 0.0f, 0.0f)).x;
    float u12 =
        read_imagef(grids, SAMP, coord + (float4)(1.0f, 0.0f, 0.0f, 0.0f)).x;
    float u20 =
        read_imagef(grids, SAMP, coord + (float4)(-1.0f, 1.0f, 0.0f, 0.0f)).x;
    float u21 =
        read_imagef(grids, SAMP, coord + (float4)(0.0f, 1.0f, 0.0f, 0.0f)).x;
    float u22 =
        read_imagef(grids, SAMP, coord + (float4)(1.0f, 1.0f, 0.0f, 0.0f)).x;

    return (float2)(u00 + u10 + u10 + u20 - u02 - u12 - u12 - u22,
                    u00 + u01 + u01 + u02 - u20 - u21 - u21 - u22);
}

// inline float random(uint x) {
//     int id = get_global_id(0);
//     x += id << 8;

//     // XOR Shift
//     x ^= x << 13;
//     x ^= x >> 17;
//     x ^= x << 5;

//     return (float)(x & 0xFFFFFF) / 0x1000000; // normalization
// }

__kernel void
calc_next_state(uint ped_count, __global float2 *positions,
                __global float2 *velocities, __global float *desired_speeds,
                __global uint *destinations,
                read_only image2d_array_t potential_map,
                read_only image2d_t distance_map, float field_unit,
                __global uint *neighbor_grid_indices, int2 neighbor_grid_shape,
                float neighbor_grid_unit, __global float2 *accelerations) {

    int id = get_global_id(0);
    if (id >= ped_count) {
        return;
    }

    float2 pos = positions[id];
    float2 vel = velocities[id];
    float desired_speed = desired_speeds[id];
    float dest_id = (float)destinations[id];

    float2 acc = (float2)(0.0f, 0.0f);

    // Calculate force toward the destination.
    float2 coord = pos / field_unit - (float2)(0.5f, 0.5f);
    float2 grad = sobel_array(potential_map, (float4)(coord, dest_id, 0.0f));
    float2 e = normalize(grad);
    acc += (e * desired_speed - vel) / 0.5f;

    // Calculate force from other pedestrians.
    int2 grid_id = convert_int2((float2)(pos / neighbor_grid_unit));

    int y_start = max(grid_id.y - 1, 0);
    int y_end = min(grid_id.y + 1, neighbor_grid_shape.y - 1);
    int x_start = max(grid_id.x - 1, 0);
    int x_end = min(grid_id.x + 1, neighbor_grid_shape.x);

    for (int y = y_start; y <= y_end; y++) {
        int row_id = y * neighbor_grid_shape.x;
        for (int i = neighbor_grid_indices[row_id + x_start];
             i < neighbor_grid_indices[row_id + x_end + 1]; i++) {
            if (i != id) {
                float2 difference = pos - positions[i];
                float distance = length(difference);

                if (distance <= 4.0f) {
                    float2 direction = normalize(difference);
                    float2 vel_i = velocities[i];
                    float2 t1 = difference - vel_i * 0.1f;
                    float t1_length = length(t1);
                    float t2 = distance + t1_length;
                    float t3 = length(vel_i) * 0.1f;
                    float b = native_sqrt(t2 * t2 - t3 * t3) * 0.5f;

                    float2 nabla_b =
                        t2 * (direction + t1 / t1_length) / (4.0f * b);
                    float2 force = 7.0f * native_exp(-b / 0.3f) * nabla_b;

                    if (dot(e, -force) < length(force) * COS_PHI) {
                        force *= 0.5f;
                    }

                    acc += force;
                }
            }
        }
    }

    // Calculate force from obstacles.
    float distance = read_imagef(distance_map, SAMP, coord).x;
    float2 direction = -normalize(sobel(distance_map, coord));
    acc += 2.0f * native_exp(-distance / 0.2f) * direction;

    accelerations[id] = acc;
}
