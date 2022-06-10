#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct InfiniteGridMaterial {
    planar_rotation_matrix: mat3x3<f32>;
    origin: vec3<f32>;
    normal: vec3<f32>;
    scale: f32;
    x_axis_col: vec3<f32>;
    z_axis_col: vec3<f32>;
    shadow_col: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> material: InfiniteGridMaterial;

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[builtin(vertex_index)]] index: u32;
};

fn inverse(m: mat4x4<f32>) -> mat4x4<f32> {
    let m00 = m[0][0];
    let m01 = m[0][1];
    let m02 = m[0][2];
    let m03 = m[0][3];
    let m10 = m[1][0];
    let m11 = m[1][1];
    let m12 = m[1][2];
    let m13 = m[1][3];
    let m20 = m[2][0];
    let m21 = m[2][1];
    let m22 = m[2][2];
    let m23 = m[2][3];
    let m30 = m[3][0];
    let m31 = m[3][1];
    let m32 = m[3][2];
    let m33 = m[3][3];

    let coef00 = m22 * m33 - m32 * m23;
    let coef02 = m12 * m33 - m32 * m13;
    let coef03 = m12 * m23 - m22 * m13;

    let coef04 = m21 * m33 - m31 * m23;
    let coef06 = m11 * m33 - m31 * m13;
    let coef07 = m11 * m23 - m21 * m13;

    let coef08 = m21 * m32 - m31 * m22;
    let coef10 = m11 * m32 - m31 * m12;
    let coef11 = m11 * m22 - m21 * m12;

    let coef12 = m20 * m33 - m30 * m23;
    let coef14 = m10 * m33 - m30 * m13;
    let coef15 = m10 * m23 - m20 * m13;

    let coef16 = m20 * m32 - m30 * m22;
    let coef18 = m10 * m32 - m30 * m12;
    let coef19 = m10 * m22 - m20 * m12;

    let coef20 = m20 * m31 - m30 * m21;
    let coef22 = m10 * m31 - m30 * m11;
    let coef23 = m10 * m21 - m20 * m11;

    let fac0 = vec4<f32>(coef00, coef00, coef02, coef03);
    let fac1 = vec4<f32>(coef04, coef04, coef06, coef07);
    let fac2 = vec4<f32>(coef08, coef08, coef10, coef11);
    let fac3 = vec4<f32>(coef12, coef12, coef14, coef15);
    let fac4 = vec4<f32>(coef16, coef16, coef18, coef19);
    let fac5 = vec4<f32>(coef20, coef20, coef22, coef23);

    let vecc0 = vec4<f32>(m10, m00, m00, m00);
    let vecc1 = vec4<f32>(m11, m01, m01, m01);
    let vecc2 = vec4<f32>(m12, m02, m02, m02);
    let vecc3 = vec4<f32>(m13, m03, m03, m03);

    let inv0 = (vecc1 * fac0) - (vecc2 * fac1) + (vecc3 * fac2);
    let inv1 = (vecc0 * fac0) - (vecc2 * fac3) + (vecc3 * fac4);
    let inv2 = (vecc0 * fac1) - (vecc1 * fac3) + (vecc3 * fac5);
    let inv3 = (vecc0 * fac2) - (vecc1 * fac4) + (vecc2 * fac5);

    let sign_a = vec4<f32>(1., -1., 1., -1.);
    let sign_b = vec4<f32>(-1., 1., -1., 1.);

    let inverse = mat4x4<f32>(
        inv0 * sign_a,
        inv1 * sign_b,
        inv2 * sign_a,
        inv3 * sign_b,
    );

    let col0 = vec4<f32>(
        inverse[0].x,
        inverse[1].x,
        inverse[2].x,
        inverse[3].x,
    );

    let dot0 = m[0] * col0;
    let dot1 = dot0.x + dot0.y + dot0.z + dot0.w;

    let rcp_det = 1. / dot1;
    return inverse * rcp_det;
}

fn unproject_point(point: vec3<f32>) -> vec3<f32> {
    let proj_inverse = inverse(view.projection);
    let unprojected = view.view * proj_inverse * vec4<f32>(point, 1.0);
    return unprojected.xyz / unprojected.w;
}

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] near_point: vec3<f32>;
    [[location(1)]] far_point: vec3<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    // 0 2 1 0 3 2
    var grid_plane = array<vec3<f32>, 4>(
        vec3<f32>(-1., -1., 1.),
        vec3<f32>(-1., 1., 1.),
        vec3<f32>(1., 1., 1.),
        vec3<f32>(1., -1., 1.),
    );
    let p = grid_plane[vertex.index].xyz;

    var out: VertexOutput;
    // out.clip_position = view.view_proj * vec4<f32>(vertex.position, 1.);
    out.clip_position = vec4<f32>(p, 1.);
    out.near_point = unproject_point(p);
    out.far_point = unproject_point(vec3<f32>(p.xy, 0.001)); // unprojecting on the far plane
    return out;
}

