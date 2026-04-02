use bevy::prelude::*;

#[test]
fn concave_l_shape_decomposes_into_convex_pieces() {
    let polygon = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 2.0),
        Vec2::new(0.0, 2.0),
    ];

    let pieces = super::convex_decompose_polygon(&polygon, usize::MAX, 0.0)
        .expect("L-shape should decompose");

    assert!(pieces.len() >= 2);
    assert!(pieces.iter().all(|piece| crate::is_convex(&piece.points)));
    let total_area: f32 = pieces.iter().map(|piece| piece.area).sum();
    assert!((total_area - 3.0).abs() <= 1.0e-4);
}
