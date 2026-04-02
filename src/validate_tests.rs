use bevy::prelude::*;

use super::*;

#[test]
fn self_intersection_detection_catches_bow_tie() {
    let polygon = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
        Vec2::new(2.0, 0.0),
    ];

    assert!(has_self_intersections(&polygon));
    assert!(!is_simple_polygon(&polygon));
}

#[test]
fn duplicate_and_degenerate_edges_are_removed() {
    let polygon = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];

    let deduped = remove_duplicate_vertices(&polygon, 1.0e-5);
    let cleaned = remove_degenerate_edges(&polygon, 1.0e-5);

    assert!(deduped.len() < polygon.len());
    assert!(cleaned.len() < polygon.len());
}

#[test]
fn convexity_check_distinguishes_square_from_l_shape() {
    let square = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
    ];
    let l_shape = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 2.0),
        Vec2::new(0.0, 2.0),
    ];

    assert!(is_convex(&square));
    assert!(!is_convex(&l_shape));
}

#[test]
fn zero_area_polygons_are_reported() {
    let polygon = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
    ];

    let issues = validate_polygon(&polygon, 1.0e-5);

    assert!(issues.contains(&ValidationIssue::ZeroArea));
}
