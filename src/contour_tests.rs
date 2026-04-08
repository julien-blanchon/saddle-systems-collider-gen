use bevy::prelude::*;

use super::*;
use crate::BinaryImage;

#[test]
fn single_pixel_generates_axis_aligned_square_contour() {
    let mut mask = BinaryImage::new(1, 1);
    mask.set(0, 0, true);

    let (contours, warnings) =
        extract_pixel_exact_contours(&mask, CoordinateTransform::centered(1, 1, Vec2::ONE))
            .expect("single pixel contour should extract");

    assert!(warnings.is_empty());
    assert_eq!(contours.len(), 1);
    assert_eq!(contours[0].vertex_count(), 4);
    assert_eq!(crate::signed_area(&contours[0].points).abs(), 1.0);
}

#[test]
fn disconnected_islands_generate_multiple_contours() {
    let mut mask = BinaryImage::new(4, 2);
    mask.set(0, 0, true);
    mask.set(3, 1, true);

    let (contours, _) =
        extract_pixel_exact_contours(&mask, CoordinateTransform::centered(4, 2, Vec2::ONE))
            .expect("two islands should extract");

    assert_eq!(contours.len(), 2);
}

#[test]
fn donut_mask_generates_outer_and_hole_loops() {
    let mut mask = BinaryImage::new(5, 5);
    mask.fill_rect(0, 0, 5, 5);
    mask.carve_rect(1, 1, 3, 3);

    let (contours, _) =
        extract_pixel_exact_contours(&mask, CoordinateTransform::centered(5, 5, Vec2::ONE))
            .expect("donut contour extraction should succeed");

    assert_eq!(contours.len(), 2);
}

#[test]
fn fully_filled_mask_generates_single_outer_loop() {
    let mut mask = BinaryImage::new(4, 3);
    mask.fill_rect(0, 0, 4, 3);

    let (contours, warnings) =
        extract_pixel_exact_contours(&mask, CoordinateTransform::centered(4, 3, Vec2::ONE))
            .expect("filled contour extraction should succeed");

    assert!(warnings.is_empty());
    assert_eq!(contours.len(), 1);
    assert_eq!(crate::signed_area(&contours[0].points).abs(), 12.0);
}

#[test]
fn touching_diagonals_stay_as_separate_islands() {
    let mut mask = BinaryImage::new(2, 2);
    mask.set(0, 0, true);
    mask.set(1, 1, true);

    let (contours, warnings) =
        extract_pixel_exact_contours(&mask, CoordinateTransform::centered(2, 2, Vec2::ONE))
            .expect("diagonal islands should extract");

    assert!(warnings.is_empty());
    assert_eq!(contours.len(), 2);
    assert!(
        contours
            .iter()
            .all(|contour| (crate::signed_area(&contour.points).abs() - 1.0).abs() <= 1.0e-5)
    );
}
