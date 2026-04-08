use std::collections::HashMap;

use bevy::math::{IRect, Rect, URect};
use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::errors::{ColliderGenError, ColliderGenWarning};
use crate::validate::remove_degenerate_edges;
use crate::{convex_hull, BinaryImage};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ContourSpace {
    Pixel,
    LocalCentered,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoordinateTransform {
    pub image_size: UVec2,
    pub scale: Vec2,
}

impl CoordinateTransform {
    pub fn centered(width: u32, height: u32, scale: Vec2) -> Self {
        Self {
            image_size: UVec2::new(width, height),
            scale,
        }
    }

    pub fn pixel_to_local(self, point: Vec2) -> Vec2 {
        let half = self.image_size.as_vec2() * 0.5;
        Vec2::new(
            (point.x - half.x) * self.scale.x,
            (point.y - half.y) * self.scale.y,
        )
    }

    pub fn local_to_pixel(self, point: Vec2) -> Vec2 {
        let half = self.image_size.as_vec2() * 0.5;
        Vec2::new(
            point.x / self.scale.x + half.x,
            point.y / self.scale.y + half.y,
        )
    }

    pub fn rect_to_local(self, rect: Rect) -> Rect {
        Rect {
            min: self.pixel_to_local(rect.min),
            max: self.pixel_to_local(rect.max),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Contour {
    pub points: Vec<Vec2>,
    pub space: ContourSpace,
}

impl Contour {
    pub fn new(points: Vec<Vec2>, space: ContourSpace) -> Self {
        Self { points, space }
    }

    pub fn pixel(points: Vec<Vec2>) -> Self {
        Self::new(points, ContourSpace::Pixel)
    }

    pub fn local(points: Vec<Vec2>) -> Self {
        Self::new(points, ContourSpace::LocalCentered)
    }

    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.points.iter();
        let first = *points.next()?;
        let mut min = first;
        let mut max = first;
        for point in points {
            min = min.min(*point);
            max = max.max(*point);
        }
        Some(Rect { min, max })
    }

    pub fn vertex_count(&self) -> usize {
        self.points.len()
    }

    pub fn summary(&self, index: usize) -> ContourSummary {
        ContourSummary {
            index,
            vertex_count: self.points.len(),
            bounds: self.bounds().unwrap_or_default(),
            signed_area: crate::signed_area(&self.points),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ContourSummary {
    pub index: usize,
    pub vertex_count: usize,
    pub bounds: Rect,
    pub signed_area: f32,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompoundPolygon {
    pub offset: Vec2,
    pub points: Vec<Vec2>,
    pub area: f32,
}

impl CompoundPolygon {
    pub fn from_points(points: Vec<Vec2>) -> Self {
        let offset = crate::centroid(&points).unwrap_or(Vec2::ZERO);
        let local_points = points.iter().map(|point| *point - offset).collect();
        Self {
            offset,
            points: local_points,
            area: crate::signed_area(&points).abs(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DirtyRegionRequest {
    pub rect: Option<IRect>,
    pub margin: u32,
}

impl DirtyRegionRequest {
    pub fn expanded(self, image_size: UVec2) -> Option<URect> {
        let rect = self.rect?;
        let margin = self.margin as i32;
        let min = IVec2::new((rect.min.x - margin).max(0), (rect.min.y - margin).max(0));
        let max = IVec2::new(
            (rect.max.x + margin).min(image_size.x as i32),
            (rect.max.y + margin).min(image_size.y as i32),
        );
        Some(URect::from_corners(min.as_uvec2(), max.as_uvec2()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct GridPoint {
    x: i32,
    y: i32,
}

impl GridPoint {
    fn as_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectedEdge {
    start: GridPoint,
    end: GridPoint,
}

pub fn extract_pixel_exact_contours(
    mask: &BinaryImage,
    transform: CoordinateTransform,
) -> Result<(Vec<Contour>, Vec<ColliderGenWarning>), ColliderGenError> {
    if mask.is_empty() {
        return Err(ColliderGenError::EmptyImage);
    }

    let edges = extract_boundary_edges(mask);
    let successors = compute_successors(&edges)?;
    let mut start_indices: Vec<usize> = (0..edges.len()).collect();
    start_indices.sort_by_key(|index| {
        let edge = edges[*index];
        (
            edge.start.y,
            edge.start.x,
            direction_rank(edge.start, edge.end),
        )
    });

    let mut visited = vec![false; edges.len()];
    let mut contours = Vec::new();
    let mut warnings = Vec::new();

    for start in start_indices {
        if visited[start] {
            continue;
        }

        let mut points = Vec::new();
        let mut current = start;
        loop {
            if visited[current] {
                break;
            }
            visited[current] = true;
            points.push(edges[current].start.as_vec2());
            let next = successors[current];
            if next == start {
                break;
            }
            current = next;
        }

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

    sort_contours(&mut contours);
    Ok((contours, warnings))
}

pub fn build_hulls(contours: &[Contour]) -> Vec<Contour> {
    contours
        .iter()
        .filter_map(|contour| {
            let hull = convex_hull(&contour.points);
            (hull.len() >= 3).then_some(Contour::new(hull, contour.space))
        })
        .collect()
}

fn extract_boundary_edges(mask: &BinaryImage) -> Vec<DirectedEdge> {
    let mut edges = Vec::new();
    let height = mask.height() as i32;

    for y in 0..mask.height() as i32 {
        for x in 0..mask.width() as i32 {
            if !mask.get_i32(x, y) {
                continue;
            }

            let x0 = x;
            let x1 = x + 1;
            let y0 = height - y - 1;
            let y1 = y0 + 1;

            if !mask.get_i32(x, y + 1) {
                edges.push(DirectedEdge {
                    start: GridPoint { x: x0, y: y0 },
                    end: GridPoint { x: x1, y: y0 },
                });
            }
            if !mask.get_i32(x + 1, y) {
                edges.push(DirectedEdge {
                    start: GridPoint { x: x1, y: y0 },
                    end: GridPoint { x: x1, y: y1 },
                });
            }
            if !mask.get_i32(x, y - 1) {
                edges.push(DirectedEdge {
                    start: GridPoint { x: x1, y: y1 },
                    end: GridPoint { x: x0, y: y1 },
                });
            }
            if !mask.get_i32(x - 1, y) {
                edges.push(DirectedEdge {
                    start: GridPoint { x: x0, y: y1 },
                    end: GridPoint { x: x0, y: y0 },
                });
            }
        }
    }

    edges
}

fn compute_successors(edges: &[DirectedEdge]) -> Result<Vec<usize>, ColliderGenError> {
    let mut outgoing: HashMap<GridPoint, Vec<(u8, usize)>> = HashMap::new();
    for (index, edge) in edges.iter().enumerate() {
        outgoing
            .entry(edge.start)
            .or_default()
            .push((direction_rank(edge.start, edge.end), index));
    }

    for entry in outgoing.values_mut() {
        entry.sort_by_key(|(direction, _)| *direction);
    }

    let mut successors = Vec::with_capacity(edges.len());
    for edge in edges {
        let reverse_rank = direction_rank(edge.end, edge.start);
        let options = outgoing.get(&edge.end).ok_or_else(|| {
            ColliderGenError::InvalidPolygon("dangling boundary edge".to_string())
        })?;

        let mut selected = None;
        for step in 1..=4u8 {
            let wanted = (reverse_rank + 4 - step) % 4;
            if let Some((_, index)) = options.iter().find(|(direction, _)| *direction == wanted) {
                selected = Some(*index);
                break;
            }
        }

        successors.push(selected.ok_or_else(|| {
            ColliderGenError::InvalidPolygon("boundary graph has no valid successor".to_string())
        })?);
    }

    Ok(successors)
}

fn direction_rank(start: GridPoint, end: GridPoint) -> u8 {
    match (end.x - start.x, end.y - start.y) {
        (1, 0) => 0,
        (0, 1) => 1,
        (-1, 0) => 2,
        (0, -1) => 3,
        delta => panic!("invalid contour edge direction: {delta:?}"),
    }
}

fn sort_contours(contours: &mut [Contour]) {
    contours.sort_by(|left, right| {
        let left_area = crate::signed_area(&left.points).abs();
        let right_area = crate::signed_area(&right.points).abs();
        right_area
            .partial_cmp(&left_area)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let left_bounds = left.bounds().unwrap_or_default();
                let right_bounds = right.bounds().unwrap_or_default();
                left_bounds
                    .min
                    .x
                    .partial_cmp(&right_bounds.min.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        left_bounds
                            .min
                            .y
                            .partial_cmp(&right_bounds.min.y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| left.points.len().cmp(&right.points.len()))
            })
    });
}

#[cfg(test)]
#[path = "contour_tests.rs"]
mod tests;
