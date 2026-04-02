use bevy::math::IVec2;
use bevy::prelude::*;

use super::*;
use crate::config::{ColorKey, ImageMaskConfig, MaskChannelMode, RawImageFormat};

#[test]
fn out_of_bounds_queries_are_safe() {
    let mut image = BinaryImage::new(2, 2);
    image.set(1, 1, true);

    assert!(!image.get(99, 99));
    assert!(!image.get_i32(-1, 0));
    assert!(image.get(1, 1));
}

#[test]
fn fill_and_carve_operations_change_pixel_counts() {
    let mut image = BinaryImage::new(6, 6);
    image.fill_rect(1, 1, 4, 4);
    assert_eq!(image.filled_count(), 16);

    image.carve_rect(2, 2, 2, 2);
    assert_eq!(image.filled_count(), 12);

    image.fill_circle(IVec2::new(3, 3), 1);
    assert!(image.filled_count() >= 13);
}

#[test]
fn crop_and_polygon_fill_work_together() {
    let mut image = BinaryImage::new(8, 8);
    image.fill_polygon(&[
        Vec2::new(1.0, 1.0),
        Vec2::new(6.0, 1.0),
        Vec2::new(3.5, 6.0),
    ]);

    let cropped = image.crop(URect::from_corners(UVec2::new(1, 1), UVec2::new(7, 7)));
    assert_eq!(cropped.width(), 6);
    assert_eq!(cropped.height(), 6);
    assert!(cropped.filled_count() > 0);
}

#[test]
fn morphology_closes_single_pixel_gap() {
    let mut image = BinaryImage::new(5, 3);
    image.fill_rect(0, 1, 2, 1);
    image.fill_rect(3, 1, 2, 1);

    let closed = image.close(1);
    assert!(closed.get(2, 1));
}

#[test]
fn grow_and_shrink_aliases_match_morphology_operations() {
    let mut image = BinaryImage::new(7, 7);
    image.fill_circle(IVec2::new(3, 3), 1);

    assert_eq!(image.grow(1), image.dilate(1));
    assert_eq!(image.shrink(1), image.erode(1));
}

#[test]
fn stamp_and_carve_mask_support_composite_authoring() {
    let mut tile = BinaryImage::new(3, 3);
    tile.fill_rect(0, 1, 3, 1);
    tile.fill_rect(1, 0, 1, 3);

    let mut canvas = BinaryImage::new(8, 5);
    canvas.stamp_mask(&tile, UVec2::new(1, 1));
    canvas.stamp_mask(&tile, UVec2::new(4, 1));

    assert!(canvas.get(2, 1));
    assert!(canvas.get(5, 1));
    assert!(canvas.get(3, 2));

    canvas.carve_mask(&tile, UVec2::new(4, 1));

    assert!(canvas.get(2, 1));
    assert!(!canvas.get(5, 1));
    assert!(!canvas.get(5, 2));
}

#[test]
fn raw_byte_extraction_supports_thresholds_and_color_keys() {
    let config = ImageMaskConfig {
        channel_mode: MaskChannelMode::Alpha,
        alpha_threshold: 128,
        color_key: Some(ColorKey {
            rgba: [255, 0, 255, 255],
            tolerance: 0,
        }),
        ..default()
    };
    let bytes = [
        255, 255, 255, 255, //
        255, 0, 255, 255, //
        255, 255, 255, 0,
    ];

    let image = BinaryImage::from_raw_bytes(3, 1, &bytes, RawImageFormat::Rgba8, &config)
        .expect("raw bytes should decode");

    assert!(image.get(0, 0));
    assert!(!image.get(1, 0));
    assert!(!image.get(2, 0));
}

#[test]
fn threshold_edges_respect_cutoffs_and_inversion() {
    let bytes = [
        255, 255, 255, 127, //
        255, 255, 255, 128,
    ];

    let alpha_mask = BinaryImage::from_raw_bytes(
        2,
        1,
        &bytes,
        RawImageFormat::Rgba8,
        &ImageMaskConfig {
            channel_mode: MaskChannelMode::Alpha,
            alpha_threshold: 128,
            ..default()
        },
    )
    .expect("alpha threshold mask should decode");
    let inverted = BinaryImage::from_raw_bytes(
        2,
        1,
        &bytes,
        RawImageFormat::Rgba8,
        &ImageMaskConfig {
            channel_mode: MaskChannelMode::Alpha,
            alpha_threshold: 128,
            invert_mask: true,
            ..default()
        },
    )
    .expect("inverted alpha mask should decode");

    assert!(!alpha_mask.get(0, 0));
    assert!(alpha_mask.get(1, 0));
    assert!(inverted.get(0, 0));
    assert!(!inverted.get(1, 0));
}
