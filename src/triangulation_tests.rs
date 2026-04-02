use bevy::prelude::*;

use super::*;

#[test]
fn square_triangulates_into_two_triangles() {
    let polygon = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
    ];

    let triangles = triangulate_simple_polygon(&polygon).expect("square should triangulate");
    let total_area: f32 = triangles
        .iter()
        .map(|triangle| crate::signed_area(&triangle.vertices).abs())
        .sum();

    assert_eq!(triangles.len(), 2);
    assert!((total_area - 4.0).abs() <= 1.0e-5);
}
