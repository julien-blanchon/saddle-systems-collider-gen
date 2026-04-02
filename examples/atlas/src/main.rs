use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{AtlasSlicer, BinaryImage, ColliderGenConfig, CoordinateTransform};
use image::{DynamicImage, Rgba, RgbaImage};

#[derive(Resource)]
struct AtlasScene {
    tiles: Vec<(BinaryImage, saddle_systems_collider_gen::ColliderGenResult, Vec2)>,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen atlas");
    app.add_systems(Startup, setup)
        .add_systems(Update, draw_scene)
        .run();
}

fn setup(mut commands: Commands) {
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

    let mut tiles = Vec::new();
    for (slot, region) in slicer.iter_regions().enumerate() {
        let tile = slicer.slice_rect(region.rect).expect("tile should slice");
        if tile.filled_count() == 0 {
            continue;
        }
        let result = saddle_systems_collider_gen::generate_collider_geometry(&tile, &ColliderGenConfig::default())
            .expect("tile collider generation should succeed");
        let column = (slot % 3) as f32;
        let row = (slot / 3) as f32;
        tiles.push((
            tile,
            result,
            Vec2::new(column * 420.0 - 420.0, 180.0 - row * 320.0),
        ));
    }

    commands.insert_resource(AtlasScene { tiles });
}

fn draw_scene(mut gizmos: Gizmos, scene: Res<AtlasScene>) {
    for (tile, result, offset) in &scene.tiles {
        let transform =
            CoordinateTransform::centered(tile.width(), tile.height(), Vec2::splat(16.0));
        support::draw_mask_at(
            &mut gizmos,
            tile,
            transform,
            *offset,
            Color::srgba(0.30, 0.34, 0.40, 0.4),
        );
        for contour in &result.contours {
            let shifted = contour
                .points
                .iter()
                .map(|point| *point + *offset)
                .collect::<Vec<_>>();
            support::draw_loop(&mut gizmos, &shifted, Color::srgb(0.28, 0.92, 0.85));
        }
    }
}
