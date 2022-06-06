#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct InfiniteGridMaterial {
    planar_rotation_matrix: mat3x3<f32>;
    origin: vec3<f32>;
    normal: vec3<f32>;
    scale: f32;
    x_axis_col: vec3<f32>;
    z_axis_col: vec3<f32>;
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

fn grid(plane_coords: vec2<f32>, scale: f32) -> vec4<f32> {
    let coord = plane_coords * scale; // use the scale variable to set the distance between the lines
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line = min(grid.x, grid.y);
    let minimumz = min(derivative.y, 1.);
    let minimumx = min(derivative.x, 1.);
    var color = vec4<f32>(0.2, 0.2, 0.2, 1.0 - min(line, 1.0));
    // z axis
    if (plane_coords.x > -0.5 * minimumx && plane_coords.x < 0.5 * minimumx) {
        color = vec4<f32>(material.z_axis_col, color.a);
    }
    // x axis
    if (plane_coords.y > -0.5 * minimumz && plane_coords.y < 0.5 * minimumz) {
        color = vec4<f32>(material.x_axis_col, color.a);
    }
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

    var out: FragmentOutput;
    out.color = grid(plane_coords, material.scale) * f32(t > 0.);
    out.depth = compute_depth(frag_pos_3d);
    return out;
}
