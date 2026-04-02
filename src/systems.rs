use bevy::prelude::*;

use crate::contour::DirtyRegionRequest;
use crate::decompose::summarize_pieces;
use crate::{
    BinaryImage, ColliderGenDirty, ColliderGenError, ColliderGenFailed, ColliderGenFinished,
    ColliderGenOutput, ColliderGenResult, ColliderGenSource, ColliderGenSourceKind,
    CompoundPolygon, Contour, CoordinateTransform, Winding, bounds_for_contours, build_topology,
    normalize_winding,
};

#[derive(Component, Clone)]
pub(crate) struct PreparedBinaryMask {
    pub mask: BinaryImage,
    pub full_mask: Option<BinaryImage>,
    pub source_region: Option<URect>,
}

#[derive(Resource, Default)]
pub(crate) struct PendingMessages {
    finished: Vec<ColliderGenFinished>,
    failed: Vec<ColliderGenFailed>,
}

pub(crate) fn extract_sources(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    query: Query<(
        Entity,
        Ref<ColliderGenSource>,
        Option<&ColliderGenDirty>,
        Option<&ColliderGenOutput>,
    )>,
) {
    for (entity, source, dirty, output) in &query {
        if output.is_some() && !source.is_changed() && dirty.is_none() {
            continue;
        }

        let prepared = match &source.kind {
            ColliderGenSourceKind::Binary(binary) => {
                prepare_binary_source(binary.clone(), source.config.dirty_region_margin, dirty)
            }
            ColliderGenSourceKind::Image { handle, region } => {
                let Some(image) = images.get(handle) else {
                    continue;
                };
                let base = match region {
                    Some(region) => {
                        match BinaryImage::from_bevy_image_region(
                            image,
                            *region,
                            &source.config.image,
                        ) {
                            Ok(mask) => mask,
                            Err(_) => continue,
                        }
                    }
                    None => match BinaryImage::from_bevy_image(image, &source.config.image) {
                        Ok(mask) => mask,
                        Err(_) => continue,
                    },
                };
                prepare_binary_source(base, source.config.dirty_region_margin, dirty)
            }
        };

        commands.entity(entity).insert(prepared);
    }
}

pub(crate) fn generate_geometry(
    mut commands: Commands,
    mut pending: ResMut<PendingMessages>,
    query: Query<(
        Entity,
        &ColliderGenSource,
        &PreparedBinaryMask,
        Option<&ColliderGenOutput>,
    )>,
) {
    for (entity, source, prepared, previous) in &query {
        match generate_prepared_result(source, prepared, previous) {
            Ok(result) => {
                let piece_summary = summarize_pieces(&result.convex_pieces);
                let contour_count = result.contours.len();
                let convex_piece_count = result.convex_pieces.len();
                commands.entity(entity).insert(ColliderGenOutput {
                    result,
                    source_region: prepared.source_region,
                    piece_summary,
                });
                commands.entity(entity).remove::<ColliderGenDirty>();
                commands.entity(entity).remove::<PreparedBinaryMask>();
                pending.finished.push(ColliderGenFinished {
                    entity,
                    contour_count,
                    convex_piece_count,
                });
            }
            Err(error) => {
                commands.entity(entity).remove::<PreparedBinaryMask>();
                pending.failed.push(ColliderGenFailed {
                    entity,
                    error: describe_error(error),
                });
            }
        }
    }
}

pub(crate) fn publish_messages(
    mut pending: ResMut<PendingMessages>,
    mut finished: MessageWriter<ColliderGenFinished>,
    mut failed: MessageWriter<ColliderGenFailed>,
) {
    for message in pending.finished.drain(..) {
        finished.write(message);
    }
    for message in pending.failed.drain(..) {
        failed.write(message);
    }
}

fn prepare_binary_source(
    binary: BinaryImage,
    dirty_margin: u32,
    dirty: Option<&ColliderGenDirty>,
) -> PreparedBinaryMask {
    let request = dirty.map(|dirty| DirtyRegionRequest {
        rect: dirty.region,
        margin: dirty_margin,
    });
    let source_region =
        request.and_then(|request| request.expanded(UVec2::new(binary.width(), binary.height())));
    let (mask, full_mask) = if let Some(region) = source_region {
        (binary.crop(region), Some(binary))
    } else {
        (binary, None)
    };
    PreparedBinaryMask {
        mask,
        full_mask,
        source_region,
    }
}

fn generate_prepared_result(
    source: &ColliderGenSource,
    prepared: &PreparedBinaryMask,
    previous: Option<&ColliderGenOutput>,
) -> Result<ColliderGenResult, ColliderGenError> {
    let can_merge_dirty_region = prepared.source_region.is_some()
        && previous.is_some()
        && !mask_touches_boundary(&prepared.mask);

    if can_merge_dirty_region {
        let partial = crate::generate_collider_geometry(&prepared.mask, &source.config)?;
        let source_region = prepared
            .source_region
            .expect("checked above that dirty source region exists");
        let full_mask = prepared
            .full_mask
            .as_ref()
            .expect("dirty-region updates keep the full mask for fallback and merging");
        return Ok(merge_dirty_region_result(
            &previous
                .expect("checked above that previous output exists")
                .result,
            partial,
            source_region,
            UVec2::new(full_mask.width(), full_mask.height()),
            source.config.scale,
        ));
    }

    let mask = prepared.full_mask.as_ref().unwrap_or(&prepared.mask);
    crate::generate_collider_geometry(mask, &source.config)
}

