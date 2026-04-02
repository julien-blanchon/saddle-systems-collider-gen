use super::*;

#[test]
fn generated_result_contains_topology_and_bounds() {
    let mut mask = BinaryImage::new(5, 5);
    mask.fill_rect(0, 0, 5, 5);
    mask.carve_rect(1, 1, 3, 3);

    let result = generate_collider_geometry(&mask, &ColliderGenConfig::default())
        .expect("generation should succeed");

    assert_eq!(result.contours.len(), 2);
    assert_eq!(result.topology.len(), 2);
    assert!(result.bounds.width() > 0.0);
    assert!(result.bounds.height() > 0.0);
}
