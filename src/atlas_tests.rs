use bevy::math::{URect, UVec2};

use super::*;
use crate::{BinaryImage, ColliderGenConfig};

#[test]
fn grid_indexing_matches_bevy_row_major_order() {
    let image = BinaryImage::new(8, 4);
    let slicer = AtlasSlicer::from_grid(image, UVec2::new(2, 2), 4, 2, None, None);

    let region0 = slicer.region_for_index(0).expect("region 0 should exist");
    let region5 = slicer.region_for_index(5).expect("region 5 should exist");

    assert_eq!(region0.rect, URect::new(0, 0, 2, 2));
    assert_eq!(region5.rect, URect::new(2, 2, 4, 4));
}

#[test]
fn slicing_returns_exact_tile_pixels() {
    let mut image = BinaryImage::new(4, 2);
    image.fill_rect(2, 0, 2, 2);
    let slicer = AtlasSlicer::from_grid(image, UVec2::new(2, 2), 2, 1, None, None);

    let tile = slicer.slice_index(1).expect("second tile should slice");

    assert_eq!(tile.width(), 2);
    assert_eq!(tile.height(), 2);
    assert_eq!(tile.filled_count(), 4);
}

#[test]
fn offset_and_padding_regions_match_exact_source_pixels() {
    let mut image = BinaryImage::new(10, 6);
    image.fill_rect(3, 1, 2, 2);
    let slicer = AtlasSlicer::from_grid(
        image.clone(),
        UVec2::new(2, 2),
        2,
        1,
        Some(UVec2::new(1, 0)),
        Some(UVec2::new(3, 1)),
    );

    let region = slicer
        .region_for_index(0)
        .expect("first region should exist");
    assert_eq!(region.rect, URect::new(3, 1, 5, 3));

    let tile = slicer.slice_index(0).expect("offset tile should slice");
    for y in 0..tile.height() {
        for x in 0..tile.width() {
            assert_eq!(
                tile.get(x, y),
                image.get(region.rect.min.x + x, region.rect.min.y + y)
            );
        }
    }
}

#[test]
fn atlas_collider_bake_skips_empty_tiles_and_preserves_order() {
    let mut image = BinaryImage::new(8, 4);
    image.fill_rect(0, 0, 2, 2);
    image.fill_rect(4, 0, 2, 4);
    let slicer = AtlasSlicer::from_grid(image, UVec2::new(2, 2), 4, 2, None, None);

    let baked =
        bake_atlas_collider_frames(&slicer, &ColliderGenConfig::default()).expect("atlas bake");

    assert_eq!(baked.len(), 3);
    assert_eq!(baked[0].region.index, 0);
    assert_eq!(baked[1].region.index, 2);
    assert_eq!(baked[2].region.index, 6);
    assert!(baked.iter().all(|frame| !frame.result.contours.is_empty()));
}