fn merge_dirty_region_result(
    previous: &ColliderGenResult,
    mut partial: ColliderGenResult,
    source_region: URect,
    full_image_size: UVec2,
    scale: Vec2,
) -> ColliderGenResult {
    let translation = dirty_region_translation(source_region, full_image_size, scale);
    translate_result(&mut partial, translation);

    let replacement_zone = replacement_zone_rect(source_region, full_image_size, scale);

    let mut contours: Vec<_> = previous
        .contours
        .iter()
        .filter(|contour| !rect_intersects(contour.bounds(), replacement_zone))
        .cloned()
        .collect();
    contours.extend(partial.contours);
    sort_contours(&mut contours);

    let topology = build_topology(&contours);
    let contours: Vec<_> = contours
        .into_iter()
        .enumerate()
        .map(|(index, contour)| {
            let desired = if topology[index].is_hole {
                Winding::Clockwise
            } else {
                Winding::CounterClockwise
            };
            normalize_winding(&contour, desired)
        })
        .collect();
    let topology = build_topology(&contours);

    let mut convex_hulls: Vec<_> = previous
        .convex_hulls
        .iter()
        .filter(|contour| !rect_intersects(contour.bounds(), replacement_zone))
        .cloned()
        .collect();
    convex_hulls.extend(partial.convex_hulls);
    sort_contours(&mut convex_hulls);

    let mut convex_pieces: Vec<_> = previous
        .convex_pieces
        .iter()
        .filter(|piece| !rect_intersects(compound_bounds(piece), replacement_zone))
        .cloned()
        .collect();
    convex_pieces.extend(partial.convex_pieces);
    sort_compound_pieces(&mut convex_pieces);

    let bounds = bounds_for_contours(&contours).unwrap_or_default();
    let mut warnings = previous.warnings.clone();
    for warning in partial.warnings {
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }

    ColliderGenResult {
        contours,
        topology,
        convex_hulls,
        convex_pieces,
        bounds,
        warnings,
    }
}

fn translate_result(result: &mut ColliderGenResult, translation: Vec2) {
    for contour in &mut result.contours {
        for point in &mut contour.points {
            *point += translation;
        }
    }
    for contour in &mut result.convex_hulls {
        for point in &mut contour.points {
            *point += translation;
        }
    }
    for piece in &mut result.convex_pieces {
        piece.offset += translation;
    }
    result.bounds.min += translation;
    result.bounds.max += translation;
}

fn dirty_region_translation(source_region: URect, full_image_size: UVec2, scale: Vec2) -> Vec2 {
    let full_transform = CoordinateTransform::centered(full_image_size.x, full_image_size.y, scale);
    let crop_size = source_region.max - source_region.min;
    let crop_transform = CoordinateTransform::centered(crop_size.x, crop_size.y, scale);
    let full_origin = Vec2::new(
        source_region.min.x as f32,
        (full_image_size.y - source_region.max.y) as f32,
    );

    full_transform.pixel_to_local(full_origin) - crop_transform.pixel_to_local(Vec2::ZERO)
}

fn replacement_zone_rect(source_region: URect, full_image_size: UVec2, scale: Vec2) -> Rect {
    let transform = CoordinateTransform::centered(full_image_size.x, full_image_size.y, scale);
    let min = Vec2::new(
        source_region.min.x as f32,
        (full_image_size.y - source_region.max.y) as f32,
    );
    let max = Vec2::new(
        source_region.max.x as f32,
        (full_image_size.y - source_region.min.y) as f32,
    );

    Rect {
        min: transform.pixel_to_local(min),
        max: transform.pixel_to_local(max),
    }
}

fn compound_bounds(piece: &CompoundPolygon) -> Option<Rect> {
    let mut points = piece.points.iter();
    let first = *points.next()? + piece.offset;
    let mut min = first;
    let mut max = first;
    for point in points {
        let point = *point + piece.offset;
        min = min.min(point);
        max = max.max(point);
    }
    Some(Rect { min, max })
}

fn rect_intersects(left: Option<Rect>, right: Rect) -> bool {
    let Some(left) = left else {
        return false;
    };

    left.min.x < right.max.x
        && left.max.x > right.min.x
        && left.min.y < right.max.y
        && left.max.y > right.min.y
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

fn mask_touches_boundary(mask: &BinaryImage) -> bool {
    let width = mask.width();
    let height = mask.height();

    if width == 0 || height == 0 {
        return false;
    }

    (0..width).any(|x| mask.get(x, 0) || mask.get(x, height - 1))
        || (0..height).any(|y| mask.get(0, y) || mask.get(width - 1, y))
}

fn describe_error(error: ColliderGenError) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
