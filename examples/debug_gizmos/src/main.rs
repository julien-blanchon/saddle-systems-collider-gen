use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_systems_collider_gen::{
    BinaryImage, ColliderGenLod, ColliderGenResult, ContourMode, CoordinateTransform,
};
use support::{ColliderGenExamplePane, ColliderGenPaneSettings};

#[derive(Resource)]
struct ComparisonScene {
    mask: BinaryImage,
    left: ColliderGenResult,
    right: ColliderGenResult,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen debug gizmos");
    app.add_plugins(BrpExtrasPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (refresh_scene, draw_scene).chain())
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    commands.insert_resource(build_scene(&pane));
}

fn build_scene(pane: &ColliderGenExamplePane) -> ComparisonScene {
    let mut mask = BinaryImage::new(32, 20);
    mask.fill_rect(0, 0, 32, 4);
    mask.fill_rect(3, 7, 8, 2);
    mask.fill_rect(18, 8, 10, 2);
    mask.fill_polygon(&[
        Vec2::new(12.0, 4.0),
        Vec2::new(20.0, 4.0),
        Vec2::new(16.0, 12.0),
    ]);
    mask.carve_circle(IVec2::new(21, 5), 2);

    let left = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &support::pane_config(pane, ColliderGenLod::High),
    )
    .expect("pixel exact generation should succeed");
    let mut right_config = support::pane_config(pane, ColliderGenLod::Low);
    right_config.contour_mode = ContourMode::MarchingSquares;
    let right = saddle_systems_collider_gen::generate_collider_geometry(&mask, &right_config)
        .expect("marching squares generation should succeed");

    ComparisonScene { mask, left, right }
}

fn refresh_scene(
    pane: Res<ColliderGenExamplePane>,
    mut scene: ResMut<ComparisonScene>,
    mut last_settings: Local<Option<ColliderGenPaneSettings>>,
) {
    let settings = support::pane_settings(&pane);
    if last_settings.as_ref() == Some(&settings) {
        return;
    }

    *scene = build_scene(&pane);
    *last_settings = Some(settings);
}

fn draw_scene(
    mut gizmos: Gizmos,
    scene: Res<ComparisonScene>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    let transform = CoordinateTransform::centered(
        scene.mask.width(),
        scene.mask.height(),
        Vec2::splat(pane.render_scale),
    );
    let left_offset = Vec2::new(-360.0, 0.0);
    let right_offset = Vec2::new(360.0, 0.0);

    if pane.show_mask {
        support::draw_mask_at(
            &mut gizmos,
            &scene.mask,
            transform,
            left_offset,
            Color::srgba(0.30, 0.33, 0.36, 0.35),
        );
        support::draw_mask_at(
            &mut gizmos,
            &scene.mask,
            transform,
            right_offset,
            Color::srgba(0.30, 0.33, 0.36, 0.35),
        );
    }
    for contour in &scene.left.contours {
        let shifted = contour
            .points
            .iter()
            .map(|point| *point + left_offset)
            .collect::<Vec<_>>();
        support::draw_loop(&mut gizmos, &shifted, Color::srgb(0.18, 0.96, 0.86));
    }
    for contour in &scene.right.contours {
        let shifted = contour
            .points
            .iter()
            .map(|point| *point + right_offset)
            .collect::<Vec<_>>();
        support::draw_loop(&mut gizmos, &shifted, Color::srgb(0.98, 0.60, 0.24));
    }
    if pane.show_hulls {
        for hull in &scene.left.convex_hulls {
            let shifted = hull
                .points
                .iter()
                .map(|point| *point + left_offset)
                .collect::<Vec<_>>();
            support::draw_loop(&mut gizmos, &shifted, Color::srgba(0.95, 0.78, 0.34, 0.85));
        }
        for hull in &scene.right.convex_hulls {
            let shifted = hull
                .points
                .iter()
                .map(|point| *point + right_offset)
                .collect::<Vec<_>>();
            support::draw_loop(&mut gizmos, &shifted, Color::srgba(0.95, 0.78, 0.34, 0.85));
        }
    }
    if pane.show_pieces {
        for (index, piece) in scene.left.convex_pieces.iter().enumerate() {
            let shifted = piece
                .points
                .iter()
                .map(|point| *point + piece.offset + left_offset)
                .collect::<Vec<_>>();
            support::draw_loop(&mut gizmos, &shifted, support::palette(index));
        }
        for (index, piece) in scene.right.convex_pieces.iter().enumerate() {
            let shifted = piece
                .points
                .iter()
                .map(|point| *point + piece.offset + right_offset)
                .collect::<Vec<_>>();
            support::draw_loop(&mut gizmos, &shifted, support::palette(index));
        }
    }

    pane.contour_count = (scene.left.contours.len() + scene.right.contours.len()) as u32;
    pane.hull_count = (scene.left.convex_hulls.len() + scene.right.convex_hulls.len()) as u32;
    pane.piece_count = (scene.left.convex_pieces.len() + scene.right.convex_pieces.len()) as u32;
    pane.warning_count = (scene.left.warnings.len() + scene.right.warnings.len()) as u32;
}
