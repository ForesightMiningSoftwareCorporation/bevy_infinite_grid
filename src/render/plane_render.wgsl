struct InfiniteGrid {
    planar_rotation_matrix: mat3x3<f32>;
    origin: vec3<f32>;
    normal: vec3<f32>;
    scale: f32;
    x_axis_col: vec3<f32>;
    z_axis_col: vec3<f32>;
    shadow_col: vec4<f32>;
    minor_line_col: vec4<f32>;
    major_line_col: vec4<f32>;

    shadow_collapse_matrix: mat3x3<f32>;
    shadow_center_pos: vec3<f32>;
    shadow_texture_width: f32;
    shadow_texture_height: f32;
};

struct View {
    projection: mat4x4<f32>;
    inverse_projection: mat4x4<f32>;
    view: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    world_position: vec3<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var<uniform> infinite_grid: InfiniteGrid;

[[group(2), binding(0)]]
var grid_shadow_texture: texture_2d<f32>; 

[[group(2), binding(1)]]
var grid_shadow_sampler: sampler;

struct Vertex {
    [[builtin(vertex_index)]] index: u32;
};

fn unproject_point(point: vec3<f32>) -> vec3<f32> {
    let unprojected = view.view * view.inverse_projection * vec4<f32>(point, 1.0);
    return unprojected.xyz / unprojected.w;
}

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] near_point: vec3<f32>;
    [[location(1)]] far_point: vec3<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    // 0 1 2 1 2 3
    var grid_plane = array<vec3<f32>, 4>(
        vec3<f32>(-1., -1., 1.),
        vec3<f32>(-1., 1., 1.),
        vec3<f32>(1., -1., 1.),
        vec3<f32>(1., 1., 1.),
    );
    let p = grid_plane[vertex.index].xyz;

    var out: VertexOutput;

    out.clip_position = vec4<f32>(p, 1.);
    out.near_point = unproject_point(p);
    out.far_point = unproject_point(vec3<f32>(p.xy, 0.001)); // unprojecting on the far plane
    return out;
}


fn color_grid(real_coords: vec3<f32>, plane_coords: vec2<f32>, scale: f32, shadow: f32, real_depth: f32) -> vec4<f32> {
    let coord = plane_coords * scale; // use the scale variable to set the distance between the lines
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line = min(grid.x, grid.y);

    let minimumz = min(derivative.y, 1.) / scale;
    let minimumx = min(derivative.x, 1.) / scale;

    let mg_coord = coord * 0.1;
    let mg_derivative = fwidth(mg_coord);
    let mg_grid = abs(fract(mg_coord - 0.5) - 0.5) / mg_derivative;
    let mg_line = min(mg_grid.x, mg_grid.y);

    let base_color = mix(infinite_grid.major_line_col, infinite_grid.minor_line_col, step(1., mg_line));

    let base_alpha = base_color.a * (1.0 - min(line, 1.0));
    let dist_fadeout = min(1., 1. - infinite_grid.scale * real_depth / 100.);
    let dot_fadeout = abs(dot(infinite_grid.normal, normalize(view.world_position - real_coords)));
    let alpha_fadeout = mix(dist_fadeout, 1., dot_fadeout);
    let true_alpha = base_alpha * alpha_fadeout * step(0.01, abs(dot(infinite_grid.normal, view.world_position - real_coords)));

    var color = vec4<f32>(base_color.rgb, true_alpha);

    color = mix(color, infinite_grid.shadow_col, 1. - shadow);

    let z_axis_cond = plane_coords.x > -1.0 * minimumx && plane_coords.x < 1.0 * minimumx;
    let x_axis_cond = plane_coords.y > -1.0 * minimumz && plane_coords.y < 1.0 * minimumz;

    color = mix(color, vec4<f32>(infinite_grid.z_axis_col, color.a), f32(z_axis_cond));
    color = mix(color, vec4<f32>(infinite_grid.x_axis_col, color.a), f32(x_axis_cond));

    return color;
}

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
    [[builtin(frag_depth)]] depth: f32;
};

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> FragmentOutput {
    let ray_origin = in.near_point;
    let ray_direction = normalize(in.far_point - in.near_point);
    let plane_normal = infinite_grid.normal;
    let plane_origin = infinite_grid.origin;

    let denominator = dot(ray_direction, plane_normal);
    let point_to_point = plane_origin - ray_origin;
    let t = dot(plane_normal, point_to_point) / denominator;
    let frag_pos_3d = ray_direction * t + ray_origin;

    let planar_offset = frag_pos_3d - plane_origin;
    let rotation_matrix = infinite_grid.planar_rotation_matrix;
    let plane_coords = (infinite_grid.planar_rotation_matrix * planar_offset).xz;


    let view_space_pos = view.inverse_view * vec4<f32>(frag_pos_3d, 1.);
    let clip_space_pos = view.projection * view_space_pos;
    let clip_depth = clip_space_pos.z / clip_space_pos.w;
    let real_depth = -view_space_pos.z;

    var out: FragmentOutput;

    out.depth = clip_depth;

    let grid_pos_relative_to_shadow_center = (infinite_grid.shadow_collapse_matrix * (frag_pos_3d - infinite_grid.shadow_center_pos)).xz;
    let shadow_size = vec2<f32>(infinite_grid.shadow_texture_width, infinite_grid.shadow_texture_height);
    let offset_location = grid_pos_relative_to_shadow_center / shadow_size;
    let uv = offset_location + vec2<f32>(0.5);

    let shadow = textureSample(grid_shadow_texture, grid_shadow_sampler, uv).r;
    out.color = color_grid(frag_pos_3d, plane_coords, infinite_grid.scale, 1. - shadow, real_depth);

    return out;
}
