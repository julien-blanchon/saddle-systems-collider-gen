use bevy::prelude::*;
use proptest::prelude::*;

use super::*;

#[test]
fn hull_wraps_extreme_points() {
    let points = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
        Vec2::new(1.0, 1.0),
    ];

    let hull = convex_hull(&points);

    assert_eq!(hull.len(), 4);
    assert!(hull.contains(&Vec2::new(0.0, 0.0)));
    assert!(hull.contains(&Vec2::new(2.0, 2.0)));
}

proptest! {
    #[test]
    fn hull_contains_all_input_points(
        coords in prop::collection::vec((-10.0f32..10.0, -10.0f32..10.0), 3..24)
    ) {
        let points: Vec<Vec2> = coords
            .into_iter()
            .map(|(x, y)| Vec2::new(x, y))
            .collect();
        let hull = convex_hull(&points);
        prop_assume!(hull.len() >= 3);

        for point in &points {
            prop_assert!(
                crate::point_in_polygon(*point, &hull)
                    || hull
                        .iter()
                        .any(|vertex| vertex.distance_squared(*point) <= 1.0e-4)
            );
        }
    }
}
