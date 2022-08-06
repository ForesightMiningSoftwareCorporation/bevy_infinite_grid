mod render;

use bevy::math::{Vec3Swizzles, Vec4Swizzles};
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::render::view::{VisibilitySystems, VisibleEntities};
use bevy::{pbr::NotShadowCaster, render::view::NoFrustumCulling};

pub struct InfiniteGridPlugin;

impl Plugin for InfiniteGridPlugin {
    fn build(&self, app: &mut App) {
        render::render_app_builder(app);
        app.add_system_to_stage(CoreStage::PostUpdate, track_frustum_intersect_system)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                track_caster_visibility.after(VisibilitySystems::CheckVisibility),
            );
    }
}

#[derive(Component, Copy, Clone)]
pub struct InfiniteGrid {
    pub x_axis_color: Color,
    pub z_axis_color: Color,
    pub shadow_color: Option<Color>,
    pub minor_line_color: Color,
    pub major_line_color: Color,
    pub fadeout_distance: f32,
    pub dot_fadeout_strength: f32,
}

impl Default for InfiniteGrid {
    fn default() -> Self {
        Self {
            x_axis_color: Color::rgb(1.0, 0.2, 0.2),
            z_axis_color: Color::rgb(0.2, 0.2, 1.0),
            shadow_color: Some(Color::rgba(0.2, 0.2, 0.2, 0.7)),
            minor_line_color: Color::rgb(0.1, 0.1, 0.1),
            major_line_color: Color::rgb(0.25, 0.25, 0.25),
            fadeout_distance: 100.,
            dot_fadeout_strength: 0.25,
        }
    }
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct GridFrustumIntersect {
    pub points: [Vec3; 4],
    pub center: Vec3,
    pub up_dir: Vec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Bundle)]
pub struct InfiniteGridBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub grid: InfiniteGrid,
    pub frustum_intersect: GridFrustumIntersect,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub shadow_casters: VisibleEntities,
    pub no_frustum_culling: NoFrustumCulling,
}

impl Default for InfiniteGridBundle {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            global_transform: Default::default(),
            grid: Default::default(),
            frustum_intersect: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            shadow_casters: Default::default(),
            no_frustum_culling: NoFrustumCulling,
        }
    }
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

fn track_frustum_intersect_system(
    mut commands: Commands,
    mut grids: Query<(
        Entity,
        &GlobalTransform,
        &InfiniteGrid,
        Option<&mut GridFrustumIntersect>,
    )>,
    camera: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
) {
    let (cam_pos, cam) = camera.single();

    let view = cam_pos.compute_matrix();
    let inverse_view = view.inverse();
    let reverse_proj = cam.projection_matrix().inverse();

    for (entity, grid, grid_params, intersects) in grids.iter_mut() {
        let mut slot = None;
        let intersects = match (grid_params.shadow_color.is_none(), intersects) {
            (true, None) => {
                continue;
            }
            (true, Some(_)) => {
                commands.entity(entity).remove::<GridFrustumIntersect>();
                continue;
            }
            (false, None) => {
                slot = Some(GridFrustumIntersect::default());
                slot.as_mut().unwrap()
            }
            (false, Some(val)) => val.into_inner(),
        };
        let distant_point = calculate_distant_from(cam_pos, grid, grid_params.fadeout_distance);
        let projected = cam.projection_matrix() * inverse_view * distant_point.extend(1.);
        let coords = projected.xyz() / projected.w;

        let horizon_sign = (cam_pos.translation() - grid.translation())
            .dot(grid.up())
            .signum();

        let horizon = (-1.0..1.0)
            .contains(&coords.y)
            .then(|| coords.y)
            .unwrap_or(horizon_sign);
        // let horizon = horizon_sign;

        let seeds = [
            Vec2::new(1., horizon),
            Vec2::new(1., -horizon_sign),
            Vec2::new(-1., -horizon_sign),
            Vec2::new(-1., horizon),
        ];

        let plane_normal = grid.up();
        let plane_origin = grid.translation();

        let points = seeds.map(|sp| {
            let val = view * reverse_proj * sp.extend(1.).extend(1.);
            let near_point = val.xyz() / val.w;
            let val = view * reverse_proj * sp.extend(0.001).extend(1.);
            let far_point = val.xyz() / val.w;

            let ray_origin = near_point;
            let ray_direction = (far_point - near_point).normalize();

            let denominator = ray_direction.dot(plane_normal);
            let point_to_point = plane_origin - ray_origin;
            let t = plane_normal.dot(point_to_point) / denominator;
            let pos = ray_direction * t + ray_origin;

            pos
        });

        intersects.points = points;
        intersects.center = points.iter().sum::<Vec3>() / 4.;
        intersects.up_dir = ((points[0] + points[3]) - (points[1] + points[2])).normalize();

        intersects.height = (points[0] - points[1]).dot(intersects.up_dir);
        let w1 = points[0].distance_squared(points[3]);
        let w2 = points[1].distance_squared(points[2]);
        intersects.width = w1.max(w2).sqrt();

        if let Some(intersect) = slot {
            commands.entity(entity).insert(intersect);
        }
    }
}

fn track_caster_visibility(
    mut grids: Query<(
        &mut VisibleEntities,
        &GlobalTransform,
        &GridFrustumIntersect,
    )>,
    mut meshes: Query<
        (
            Entity,
            &Visibility,
            &mut ComputedVisibility,
            Option<(&GlobalTransform, &Aabb)>,
        ),
        (With<Handle<Mesh>>, Without<NotShadowCaster>),
    >,
) {
    for (mut visibles, _grid_transform, _grid) in grids.iter_mut() {
        visibles.entities.clear();
        for (entity, visibility, mut computed, _intersect_testable) in meshes.iter_mut() {
            if !visibility.is_visible {
                continue;
            }

            // TODO: add a check here for if the projection of the aabb onto the plane has any overlap with the grid frustum intersect
            computed.set_visible_in_view();
            visibles.entities.push(entity);
        }
    }
}
