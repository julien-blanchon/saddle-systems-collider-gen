use std::collections::HashMap;

use bevy::prelude::*;

use crate::validate::remove_degenerate_edges;
use crate::{BinaryImage, ColliderGenError, ColliderGenWarning, Contour, CoordinateTransform};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SegmentPoint(IVec2);

impl SegmentPoint {
    fn from_vec2(point: Vec2) -> Self {
        Self(IVec2::new(
            (point.x * 2.0).round() as i32,
            (point.y * 2.0).round() as i32,
        ))
    }

    fn as_vec2(self) -> Vec2 {
        Vec2::new(self.0.x as f32 * 0.5, self.0.y as f32 * 0.5)
    }
}

pub fn extract_marching_squares_contours(
    mask: &BinaryImage,
    transform: CoordinateTransform,
) -> Result<(Vec<Contour>, Vec<ColliderGenWarning>), ColliderGenError> {
    if mask.is_empty() {
        return Err(ColliderGenError::EmptyImage);
    }

    let mut segments = Vec::<(SegmentPoint, SegmentPoint)>::new();
    for y in 0..=mask.height() {
        for x in 0..=mask.width() {
            let bl = sample(mask, x as i32, y as i32);
            let br = sample(mask, x as i32 + 1, y as i32);
            let tr = sample(mask, x as i32 + 1, y as i32 + 1);
            let tl = sample(mask, x as i32, y as i32 + 1);
            let case = (bl as u8) | ((br as u8) << 1) | ((tr as u8) << 2) | ((tl as u8) << 3);
            let origin = Vec2::new(x as f32 - 0.5, y as f32 - 0.5);
            segments.extend(case_segments(case, origin));
        }
    }

    let mut adjacency: HashMap<SegmentPoint, Vec<usize>> = HashMap::new();
    for (index, (start, end)) in segments.iter().enumerate() {
        adjacency.entry(*start).or_default().push(index);
        adjacency.entry(*end).or_default().push(index);
    }

    let mut used = vec![false; segments.len()];
    let mut contours = Vec::new();
    let mut warnings = Vec::new();

    for segment_index in 0..segments.len() {
        if used[segment_index] {
            continue;
        }

        used[segment_index] = true;
        let (start, mut current) = segments[segment_index];
        let mut points = vec![start.as_vec2(), current.as_vec2()];

        while current != start {
            let Some(candidates) = adjacency.get(&current) else {
                return Err(ColliderGenError::MarchingSquaresFailed(
                    "segment adjacency is incomplete".to_string(),
                ));
            };
            let Some(next_index) = candidates.iter().copied().find(|index| !used[*index]) else {
                break;
            };

            used[next_index] = true;
            let (a, b) = segments[next_index];
            current = if a == current { b } else { a };
            points.push(current.as_vec2());
        }

        if current != start {
            warnings.push(ColliderGenWarning::DegenerateContourSkipped {
                index: contours.len(),
            });
            continue;
        }

        points.pop();
        let cleaned = remove_degenerate_edges(&points, 1.0e-5);
        if cleaned.len() < 3 {
            warnings.push(ColliderGenWarning::DegenerateContourSkipped {
                index: contours.len(),
            });
            continue;
        }

        let local_points = cleaned
            .into_iter()
            .map(|point| transform.pixel_to_local(point))
            .collect();
        contours.push(Contour::local(local_points));
    }

    contours.sort_by(|left, right| {
        crate::signed_area(&right.points)
            .abs()
            .partial_cmp(&crate::signed_area(&left.points).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok((contours, warnings))
}

fn sample(mask: &BinaryImage, x: i32, y: i32) -> bool {
    if x <= 0 || y <= 0 || x > mask.width() as i32 || y > mask.height() as i32 {
        return false;
    }
    let image_x = (x - 1) as u32;
    let image_y = mask.height() - y as u32;
    mask.get(image_x, image_y)
}

fn case_segments(case: u8, origin: Vec2) -> Vec<(SegmentPoint, SegmentPoint)> {
    let left = SegmentPoint::from_vec2(origin + Vec2::new(0.0, 0.5));
    let bottom = SegmentPoint::from_vec2(origin + Vec2::new(0.5, 0.0));
    let right = SegmentPoint::from_vec2(origin + Vec2::new(1.0, 0.5));
    let top = SegmentPoint::from_vec2(origin + Vec2::new(0.5, 1.0));

    match case {
        0 | 15 => Vec::new(),
        1 | 14 => vec![(left, bottom)],
        2 | 13 => vec![(bottom, right)],
        3 | 12 => vec![(left, right)],
        4 | 11 => vec![(right, top)],
        5 => vec![(left, top), (bottom, right)],
        6 | 9 => vec![(bottom, top)],
        7 | 8 => vec![(left, top)],
        10 => vec![(top, right), (left, bottom)],
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[path = "marching_squares_tests.rs"]
mod tests;
