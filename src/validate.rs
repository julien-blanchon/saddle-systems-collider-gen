use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{ColliderGenWarning, Contour, ContourTopology};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ValidationIssue {
    TooFewVertices,
    DuplicateVertices,
    DegenerateEdge,
    ZeroArea,
    SelfIntersection,
}

pub fn remove_duplicate_vertices(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    let mut deduped = Vec::with_capacity(points.len());
    for point in points {
        if deduped
            .last()
            .is_some_and(|previous: &Vec2| previous.distance_squared(*point) <= epsilon * epsilon)
        {
            continue;
        }
        deduped.push(*point);
    }

    if deduped.len() > 1
        && deduped
            .first()
            .zip(deduped.last())
            .is_some_and(|(first, last)| first.distance_squared(*last) <= epsilon * epsilon)
    {
        deduped.pop();
    }

    deduped
}

pub fn remove_degenerate_edges(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    let deduped = remove_duplicate_vertices(points, epsilon);
    if deduped.len() < 3 {
        return deduped;
    }

    let mut cleaned = Vec::with_capacity(deduped.len());
    for index in 0..deduped.len() {
        let previous = deduped[(index + deduped.len() - 1) % deduped.len()];
        let current = deduped[index];
        let next = deduped[(index + 1) % deduped.len()];
        if previous.distance_squared(current) <= epsilon * epsilon
            || current.distance_squared(next) <= epsilon * epsilon
        {
            continue;
        }
        cleaned.push(current);
    }

    cleaned
}

pub fn has_self_intersections(points: &[Vec2]) -> bool {
    if points.len() < 4 {
        return false;
    }

    for a_index in 0..points.len() {
        let a_start = points[a_index];
        let a_end = points[(a_index + 1) % points.len()];
        for b_index in (a_index + 1)..points.len() {
            if edges_are_adjacent(a_index, b_index, points.len()) {
                continue;
            }

            let b_start = points[b_index];
            let b_end = points[(b_index + 1) % points.len()];
            if segments_intersect(a_start, a_end, b_start, b_end) {
                return true;
            }
        }
    }

    false
}

pub fn is_simple_polygon(points: &[Vec2]) -> bool {
    points.len() >= 3 && signed_area(points).abs() > 1.0e-5 && !has_self_intersections(points)
}

pub fn is_convex(points: &[Vec2]) -> bool {
    if points.len() < 3 {
        return false;
    }

    let mut sign = 0.0f32;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        let c = points[(index + 2) % points.len()];
        let cross = cross_z(b - a, c - b);
        if cross.abs() <= 1.0e-5 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
            continue;
        }
        if cross.signum() != sign.signum() {
            return false;
        }
    }

    true
}

pub fn measure_max_deviation(original: &[Vec2], simplified: &[Vec2]) -> f32 {
    if original.is_empty() || simplified.is_empty() {
        return 0.0;
    }

    let mut max_distance: f32 = 0.0;
    for point in original {
        let mut best = f32::MAX;
        for index in 0..simplified.len() {
            let start = simplified[index];
            let end = simplified[(index + 1) % simplified.len()];
            best = best.min(distance_to_segment(*point, start, end));
        }
        max_distance = max_distance.max(best);
    }

    max_distance
}

pub fn validate_polygon(points: &[Vec2], epsilon: f32) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if points.len() < 3 {
        issues.push(ValidationIssue::TooFewVertices);
        return issues;
    }
    if remove_duplicate_vertices(points, epsilon).len() != points.len() {
        issues.push(ValidationIssue::DuplicateVertices);
    }
    if remove_degenerate_edges(points, epsilon).len() != points.len() {
        issues.push(ValidationIssue::DegenerateEdge);
    }
    if signed_area(points).abs() <= epsilon {
        issues.push(ValidationIssue::ZeroArea);
    }
    if has_self_intersections(points) {
        issues.push(ValidationIssue::SelfIntersection);
    }
    issues
}

pub(crate) fn validate_topology(
    contours: &[Contour],
    topology: &[ContourTopology],
) -> Vec<ColliderGenWarning> {
    let mut warnings = Vec::new();
    for (index, entry) in topology.iter().enumerate() {
        let Some(contour) = contours.get(index) else {
            warnings.push(ColliderGenWarning::DegenerateContourSkipped { index });
            continue;
        };
        let area = signed_area(&contour.points);
        if (entry.is_hole && area > 0.0) || (!entry.is_hole && area < 0.0) {
            warnings.push(ColliderGenWarning::DegenerateContourSkipped { index });
        }
    }
    warnings
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_squared();
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let t = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn signed_area(points: &[Vec2]) -> f32 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

fn cross_z(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn orientation(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    cross_z(b - a, c - a)
}

fn on_segment(a: Vec2, b: Vec2, point: Vec2) -> bool {
    point.x >= a.x.min(b.x) - 1.0e-5
        && point.x <= a.x.max(b.x) + 1.0e-5
        && point.y >= a.y.min(b.y) - 1.0e-5
        && point.y <= a.y.max(b.y) + 1.0e-5
}

fn segments_intersect(a_start: Vec2, a_end: Vec2, b_start: Vec2, b_end: Vec2) -> bool {
    let o1 = orientation(a_start, a_end, b_start);
    let o2 = orientation(a_start, a_end, b_end);
    let o3 = orientation(b_start, b_end, a_start);
    let o4 = orientation(b_start, b_end, a_end);

    if o1.signum() != o2.signum() && o3.signum() != o4.signum() {
        return true;
    }

    if o1.abs() <= 1.0e-5 && on_segment(a_start, a_end, b_start) {
        return true;
    }
    if o2.abs() <= 1.0e-5 && on_segment(a_start, a_end, b_end) {
        return true;
    }
    if o3.abs() <= 1.0e-5 && on_segment(b_start, b_end, a_start) {
        return true;
    }
    if o4.abs() <= 1.0e-5 && on_segment(b_start, b_end, a_end) {
        return true;
    }

    false
}

fn edges_are_adjacent(a_index: usize, b_index: usize, len: usize) -> bool {
    a_index == b_index
        || (a_index + 1) % len == b_index
        || (b_index + 1) % len == a_index
        || (a_index == 0 && b_index + 1 == len)
        || (b_index == 0 && a_index + 1 == len)
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