fn grid(real_coords: vec3<f32>, plane_coords: vec2<f32>, scale: f32, shadow: f32, real_depth: f32) -> vec4<f32> {
    let coord = plane_coords * scale; // use the scale variable to set the distance between the lines
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line = min(grid.x, grid.y);

    let minimumz = min(derivative.y, 1.) / scale;
    let minimumx = min(derivative.x, 1.) / scale;

    let base_alpha = 1.0 - min(line, 1.0);
    let dist_fadeout = min(1., 1. - material.scale * real_depth / 100.);
    let dot_fadeout = abs(dot(material.normal, normalize(view.world_position - real_coords)));
    let alpha_fadeout = mix(dist_fadeout, 1., dot_fadeout);
    let true_alpha = base_alpha * alpha_fadeout;

    var color = vec4<f32>(vec3<f32>(0.2), true_alpha);

    color = mix(color, material.shadow_col, 1. - shadow);

    let z_axis_cond = plane_coords.x > -0.5 * minimumx && plane_coords.x < 0.5 * minimumx;
    let x_axis_cond = plane_coords.y > -0.5 * minimumz && plane_coords.y < 0.5 * minimumz;

    color = mix(color, vec4<f32>(material.z_axis_col, color.a), f32(z_axis_cond));
    color = mix(color, vec4<f32>(material.x_axis_col, color.a), f32(x_axis_cond));

    return color;
}

fn compute_depth(pos: vec3<f32>) -> f32 {
    let clip_space_pos = view.projection * view.inverse_view * vec4<f32>(pos.xyz, 1.);
    return (clip_space_pos.z / clip_space_pos.w);
}

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
    [[builtin(frag_depth)]] depth: f32;
};

fn fetch_directional_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = lights.directional_lights[light_id];

    // The normal bias is scaled to the texel size.
    let normal_offset = light.shadow_normal_bias * surface_normal.xyz;
    let depth_offset = light.shadow_depth_bias * light.direction_to_light.xyz;
    let offset_position = vec4<f32>(frag_position.xyz + normal_offset + depth_offset, frag_position.w);

    let offset_position_clip = light.view_projection * offset_position;
    if (offset_position_clip.w <= 0.0) {
        return 1.0;
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    // No shadow outside the orthographic projection volume
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0 || any(offset_position_ndc > vec3<f32>(1.0))) {
        return 1.0;
    }

    // compute texture coordinates for shadow lookup, compensating for the Y-flip difference
    // between the NDC and texture coordinates
    let flip_correction = vec2<f32>(0.5, -0.5);
    let light_local = offset_position_ndc.xy * flip_correction + vec2<f32>(0.5, 0.5);

    let depth = offset_position_ndc.z;
    // do the lookup, using HW PCF and comparison
    // NOTE: Due to non-uniform control flow above, we must use the level variant of the texture
    // sampler to avoid use of implicit derivatives causing possible undefined behavior.
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompareLevel(directional_shadow_textures, directional_shadow_textures_sampler, light_local, depth);
#else
    return textureSampleCompareLevel(directional_shadow_textures, directional_shadow_textures_sampler, light_local, i32(light_id), depth);
#endif
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> FragmentOutput {
    let ray_origin = in.near_point;
    let ray_direction = normalize(in.far_point - in.near_point);
    let plane_normal = material.normal;
    let plane_origin = material.origin;

    let denominator = dot(ray_direction, plane_normal);
    let point_to_point = plane_origin - ray_origin;
    let t = dot(plane_normal, point_to_point) / denominator;
    let frag_pos_3d = ray_direction * t + ray_origin;

    let planar_offset = frag_pos_3d - plane_origin;
    let rotation_matrix = material.planar_rotation_matrix;
    let plane_coords = (rotation_matrix * planar_offset).xz;


    let view_space_pos = view.inverse_view * vec4<f32>(frag_pos_3d, 1.);
    let clip_space_pos = view.projection * view_space_pos;
    let clip_depth = clip_space_pos.z / clip_space_pos.w;
    let real_depth = -view_space_pos.z;

    var out: FragmentOutput;

    out.depth = clip_depth;

    let shadow = fetch_directional_shadow(0u, vec4<f32>(frag_pos_3d, 1.), plane_normal);
    out.color = grid(frag_pos_3d, plane_coords, material.scale, shadow, real_depth);

    return out;
}
