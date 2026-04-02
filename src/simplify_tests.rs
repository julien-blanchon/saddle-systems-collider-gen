use bevy::prelude::*;

use super::*;
use crate::{ColliderGenConfig, Contour};

#[test]
fn collinear_cleanup_reduces_rectangular_runs() {
    let contour = Contour::local(vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 1.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
        Vec2::new(0.0, 1.0),
    ]);

    let (simplified, stats, warnings) = simplify_contour(&contour, &ColliderGenConfig::default())
        .expect("simplification should work");

    assert!(warnings.is_empty());
    assert_eq!(simplified.points.len(), 4);
    assert!(stats.final_vertices < stats.original_vertices);
}

#[test]
fn zero_epsilons_preserve_shape() {
    let contour = Contour::local(vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 2.0),
    ]);

    let config = ColliderGenConfig::default();
    let (simplified, _, _) = simplify_contour(&contour, &config).expect("shape should stay valid");

    assert_eq!(simplified.points, contour.points);
}

#[test]
fn aggressive_simplification_keeps_polygons_simple() {
    let contour = Contour::local(vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(4.0, 0.0),
        Vec2::new(4.0, 1.0),
        Vec2::new(3.0, 1.0),
        Vec2::new(3.0, 2.0),
        Vec2::new(4.0, 2.0),
        Vec2::new(4.0, 4.0),
        Vec2::new(0.0, 4.0),
    ]);
    let config = ColliderGenConfig {
        simplification: crate::SimplificationConfig {
            collinear_epsilon: 1.0e-4,
            rdp_epsilon: 1.0,
            visvalingam_area_threshold: 0.2,
            retry_scale: 0.5,
            max_retries: 3,
        },
        ..default()
    };

    let (simplified, _, _) =
        simplify_contour(&contour, &config).expect("simplification should stay valid");

    assert!(crate::is_simple_polygon(&simplified.points));
}
