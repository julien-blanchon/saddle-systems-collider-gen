use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{ColliderGenError, Contour, Winding, is_simple_polygon, normalize_winding};

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Triangle {
    pub vertices: [Vec2; 3],
}

pub fn triangulate_simple_polygon(points: &[Vec2]) -> Result<Vec<Triangle>, ColliderGenError> {
    if points.len() < 3 {
        return Err(ColliderGenError::TriangulationFailed(
            "polygon has fewer than three vertices".to_string(),
        ));
    }
    if !is_simple_polygon(points) {
        return Err(ColliderGenError::TriangulationFailed(
            "polygon is not simple".to_string(),
        ));
    }

    let contour = normalize_winding(&Contour::pixel(points.to_vec()), Winding::CounterClockwise);
    let points = contour.points;
    let mut indices: Vec<usize> = (0..points.len()).collect();
    let mut triangles = Vec::new();

    while indices.len() > 3 {
        let mut ear_found = false;
        for index in 0..indices.len() {
            let previous = indices[(index + indices.len() - 1) % indices.len()];
            let current = indices[index];
            let next = indices[(index + 1) % indices.len()];

            let triangle = [points[previous], points[current], points[next]];
            if !is_convex_triangle(triangle) || triangle_area(triangle).abs() <= 1.0e-5 {
                continue;
            }
            if contains_any_point(triangle, &points, &indices, previous, current, next) {
                continue;
            }

            triangles.push(Triangle { vertices: triangle });
            indices.remove(index);
            ear_found = true;
            break;
        }

        if !ear_found {
            return Err(ColliderGenError::TriangulationFailed(
                "no ear could be clipped".to_string(),
            ));
        }
    }

    if indices.len() == 3 {
        triangles.push(Triangle {
            vertices: [points[indices[0]], points[indices[1]], points[indices[2]]],
        });
    }

    Ok(triangles)
}

fn is_convex_triangle(triangle: [Vec2; 3]) -> bool {
    triangle_area(triangle) > 0.0
}

fn triangle_area(triangle: [Vec2; 3]) -> f32 {
    let [a, b, c] = triangle;
    ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) * 0.5
}

fn contains_any_point(
    triangle: [Vec2; 3],
    points: &[Vec2],
    indices: &[usize],
    previous: usize,
    current: usize,
    next: usize,
) -> bool {
    indices
        .iter()
        .copied()
        .filter(|index| *index != previous && *index != current && *index != next)
        .any(|index| point_in_triangle(points[index], triangle))
}

fn point_in_triangle(point: Vec2, triangle: [Vec2; 3]) -> bool {
    let [a, b, c] = triangle;
    let ab = sign(point, a, b);
    let bc = sign(point, b, c);
    let ca = sign(point, c, a);

    let has_negative = ab < 0.0 || bc < 0.0 || ca < 0.0;
    let has_positive = ab > 0.0 || bc > 0.0 || ca > 0.0;
    !(has_negative && has_positive)
}

fn sign(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    (point.x - b.x) * (a.y - b.y) - (a.x - b.x) * (point.y - b.y)
}

#[cfg(test)]
#[path = "triangulation_tests.rs"]
mod tests;
