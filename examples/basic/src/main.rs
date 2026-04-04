use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, ColliderGenLod, ColliderGenResult, CoordinateTransform};
use support::{ColliderGenExamplePane, ColliderGenPaneSettings};

#[derive(Resource)]
struct BasicScene {
    mask: BinaryImage,
    transform: CoordinateTransform,
    result: ColliderGenResult,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen basic");
    app.add_systems(Startup, setup)
        .add_systems(Update, (refresh_scene, draw_scene).chain())
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    commands.insert_resource(build_scene(&pane));
}

fn build_scene(pane: &ColliderGenExamplePane) -> BasicScene {
    let mut mask = BinaryImage::new(24, 16);
    mask.fill_rect(0, 0, 24, 3);
    mask.fill_rect(2, 6, 6, 2);
    mask.fill_rect(10, 9, 10, 2);
    mask.fill_polygon(&[
        Vec2::new(15.0, 3.0),
        Vec2::new(22.0, 3.0),
        Vec2::new(18.0, 8.0),
    ]);

    let config = support::pane_config(pane, ColliderGenLod::High);
    let result = saddle_systems_collider_gen::generate_collider_geometry(&mask, &config)
        .expect("basic scene should generate");
    let transform = CoordinateTransform::centered(mask.width(), mask.height(), config.scale);

    BasicScene {
        mask,
        transform,
        result,
    }
}

fn refresh_scene(
    pane: Res<ColliderGenExamplePane>,
    mut scene: ResMut<BasicScene>,
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
    scene: Res<BasicScene>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    if pane.show_mask {
        support::draw_mask(
            &mut gizmos,
            &scene.mask,
            scene.transform,
            Color::srgba(0.35, 0.39, 0.44, 0.4),
        );
    }

    for contour in &scene.result.contours {
        support::draw_contour(&mut gizmos, contour, Color::srgb(0.32, 0.91, 0.91));
    }
    if pane.show_hulls {
        for hull in &scene.result.convex_hulls {
            support::draw_contour(&mut gizmos, hull, Color::srgb(0.95, 0.44, 0.75));
        }
    }
    if pane.show_pieces {
        for (index, piece) in scene.result.convex_pieces.iter().enumerate() {
            support::draw_piece(&mut gizmos, piece, support::palette(index));
        }
    }

    support::update_result_stats(&mut pane, &scene.result);
}
