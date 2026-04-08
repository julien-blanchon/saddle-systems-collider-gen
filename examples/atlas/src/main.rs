use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use image::{DynamicImage, Rgba, RgbaImage};
use saddle_systems_collider_gen::{
    AtlasSlicer, BinaryImage, ColliderGenLod, ColliderGenResult, CoordinateTransform,
};
use support::{ColliderGenExamplePane, ColliderGenPaneSettings};

#[derive(Resource)]
struct AtlasScene {
    tiles: Vec<(BinaryImage, ColliderGenResult, Vec2)>,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen atlas");
    app.add_systems(Startup, setup)
        .add_systems(Update, (refresh_scene, draw_scene).chain())
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    commands.insert_resource(build_scene(&pane));
}

fn build_scene(pane: &ColliderGenExamplePane) -> AtlasScene {
    let mut image = RgbaImage::from_pixel(48, 32, Rgba([0, 0, 0, 0]));
    for y in 0..16 {
        for x in 0..16 {
            if x > 1 && x < 14 && y > 1 && y < 14 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }
    for y in 0..16 {
        for x in 16..32 {
            if (x + y) % 2 == 0 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }
    for y in 16..32 {
        for x in 32..48 {
            let dx = x as i32 - 40;
            let dy = y as i32 - 24;
            if dx * dx + dy * dy <= 36 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }

    let atlas = BinaryImage::from_dynamic_image(&DynamicImage::ImageRgba8(image), &default())
        .expect("atlas image should convert");
    let slicer = AtlasSlicer::from_grid(atlas, UVec2::new(16, 16), 3, 2, None, None);
    let config = support::pane_config(pane, ColliderGenLod::High);

    let mut tiles = Vec::new();
    for (slot, region) in slicer.iter_regions().enumerate() {
        let tile = slicer.slice_rect(region.rect).expect("tile should slice");
        if tile.filled_count() == 0 {
            continue;
        }
        let result = saddle_systems_collider_gen::generate_collider_geometry(&tile, &config)
            .expect("tile collider generation should succeed");
        let column = (slot % 3) as f32;
        let row = (slot / 3) as f32;
        tiles.push((
            tile,
            result,
            Vec2::new(column * 420.0 - 420.0, 180.0 - row * 320.0),
        ));
    }

    AtlasScene { tiles }
}

fn refresh_scene(
    pane: Res<ColliderGenExamplePane>,
    mut scene: ResMut<AtlasScene>,
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
    scene: Res<AtlasScene>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    let mut contour_count = 0;
    let mut hull_count = 0;
    let mut piece_count = 0;
    let mut warning_count = 0;

    for (tile, result, offset) in &scene.tiles {
        let transform = CoordinateTransform::centered(
            tile.width(),
            tile.height(),
            Vec2::splat(pane.render_scale),
        );
        if pane.show_mask {
            support::draw_mask_at(
                &mut gizmos,
                tile,
                transform,
                *offset,
                Color::srgba(0.30, 0.34, 0.40, 0.4),
            );
        }
        for contour in &result.contours {
            let shifted = contour
                .points
                .iter()
                .map(|point| *point + *offset)
                .collect::<Vec<_>>();
            support::draw_loop(&mut gizmos, &shifted, Color::srgb(0.28, 0.92, 0.85));
        }
        if pane.show_hulls {
            for hull in &result.convex_hulls {
                let shifted = hull
                    .points
                    .iter()
                    .map(|point| *point + *offset)
                    .collect::<Vec<_>>();
                support::draw_loop(&mut gizmos, &shifted, Color::srgb(0.96, 0.66, 0.24));
            }
        }
        if pane.show_pieces {
            for (index, piece) in result.convex_pieces.iter().enumerate() {
                let shifted = piece
                    .points
                    .iter()
                    .map(|point| *point + piece.offset + *offset)
                    .collect::<Vec<_>>();
                support::draw_loop(&mut gizmos, &shifted, support::palette(index));
            }
        }

        contour_count += result.contours.len() as u32;
        hull_count += result.convex_hulls.len() as u32;
        piece_count += result.convex_pieces.len() as u32;
        warning_count += result.warnings.len() as u32;
    }

    pane.contour_count = contour_count;
    pane.hull_count = hull_count;
    pane.piece_count = piece_count;
    pane.warning_count = warning_count;
}
