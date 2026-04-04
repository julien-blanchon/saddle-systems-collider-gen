use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::validate::{ValidationIssue, measure_max_deviation, validate_polygon};
use crate::{ColliderGenConfig, ColliderGenError, ColliderGenWarning, Contour};

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SimplificationStats {
    pub original_vertices: usize,
    pub final_vertices: usize,
    pub retries: u32,
    pub max_deviation: f32,
}

pub fn simplify_contour(
    contour: &Contour,
    config: &ColliderGenConfig,
) -> Result<(Contour, SimplificationStats, Vec<ColliderGenWarning>), ColliderGenError> {
    let original = contour.points.clone();
    let mut warnings = Vec::new();
    let mut epsilon = config.simplification;

    for retry in 0..=config.simplification.max_retries {
        let mut points = remove_collinear_points(&original, epsilon.collinear_epsilon.max(1.0e-5));
        if epsilon.rdp_epsilon > 0.0 {
            points = ramer_douglas_peucker_closed(&points, epsilon.rdp_epsilon.max(1.0e-5));
        }
        if epsilon.visvalingam_area_threshold > 0.0 {
            points = visvalingam_whyatt_closed(
                &points,
                epsilon.visvalingam_area_threshold.max(1.0e-5),
                config.minimum_vertices,
            );
        }

        let issues = validate_polygon(&points, epsilon.collinear_epsilon.max(1.0e-5));
        let valid = issues.is_empty()
            && points.len() >= config.minimum_vertices
            && crate::signed_area(&points).abs() >= config.minimum_area;

        if valid {
            let stats = SimplificationStats {
                original_vertices: original.len(),
                final_vertices: points.len(),
                retries: retry,
                max_deviation: measure_max_deviation(&original, &points),
            };
            return Ok((Contour::new(points, contour.space), stats, warnings));
        }

        if retry == config.simplification.max_retries {
            return Err(ColliderGenError::InvalidPolygon(format!(
                "simplification produced invalid output: {issues:?}"
            )));
        }

        warnings.push(ColliderGenWarning::SimplificationRetried {
            index: 0,
            retry_count: retry + 1,
        });
        epsilon.rdp_epsilon *= epsilon.retry_scale.max(0.1);
        epsilon.visvalingam_area_threshold *= epsilon.retry_scale.max(0.1);
        epsilon.collinear_epsilon *= epsilon.retry_scale.max(0.1);
    }

    Err(ColliderGenError::InvalidPolygon(
        "simplification exhausted retries".to_string(),
    ))
}

pub(crate) fn simplify_contours(
    contours: &[Contour],
    config: &ColliderGenConfig,
) -> Result<(Vec<Contour>, Vec<ColliderGenWarning>), ColliderGenError> {
    let mut simplified = Vec::with_capacity(contours.len());
    let mut warnings = Vec::new();

    for (index, contour) in contours.iter().enumerate() {
        match simplify_contour(contour, config) {
            Ok((simplified_contour, _, contour_warnings)) => {
                warnings.extend(contour_warnings.into_iter().map(|warning| match warning {
                    ColliderGenWarning::SimplificationRetried { retry_count, .. } => {
                        ColliderGenWarning::SimplificationRetried { index, retry_count }
                    }
                    other => other,
                }));
                simplified.push(simplified_contour);
            }
            Err(_) => {
                // Simplification failed to produce a valid polygon even after all retries.
                // Fall back to the original unsimplified contour and emit a warning so
                // callers are informed without crashing.
                warnings.push(ColliderGenWarning::SimplificationFallback { index });
                simplified.push(contour.clone());
            }
        }
    }

    Ok((simplified, warnings))
}

fn remove_collinear_points(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut simplified = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let cross = (current.x - previous.x) * (next.y - current.y)
            - (current.y - previous.y) * (next.x - current.x);
        if cross.abs() <= epsilon {
            continue;
        }
        simplified.push(current);
    }

    if simplified.len() < 3 {
        points.to_vec()
    } else {
        simplified
    }
}

fn ramer_douglas_peucker_closed(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    if points.len() < 4 || epsilon <= 0.0 {
        return points.to_vec();
    }

    let mut polyline = points.to_vec();
    polyline.push(points[0]);
    let mut reduced = rdp_recursive(&polyline, epsilon);
    if reduced.len() > 1
        && reduced
            .first()
            .zip(reduced.last())
            .is_some_and(|(first, last)| first.distance_squared(*last) <= 1.0e-5)
    {
        reduced.pop();
    }
    reduced
}

fn rdp_recursive(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    if points.len() <= 2 {
        return vec![points[0], *points.last().unwrap_or(&points[0])];
    }

    let start = points[0];
    let end = *points.last().unwrap_or(&start);
    let mut index = 0;
    let mut max_distance = 0.0;

    for (point_index, point) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let distance = distance_to_segment(*point, start, end);
        if distance > max_distance {
            max_distance = distance;
            index = point_index;
        }
    }

    if max_distance <= epsilon {
        return vec![start, end];
    }

    let mut left = rdp_recursive(&points[..=index], epsilon);
    let right = rdp_recursive(&points[index..], epsilon);
    left.pop();
    left.extend(right);
    left
}

fn visvalingam_whyatt_closed(
    points: &[Vec2],
    threshold: f32,
    minimum_vertices: usize,
) -> Vec<Vec2> {
    if points.len() <= minimum_vertices || threshold <= 0.0 {
        return points.to_vec();
    }

    let mut working = points.to_vec();
    while working.len() > minimum_vertices {
        let mut best_index = None;
        let mut best_area = f32::MAX;
        for index in 0..working.len() {
            let previous = working[(index + working.len() - 1) % working.len()];
            let current = working[index];
            let next = working[(index + 1) % working.len()];
            let area = triangle_area(previous, current, next).abs();
            if area < best_area {
                best_area = area;
                best_index = Some(index);
            }
        }

        if best_area > threshold {
            break;
        }

        let Some(index) = best_index else {
            break;
        };
        let mut candidate = working.clone();
        candidate.remove(index);
        if validate_polygon(&candidate, 1.0e-5).contains(&ValidationIssue::SelfIntersection) {
            break;
        }
        working = candidate;
    }

    working
}

fn triangle_area(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) * 0.5
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let delta = end - start;
    let length_squared = delta.length_squared();
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let t = ((point - start).dot(delta) / length_squared).clamp(0.0, 1.0);
    point.distance(start + delta * t)
}

#[cfg(test)]
#[path = "simplify_tests.rs"]
mod tests;
