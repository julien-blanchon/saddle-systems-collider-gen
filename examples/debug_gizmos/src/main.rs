use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_systems_collider_gen::{BinaryImage, ColliderGenConfig, ContourMode, CoordinateTransform};

#[derive(Resource)]
struct ComparisonScene {
    mask: BinaryImage,
    left: saddle_systems_collider_gen::ColliderGenResult,
    right: saddle_systems_collider_gen::ColliderGenResult,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "collider_gen debug gizmos".to_string(),
            resolution: (1600, 920).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(BrpExtrasPlugin::default())
    .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
    .add_systems(Startup, setup)
    .add_systems(Update, draw_scene);
    support::install_auto_exit(&mut app);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Name::new("ExampleCamera"), Camera2d));

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
        &ColliderGenConfig::default().with_lod(saddle_systems_collider_gen::ColliderGenLod::High),
    )
    .expect("pixel exact generation should succeed");
    let right = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig {
            contour_mode: ContourMode::MarchingSquares,
            ..ColliderGenConfig::default().with_lod(saddle_systems_collider_gen::ColliderGenLod::Low)
        },
    )
    .expect("marching squares generation should succeed");

    commands.insert_resource(ComparisonScene { mask, left, right });
}

fn draw_scene(mut gizmos: Gizmos, scene: Res<ComparisonScene>) {
    let transform =
        CoordinateTransform::centered(scene.mask.width(), scene.mask.height(), Vec2::splat(18.0));
    let left_offset = Vec2::new(-360.0, 0.0);
    let right_offset = Vec2::new(360.0, 0.0);

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
}
