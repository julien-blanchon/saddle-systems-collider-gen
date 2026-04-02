use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, ColliderGenConfig, CoordinateTransform};

#[derive(Resource)]
struct ExampleScene {
    mask: BinaryImage,
    transform: CoordinateTransform,
    result: saddle_systems_collider_gen::ColliderGenResult,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen basic");
    app.add_systems(Startup, setup)
        .add_systems(Update, draw_scene)
        .run();
}

fn setup(mut commands: Commands) {
    let mut mask = BinaryImage::new(24, 16);
    mask.fill_rect(0, 0, 24, 3);
    mask.fill_rect(2, 6, 6, 2);
    mask.fill_rect(10, 9, 10, 2);
    mask.fill_polygon(&[
        Vec2::new(15.0, 3.0),
        Vec2::new(22.0, 3.0),
        Vec2::new(18.0, 8.0),
    ]);

    let config = ColliderGenConfig::default();
    let result = saddle_systems_collider_gen::generate_collider_geometry(&mask, &config)
        .expect("basic scene should generate");
    let transform = CoordinateTransform::centered(mask.width(), mask.height(), config.scale * 24.0);

    commands.insert_resource(ExampleScene {
        mask,
        transform,
        result,
    });
}

fn draw_scene(mut gizmos: Gizmos, scene: Res<ExampleScene>) {
    support::draw_mask(
        &mut gizmos,
        &scene.mask,
        scene.transform,
        Color::srgba(0.35, 0.39, 0.44, 0.4),
    );
    support::draw_result(
        &mut gizmos,
        &scene.result,
        Color::srgb(0.32, 0.91, 0.91),
        Color::srgb(0.95, 0.44, 0.75),
    );
}
