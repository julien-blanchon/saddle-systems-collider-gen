use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    BinaryImage, ColliderGenLod, ColliderGenResult, CoordinateTransform,
};
use support::{ColliderGenExamplePane, ColliderGenPaneSettings};

#[derive(Resource)]
struct TilemapMergeScene {
    mask: BinaryImage,
    transform: CoordinateTransform,
    result: ColliderGenResult,
    tiles: Vec<(BinaryImage, Vec2, Color)>,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen tilemap merge");
    app.add_systems(Startup, setup)
        .add_systems(Update, (refresh_scene, draw_scene).chain())
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    commands.insert_resource(build_scene(&pane));
}

fn build_scene(pane: &ColliderGenExamplePane) -> TilemapMergeScene {
    let mut canvas = BinaryImage::new(42, 24);
    let ground = ground_tile();
    let wall = wall_tile();
    let arch = arch_tile();
    let ramp = ramp_tile();

    let tiles = vec![
        (
            ground.clone(),
            Vec2::new(-360.0, -120.0),
            Color::srgba(0.45, 0.52, 0.62, 0.28),
        ),
        (
            ground.clone(),
            Vec2::new(-180.0, -120.0),
            Color::srgba(0.45, 0.52, 0.62, 0.28),
        ),
        (
            ground.clone(),
            Vec2::new(0.0, -120.0),
            Color::srgba(0.45, 0.52, 0.62, 0.28),
        ),
        (
            ground.clone(),
            Vec2::new(180.0, -120.0),
            Color::srgba(0.45, 0.52, 0.62, 0.28),
        ),
        (
            wall.clone(),
            Vec2::new(-360.0, 10.0),
            Color::srgba(0.78, 0.53, 0.34, 0.28),
        ),
        (
            arch.clone(),
            Vec2::new(-90.0, 10.0),
            Color::srgba(0.84, 0.67, 0.38, 0.28),
        ),
        (
            wall.clone(),
            Vec2::new(180.0, 10.0),
            Color::srgba(0.78, 0.53, 0.34, 0.28),
        ),
        (
            ramp.clone(),
            Vec2::new(360.0, -20.0),
            Color::srgba(0.51, 0.78, 0.62, 0.28),
        ),
    ];

    canvas.stamp_mask(&ground, UVec2::new(0, 16));
    canvas.stamp_mask(&ground, UVec2::new(10, 16));
    canvas.stamp_mask(&ground, UVec2::new(20, 16));
    canvas.stamp_mask(&ground, UVec2::new(30, 16));
    canvas.stamp_mask(&wall, UVec2::new(0, 9));
    canvas.stamp_mask(&arch, UVec2::new(12, 9));
    canvas.stamp_mask(&wall, UVec2::new(24, 9));
    canvas.stamp_mask(&ramp, UVec2::new(34, 12));
    canvas.carve_rect(20, 18, 2, 2);

    let config = support::pane_config(pane, ColliderGenLod::Medium);
    let result = saddle_systems_collider_gen::generate_collider_geometry(&canvas, &config)
        .expect("tilemap merge should generate");
    let transform = CoordinateTransform::centered(canvas.width(), canvas.height(), config.scale);

    TilemapMergeScene {
        mask: canvas,
        transform,
        result,
        tiles,
    }
}

fn refresh_scene(
    pane: Res<ColliderGenExamplePane>,
    mut scene: ResMut<TilemapMergeScene>,
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
    scene: Res<TilemapMergeScene>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    for (tile, offset, color) in &scene.tiles {
        let transform =
            CoordinateTransform::centered(tile.width(), tile.height(), Vec2::splat(pane.render_scale));
        support::draw_mask_at(&mut gizmos, tile, transform, *offset, *color);
    }

    if pane.show_mask {
        support::draw_mask(
            &mut gizmos,
            &scene.mask,
            scene.transform,
            Color::srgba(0.25, 0.30, 0.36, 0.22),
        );
    }
    for contour in &scene.result.contours {
        support::draw_contour(&mut gizmos, contour, Color::srgb(0.24, 0.92, 0.80));
    }
    if pane.show_hulls {
        for hull in &scene.result.convex_hulls {
            support::draw_contour(&mut gizmos, hull, Color::srgb(0.97, 0.67, 0.26));
        }
    }
    if pane.show_pieces {
        for (index, piece) in scene.result.convex_pieces.iter().enumerate() {
            support::draw_piece(&mut gizmos, piece, support::palette(index));
        }
    }

    support::update_result_stats(&mut pane, &scene.result);
}

fn ground_tile() -> BinaryImage {
    let mut tile = BinaryImage::new(12, 8);
    tile.fill_rect(0, 4, 12, 4);
    tile.fill_polygon(&[
        Vec2::new(0.0, 4.0),
        Vec2::new(3.0, 2.0),
        Vec2::new(9.0, 2.0),
        Vec2::new(12.0, 4.0),
        Vec2::new(12.0, 8.0),
        Vec2::new(0.0, 8.0),
    ]);
    tile
}

fn wall_tile() -> BinaryImage {
    let mut tile = BinaryImage::new(10, 12);
    tile.fill_rect(0, 2, 10, 10);
    tile.fill_polygon(&[
        Vec2::new(0.0, 2.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(8.0, 0.0),
        Vec2::new(10.0, 2.0),
    ]);
    tile
}

fn arch_tile() -> BinaryImage {
    let mut tile = BinaryImage::new(12, 12);
    tile.fill_rect(0, 2, 12, 10);
    tile.fill_polygon(&[
        Vec2::new(0.0, 2.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(12.0, 2.0),
    ]);
    tile.carve_rect(4, 4, 4, 8);
    tile
}

fn ramp_tile() -> BinaryImage {
    let mut tile = BinaryImage::new(8, 8);
    tile.fill_polygon(&[
        Vec2::new(0.0, 8.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(8.0, 0.0),
    ]);
    tile
}
