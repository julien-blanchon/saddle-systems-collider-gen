use bevy::prelude::*;
use proptest::prelude::*;

use super::*;
use crate::{BinaryImage, CoordinateTransform};

#[test]
fn build_topology_identifies_hole_relationships() {
    let mut mask = BinaryImage::new(5, 5);
    mask.fill_rect(0, 0, 5, 5);
    mask.carve_rect(1, 1, 3, 3);

    let (contours, _) =
        crate::extract_pixel_exact_contours(&mask, CoordinateTransform::centered(5, 5, Vec2::ONE))
            .expect("donut contour extraction should succeed");
    let topology = build_topology(&contours);

    assert_eq!(topology.len(), 2);
    assert!(
        topology
            .iter()
            .any(|entry| entry.parent.is_none() && !entry.is_hole)
    );
    assert!(
        topology
            .iter()
            .any(|entry| entry.parent.is_some() && entry.is_hole)
    );
}

#[test]
fn winding_normalization_respects_requested_orientation() {
    let contour = crate::Contour::pixel(vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
    ]);
    let normalized = normalize_winding(&contour, Winding::CounterClockwise);

    assert_eq!(winding(&normalized.points), Winding::CounterClockwise);
}

#[test]
fn point_in_polygon_treats_boundary_as_inside() {
    let polygon = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
    ];

    assert!(point_in_polygon(Vec2::new(1.0, 1.0), &polygon));
    assert!(point_in_polygon(Vec2::new(0.0, 1.0), &polygon));
    assert!(!point_in_polygon(Vec2::new(3.0, 1.0), &polygon));
}

proptest! {
    #[test]
    fn winding_normalization_is_idempotent(
        coords in prop::collection::vec((-8.0f32..8.0, -8.0f32..8.0), 3..18)
    ) {
        let hull = crate::convex_hull(
            &coords
                .into_iter()
                .map(|(x, y)| Vec2::new(x, y))
                .collect::<Vec<_>>(),
        );
        prop_assume!(hull.len() >= 3);

        let contour = crate::Contour::local(hull);
        let once = normalize_winding(&contour, Winding::CounterClockwise);
        let twice = normalize_winding(&once, Winding::CounterClockwise);

        prop_assert_eq!(winding(&once.points), Winding::CounterClockwise);
        prop_assert_eq!(once.points, twice.points);
    }
}
