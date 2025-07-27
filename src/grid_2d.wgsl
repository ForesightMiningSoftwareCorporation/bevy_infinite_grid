struct InfiniteGrid2DSettings {
    scale: f32,
    x_axis_col: vec3<f32>,
    y_axis_col: vec3<f32>,
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
@group(1) @binding(0) var<uniform> grid_settings: InfiniteGrid2DSettings;

struct Vertex {
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    // Generate a full-screen quad
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );
    
    let position = positions[vertex.index];
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 0.0, 1.0);
    
    // Convert clip space to world space properly
    // First, unproject from clip space to view space
    let view_pos = view.inverse_projection * vec4<f32>(position, 0.0, 1.0);
    let view_pos_normalized = view_pos.xy / view_pos.w;
    
    // Then transform from view space to world space
    let world_pos_4d = view.inverse_view * vec4<f32>(view_pos_normalized, 0.0, 1.0);
    out.world_position = world_pos_4d.xy;
    
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    
    let scale = grid_settings.scale;
    let coord = in.world_position * scale;
    
    // Calculate grid lines using derivatives for anti-aliasing
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line = min(grid.x, grid.y);
    
    // Major grid lines (every 10 units)
    let derivative2 = fwidth(coord * 0.1);
    let grid2 = abs(fract((coord * 0.1) - 0.5) - 0.5) / derivative2;
    let major_line = min(grid2.x, grid2.y);
    
    // Axis lines (X and Y axes)
    let grid_axis = abs(coord) / derivative;
    let axis_line = min(grid_axis.x, grid_axis.y);
    
    // Calculate alpha values for different line types
    var alpha = vec3<f32>(1.0) - min(vec3<f32>(axis_line, major_line, line), vec3<f32>(1.0));
    alpha.y *= (1.0 - alpha.x) * grid_settings.major_line_col.a;
    alpha.z *= (1.0 - (alpha.x + alpha.y)) * grid_settings.minor_line_col.a;
    
    let total_alpha = alpha.x + alpha.y + alpha.z;
    alpha /= max(total_alpha, 0.001); // Prevent division by zero
    alpha = clamp(alpha, vec3<f32>(0.0), vec3<f32>(1.0));
    
    // Choose axis color based on which axis is more prominent
    let axis_color = mix(grid_settings.x_axis_col, grid_settings.y_axis_col, step(grid_axis.x, grid_axis.y));
    
    let grid_color = vec4<f32>(
        axis_color * alpha.x + grid_settings.major_line_col.rgb * alpha.y + grid_settings.minor_line_col.rgb * alpha.z,
        max(total_alpha, 0.0)
    );
    
    out.color = grid_color;
    return out;
}