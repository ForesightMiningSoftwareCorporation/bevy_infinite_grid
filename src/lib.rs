mod render;

use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    render::view::{NoFrustumCulling, VisibleEntities},
};

pub struct InfiniteGridPlugin;

impl Plugin for InfiniteGridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalInfiniteGridSettings>();
    }

    fn finish(&self, app: &mut App) {
        render::render_app_builder(app);
    }
}

#[deprecated]
#[derive(Resource, Clone, Default)]
pub struct RenderSettings {
    pub max_texture_size: u32,
}

#[deprecated]
#[derive(Resource, Default)]
pub struct GlobalInfiniteGridSettings {
    pub render_settings: RenderSettings,
}

#[derive(Component, Default)]
pub struct InfiniteGrid;

#[derive(Component, Copy, Clone)]
pub struct InfiniteGridSettings {
    pub x_axis_color: Color,
    pub z_axis_color: Color,
    pub minor_line_color: Color,
    pub major_line_color: Color,
    pub fadeout_distance: f32,
    pub dot_fadeout_strength: f32,
    pub scale: f32,
}

impl Default for InfiniteGridSettings {
    fn default() -> Self {
        Self {
            x_axis_color: Color::rgb(1.0, 0.2, 0.2),
            z_axis_color: Color::rgb(0.2, 0.2, 1.0),
            minor_line_color: Color::rgb(0.1, 0.1, 0.1),
            major_line_color: Color::rgb(0.25, 0.25, 0.25),
            fadeout_distance: 100.,
            dot_fadeout_strength: 0.25,
            scale: 1.,
        }
    }
}

#[derive(Bundle, Default)]
pub struct InfiniteGridBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub settings: InfiniteGridSettings,
    pub grid: InfiniteGrid,
    pub visibility: Visibility,
    pub view_visibility: ViewVisibility,
    pub inherited_visibility: InheritedVisibility,
    pub shadow_casters: VisibleEntities,
    pub no_frustum_culling: NoFrustumCulling,
}

pub fn calculate_distant_from(
    cam: &GlobalTransform,
    grid: &GlobalTransform,
    view_distance: f32,
) -> Vec3 {
    let cam_pos = cam.translation();
    let cam_dir = cam.back();

    let (_, grid_rot, _) = grid.to_scale_rotation_translation();

    let inverse_rot = grid_rot.inverse();

    let gs_cam_pos = (inverse_rot * (cam_pos - grid.translation())).xz();
    let gs_cam_dir = (inverse_rot * cam_dir).xz().normalize();

    let h = (cam_pos - grid.translation()).dot(grid.up()).abs();
    let s = 1. / view_distance;

    let f = |d: f32| (1. - d * s) * (h * h + d * d).sqrt() + h * d * s;
    let f_prime =
        |d: f32| -s * (h * h + d * d).sqrt() + ((1. - d * s) * d / (h * h + d * d).sqrt()) + h * s;

    // use a non-zero first guess for newton iteration as f_prime(0) == 0
    let x_zero = (1. + h * s) / s;

    let mut x = x_zero;
    for _ in 0..2 {
        x = x - f(x) / f_prime(x);
    }

    let dist = x;

    let pos_in_grid_space = gs_cam_pos - gs_cam_dir * dist;
    let pos_in_3d_gs = grid_rot * pos_in_grid_space.extend(0.).xzy();

    grid.translation() + pos_in_3d_gs
}
