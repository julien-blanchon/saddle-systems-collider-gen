use bevy::prelude::*;

use super::*;

#[test]
fn binary_sources_are_cropped_to_expanded_dirty_regions() {
    let mut image = crate::BinaryImage::new(8, 8);
    image.fill_rect(0, 0, 8, 8);

    let prepared = prepare_binary_source(
        image,
        1,
        Some(&crate::ColliderGenDirty {
            region: Some(IRect::new(2, 2, 4, 4)),
        }),
    );

    assert_eq!(prepared.source_region, Some(URect::new(1, 1, 5, 5)));
    assert_eq!(prepared.mask.width(), 4);
    assert_eq!(prepared.mask.height(), 4);
    assert_eq!(
        prepared
            .full_mask
            .as_ref()
            .map(|mask| UVec2::new(mask.width(), mask.height())),
        Some(UVec2::new(8, 8))
    );
}

#[test]
fn boundary_detection_only_flags_masks_with_filled_edges() {
    let mut isolated = crate::BinaryImage::new(7, 7);
    isolated.fill_rect(2, 2, 2, 2);
    assert!(!mask_touches_boundary(&isolated));

    let mut touching = crate::BinaryImage::new(7, 7);
    touching.fill_rect(0, 2, 3, 2);
    assert!(mask_touches_boundary(&touching));
}
