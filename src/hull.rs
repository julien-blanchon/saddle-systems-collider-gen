use bevy::prelude::*;

fn cross(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

pub fn convex_hull(points: &[Vec2]) -> Vec<Vec2> {
    if points.len() <= 1 {
        return points.to_vec();
    }

    let mut sorted = points.to_vec();
    sorted.sort_by(|left, right| {
        left.x
            .partial_cmp(&right.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.y
                    .partial_cmp(&right.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    sorted.dedup_by(|left, right| left.distance_squared(*right) <= 1.0e-5);

    if sorted.len() <= 2 {
        return sorted;
    }

    let mut lower = Vec::new();
    for point in &sorted {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper = Vec::new();
    for point in sorted.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
#[path = "hull_tests.rs"]
mod tests;
