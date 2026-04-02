use bevy::math::Rect;
use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Contour;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Winding {
    Clockwise,
    CounterClockwise,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ContourTopology {
    pub contour_index: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub is_hole: bool,
}

pub fn signed_area(points: &[Vec2]) -> f32 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

pub fn winding(points: &[Vec2]) -> Winding {
    if signed_area(points) < 0.0 {
        Winding::Clockwise
    } else {
        Winding::CounterClockwise
    }
}

pub fn normalize_winding(contour: &Contour, desired: Winding) -> Contour {
    let mut normalized = contour.clone();
    if winding(&normalized.points) != desired {
        normalized.points.reverse();
    }
    normalized
}

pub fn centroid(points: &[Vec2]) -> Option<Vec2> {
    if points.len() < 3 {
        return None;
    }
    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let cross = current.x * next.y - next.x * current.y;
        area += cross;
        cx += (current.x + next.x) * cross;
        cy += (current.y + next.y) * cross;
    }

    let area = area * 0.5;
    (area.abs() > 1.0e-5).then(|| Vec2::new(cx / (6.0 * area), cy / (6.0 * area)))
}

pub fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        if point_on_segment(a, b, point) {
            return true;
        }
        let intersects = ((a.y > point.y) != (b.y > point.y))
            && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y + f32::EPSILON) + a.x);
        if intersects {
            inside = !inside;
        }
    }
    inside
}

pub fn bounds_for_contours(contours: &[Contour]) -> Option<Rect> {
    let mut iter = contours.iter().filter_map(Contour::bounds);
    let mut bounds = iter.next()?;
    for rect in iter {
        bounds.min = bounds.min.min(rect.min);
        bounds.max = bounds.max.max(rect.max);
    }
    Some(bounds)
}

pub fn build_topology(contours: &[Contour]) -> Vec<ContourTopology> {
    let areas: Vec<f32> = contours
        .iter()
        .map(|contour| signed_area(&contour.points))
        .collect();
    let hole_flags: Vec<bool> = areas.iter().map(|area| *area < 0.0).collect();
    let sample_points: Vec<Vec2> = contours
        .iter()
        .map(|contour| centroid(&contour.points).unwrap_or(contour.points[0]))
        .collect();

    let mut topology = Vec::with_capacity(contours.len());
    for index in 0..contours.len() {
        let mut parent = None;
        let mut parent_area = f32::MAX;
        for other_index in 0..contours.len() {
            if index == other_index || hole_flags[index] == hole_flags[other_index] {
                continue;
            }
            if !point_in_polygon(sample_points[index], &contours[other_index].points) {
                continue;
            }
            let area = areas[other_index].abs();
            if area < parent_area && area > areas[index].abs() {
                parent_area = area;
                parent = Some(other_index);
            }
        }

        topology.push(ContourTopology {
            contour_index: index,
            parent,
            children: Vec::new(),
            is_hole: hole_flags[index],
        });
    }

    for index in 0..topology.len() {
        if let Some(parent) = topology[index].parent {
            topology[parent].children.push(index);
        }
    }

    topology
}

fn point_on_segment(a: Vec2, b: Vec2, point: Vec2) -> bool {
    let ab = b - a;
    let ap = point - a;
    let cross = ab.x * ap.y - ab.y * ap.x;
    if cross.abs() > 1.0e-5 {
        return false;
    }
    point.x >= a.x.min(b.x) - 1.0e-5
        && point.x <= a.x.max(b.x) + 1.0e-5
        && point.y >= a.y.min(b.y) - 1.0e-5
        && point.y <= a.y.max(b.y) + 1.0e-5
}

#[cfg(test)]
#[path = "topology_tests.rs"]
mod tests;
