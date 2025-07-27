mod render;
mod render_2d;

use bevy::{
    prelude::*,
    render::{
        sync_world::SyncToRenderWorld,
        view::{self, NoFrustumCulling, RenderVisibleEntities, VisibilityClass},
    },
};

pub struct InfiniteGridPlugin;
pub struct InfiniteGrid2DPlugin;

impl Plugin for InfiniteGridPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        render::render_app_builder(app);
    }
}

impl Plugin for InfiniteGrid2DPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        render_2d::render_app_builder_2d(app);
    }
}

#[derive(Component, Default)]
pub struct InfiniteGrid;

#[derive(Component, Default)]
pub struct InfiniteGrid2D;

#[derive(Component, Copy, Clone)]
#[require(VisibilityClass)]
#[component(on_add = view::add_visibility_class::<InfiniteGridSettings>)]
pub struct InfiniteGridSettings {
    pub x_axis_color: Color,
    pub z_axis_color: Color,
    pub minor_line_color: Color,
    pub major_line_color: Color,
    pub fadeout_distance: f32,
    pub dot_fadeout_strength: f32,
    pub scale: f32,
}

#[derive(Component, Copy, Clone)]
#[require(VisibilityClass)]
#[component(on_add = view::add_visibility_class::<InfiniteGrid2DSettings>)]
pub struct InfiniteGrid2DSettings {
    pub x_axis_color: Color,
    pub y_axis_color: Color,
    pub minor_line_color: Color,
    pub major_line_color: Color,
    pub scale: f32,
}

impl Default for InfiniteGridSettings {
    fn default() -> Self {
        Self {
            x_axis_color: Color::srgb(1.0, 0.2, 0.2),
            z_axis_color: Color::srgb(0.2, 0.2, 1.0),
            minor_line_color: Color::srgb(0.1, 0.1, 0.1),
            major_line_color: Color::srgb(0.25, 0.25, 0.25),
            fadeout_distance: 100.,
            dot_fadeout_strength: 0.25,
            scale: 1.,
        }
    }
}

impl Default for InfiniteGrid2DSettings {
    fn default() -> Self {
        Self {
            x_axis_color: Color::srgb(1.0, 0.2, 0.2),
            y_axis_color: Color::srgb(0.2, 1.0, 0.2),
            minor_line_color: Color::srgb(0.1, 0.1, 0.1),
            major_line_color: Color::srgb(0.25, 0.25, 0.25),
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
    pub shadow_casters: RenderVisibleEntities,
    pub no_frustum_culling: NoFrustumCulling,
    pub sync_to_render_world: SyncToRenderWorld,
}

#[derive(Bundle, Default)]
pub struct InfiniteGrid2DBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub settings: InfiniteGrid2DSettings,
    pub grid: InfiniteGrid2D,
    pub visibility: Visibility,
    pub view_visibility: ViewVisibility,
    pub inherited_visibility: InheritedVisibility,
    pub shadow_casters: RenderVisibleEntities,
    pub no_frustum_culling: NoFrustumCulling,
    pub sync_to_render_world: SyncToRenderWorld,
}
