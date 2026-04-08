use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::contour::CompoundPolygon;
use crate::topology::{build_topology, ContourTopology};
use crate::triangulation::triangulate_simple_polygon;
use crate::validate::{is_convex, is_simple_polygon, remove_degenerate_edges};
use crate::{
    extract_pixel_exact_contours, BinaryImage, ColliderGenConfig, ColliderGenError, Contour,
    CoordinateTransform,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConvexPieceMetadata {
    pub piece_count: usize,
    pub total_area: f32,
}

pub fn convex_decompose_mask(
    mask: &BinaryImage,
    transform: CoordinateTransform,
) -> Result<Vec<CompoundPolygon>, ColliderGenError> {
    let (contours, _) = extract_pixel_exact_contours(mask, transform)?;
    let topology = build_topology(&contours);
    collect_convex_pieces(&contours, &topology, usize::MAX, 0.0)
}

pub(crate) fn convex_decompose_with_config(
    contours: &[Contour],
    topology: &[ContourTopology],
    config: &ColliderGenConfig,
) -> Result<Vec<CompoundPolygon>, ColliderGenError> {
    if !config.decomposition.enabled {
        return Ok(Vec::new());
    }

    collect_convex_pieces(
        contours,
        topology,
        config.decomposition.max_piece_count,
        config.decomposition.min_piece_area,
    )
}

pub(crate) fn summarize_pieces(pieces: &[CompoundPolygon]) -> ConvexPieceMetadata {
    ConvexPieceMetadata {
        piece_count: pieces.len(),
        total_area: pieces.iter().map(|piece| piece.area).sum(),
    }
}

fn collect_convex_pieces(
    contours: &[Contour],
    topology: &[ContourTopology],
    max_piece_count: usize,
    min_piece_area: f32,
) -> Result<Vec<CompoundPolygon>, ColliderGenError> {
    let mut pieces = Vec::new();

    for (index, contour) in contours.iter().enumerate() {
        if topology[index].is_hole || !topology[index].children.is_empty() {
            continue;
        }
        pieces.extend(convex_decompose_polygon(
            &contour.points,
            usize::MAX,
            min_piece_area,
        )?);
    }

    sort_compound_pieces(&mut pieces);
    if pieces.len() > max_piece_count {
        pieces.truncate(max_piece_count);
    }
    Ok(pieces)
}

fn sort_compound_pieces(pieces: &mut [CompoundPolygon]) {
    pieces.sort_by(|left, right| {
        right
            .area
            .partial_cmp(&left.area)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.offset
                    .x
                    .partial_cmp(&right.offset.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                left.offset
                    .y
                    .partial_cmp(&right.offset.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.points.len().cmp(&right.points.len()))
    });
}

fn convex_decompose_polygon(
    points: &[Vec2],
    max_piece_count: usize,
    min_piece_area: f32,
) -> Result<Vec<CompoundPolygon>, ColliderGenError> {
    let triangles = triangulate_simple_polygon(points)?;
    let mut polygons: Vec<Vec<Vec2>> = triangles
        .into_iter()
        .map(|triangle| triangle.vertices.to_vec())
        .collect();

    loop {
        let mut merged_any = false;
        'search: for left in 0..polygons.len() {
            for right in (left + 1)..polygons.len() {
                let Some(merged) = try_merge_polygons(&polygons[left], &polygons[right]) else {
                    continue;
                };
                if !is_convex(&merged) || !is_simple_polygon(&merged) {
                    continue;
                }

                polygons[left] = merged;
                polygons.remove(right);
                merged_any = true;
                break 'search;
            }
        }

        if !merged_any || polygons.len() <= max_piece_count {
            break;
        }
    }

    let pieces = polygons
        .into_iter()
        .map(|points| remove_degenerate_edges(&points, 1.0e-5))
        .filter(|points| points.len() >= 3)
        .map(CompoundPolygon::from_points)
        .filter(|piece| piece.area >= min_piece_area)
        .collect();
    Ok(pieces)
}

fn try_merge_polygons(left: &[Vec2], right: &[Vec2]) -> Option<Vec<Vec2>> {
    for left_index in 0..left.len() {
        let left_start = left[left_index];
        let left_end = left[(left_index + 1) % left.len()];

        for right_index in 0..right.len() {
            let right_start = right[right_index];
            let right_end = right[(right_index + 1) % right.len()];

            if !approx_eq(left_start, right_end) || !approx_eq(left_end, right_start) {
                continue;
            }

            let left_path = ring_path(left, (left_index + 1) % left.len(), left_index);
            let right_path = ring_path(right, (right_index + 1) % right.len(), right_index);

            let mut merged = left_path;
            merged.extend(
                right_path
                    .iter()
                    .copied()
                    .skip(1)
                    .take(right_path.len().saturating_sub(2)),
            );

            let merged = remove_degenerate_edges(&merged, 1.0e-5);
            if merged.len() >= 3 {
                return Some(merged);
            }
        }
    }
    None
}

fn ring_path(points: &[Vec2], start: usize, end: usize) -> Vec<Vec2> {
    let mut path = Vec::new();
    let mut index = start;
    loop {
        path.push(points[index]);
        if index == end {
            break;
        }
        index = (index + 1) % points.len();
    }
    path
}

fn approx_eq(left: Vec2, right: Vec2) -> bool {
    left.distance_squared(right) <= 1.0e-5
}

#[cfg(test)]
#[path = "decompose_tests.rs"]
mod tests;
