use bevy::prelude::*;

use super::*;
use crate::BinaryImage;

#[test]
fn marching_squares_extracts_single_component() {
    let mut mask = BinaryImage::new(2, 2);
    mask.fill_rect(0, 0, 2, 2);

    let (contours, warnings) =
        extract_marching_squares_contours(&mask, CoordinateTransform::centered(2, 2, Vec2::ONE))
            .expect("marching squares should succeed");

    assert!(warnings.is_empty());
    assert_eq!(contours.len(), 1);
    assert!(contours[0].vertex_count() >= 4);
}

#[test]
fn marching_squares_handles_donut_shapes() {
    let mut mask = BinaryImage::new(5, 5);
    mask.fill_rect(0, 0, 5, 5);
    mask.carve_rect(1, 1, 3, 3);

    let (contours, _) =
        extract_marching_squares_contours(&mask, CoordinateTransform::centered(5, 5, Vec2::ONE))
            .expect("marching squares should extract a donut");

    assert_eq!(contours.len(), 2);
}
