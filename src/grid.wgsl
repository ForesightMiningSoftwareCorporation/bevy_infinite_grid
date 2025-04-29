struct InfiniteGridPosition {
    planar_rotation_matrix: mat3x3<f32>,
    origin: vec3<f32>,
    normal: vec3<f32>,

};

struct InfiniteGridSettings {
    scale: f32,
    // 1 / fadeout_distance
    dist_fadeout_const: f32,
    dot_fadeout_const: f32,
    x_axis_col: vec3<f32>,
    z_axis_col: vec3<f32>,
    minor_line_col: vec4<f32>,
    major_line_col: vec4<f32>,

};

struct View {
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    world_position: vec3<f32>,
};

@group(0) @binding(0) var<uniform> view: View;

@group(1) @binding(0) var<uniform> grid_position: InfiniteGridPosition;
@group(1) @binding(1) var<uniform> grid_settings: InfiniteGridSettings;

struct Vertex {
    @builtin(vertex_index) index: u32,
};

fn unproject_point(p: vec3<f32>) -> vec3<f32> {
    let unprojected = view.view * view.inverse_projection * vec4<f32>(p, 1.0);
    return unprojected.xyz / unprojected.w;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) near_point: vec3<f32>,
    @location(1) far_point: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    // 0 1 2 1 2 3
    var grid_plane = array<vec3<f32>, 4>(
        vec3<f32>(-1., -1., 1.),
        vec3<f32>(-1., 1., 1.),
        vec3<f32>(1., -1., 1.),
        vec3<f32>(1., 1., 1.)
    );
    let p = grid_plane[vertex.index].xyz;

    var out: VertexOutput;

    out.clip_position = vec4<f32>(p, 1.);
    out.near_point = unproject_point(p);
    out.far_point = unproject_point(vec3<f32>(p.xy, 0.001)); // unprojecting on the far plane
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let ray_origin = in.near_point;
    let ray_direction = normalize(in.far_point - in.near_point);
    let plane_normal = grid_position.normal;
    let plane_origin = grid_position.origin;

    let denominator = dot(ray_direction, plane_normal);
    let point_to_point = plane_origin - ray_origin;
    let t = dot(plane_normal, point_to_point) / denominator;
    let frag_pos_3d = ray_direction * t + ray_origin;

    let planar_offset = frag_pos_3d - plane_origin;
    let rotation_matrix = grid_position.planar_rotation_matrix;
    let plane_coords = (grid_position.planar_rotation_matrix * planar_offset).xz;


    let view_space_pos = view.inverse_view * vec4(frag_pos_3d, 1.);
    let clip_space_pos = view.projection * view_space_pos;
    let clip_depth = clip_space_pos.z / clip_space_pos.w;
    let real_depth = -view_space_pos.z;

    var out: FragmentOutput;

    out.depth = clip_depth;

    let scale = grid_settings.scale;
    let coord = plane_coords * scale; // use the scale variable to set the distance between the lines
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let lne = min(grid.x, grid.y);

    let derivative2 = fwidth(coord * 0.1);
    let grid2 = abs(fract((coord * 0.1) - 0.5) - 0.5) / derivative2;
    let mg_line = min(grid2.x, grid2.y);

    let grid3 = abs(coord) / derivative;
    let axis_line = min(grid3.x, grid3.y);

    var alpha = vec3(1.0) - min(vec3(axis_line, mg_line, lne), vec3(1.0));
    alpha.y *= (1.0 - alpha.x) * grid_settings.major_line_col.a;
    alpha.z *= (1.0 - (alpha.x + alpha.y)) * grid_settings.minor_line_col.a;

    let dist_fadeout = min(1., 1. - grid_settings.dist_fadeout_const * real_depth);
    let dot_fadeout = abs(dot(grid_position.normal, normalize(view.world_position - frag_pos_3d)));
    let alpha_fadeout = mix(dist_fadeout, 1., dot_fadeout) * min(grid_settings.dot_fadeout_const * dot_fadeout, 1.);

    let a_0 = alpha.x + alpha.y + alpha.z;
    alpha /= a_0;
    // On MacOS the line above could generate NaNs and render as black instead of transparent
    alpha = clamp(alpha, vec3(0.0), vec3(1.0));
    let axis_color = mix(grid_settings.x_axis_col, grid_settings.z_axis_col, step(grid3.x, grid3.y));
    var grid_color = vec4(
        axis_color * alpha.x + grid_settings.major_line_col.rgb * alpha.y + grid_settings.minor_line_col.rgb * alpha.z,
        max(a_0 * alpha_fadeout, 0.0),
    );
    out.color = grid_color;

    return out;
}
