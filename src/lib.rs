#![doc = include_str!("../README.md")]

mod atlas;
mod binary_image;
mod components;
mod config;
mod contour;
mod decompose;
mod errors;
mod hull;
mod marching_squares;
mod messages;
mod simplify;
mod systems;
mod topology;
mod triangulation;
mod validate;

pub use atlas::{AtlasRegion, AtlasSlicer};
pub use binary_image::BinaryImage;
pub use components::{
    ColliderGenDirty, ColliderGenOutput, ColliderGenSource, ColliderGenSourceKind,
};
pub use config::{
    ColliderGenConfig, ColliderGenLod, ColorKey, ContourMode, DecompositionConfig, ImageMaskConfig,
    MaskChannelMode, RawImageFormat, SimplificationConfig,
};
pub use contour::{
    CompoundPolygon, Contour, ContourSpace, ContourSummary, CoordinateTransform,
    DirtyRegionRequest, extract_pixel_exact_contours,
};
pub use decompose::{ConvexPieceMetadata, convex_decompose_mask};
pub use errors::{ColliderGenError, ColliderGenResult, ColliderGenWarning};
pub use hull::convex_hull;
pub use marching_squares::extract_marching_squares_contours;
pub use messages::{ColliderGenFailed, ColliderGenFinished};
pub use simplify::{SimplificationStats, simplify_contour};
pub use topology::{
    ContourTopology, Winding, bounds_for_contours, build_topology, centroid, normalize_winding,
    point_in_polygon, signed_area, winding,
};
pub use triangulation::{Triangle, triangulate_simple_polygon};
pub use validate::{
    ValidationIssue, has_self_intersections, is_convex, is_simple_polygon, measure_max_deviation,
    remove_degenerate_edges, remove_duplicate_vertices,
};

use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ColliderGenSystems {
    Extract,
    Generate,
    Validate,
    Cache,
}

pub struct ColliderGenPlugin;

impl Plugin for ColliderGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ColliderGenFinished>()
            .add_message::<ColliderGenFailed>()
            .init_resource::<systems::PendingMessages>()
            .register_type::<BinaryImage>()
            .register_type::<ColliderGenConfig>()
            .register_type::<ColliderGenDirty>()
            .register_type::<ColliderGenOutput>()
            .register_type::<ColliderGenSource>()
            .register_type::<ColliderGenSourceKind>()
            .register_type::<AtlasRegion>()
            .register_type::<CompoundPolygon>()
            .register_type::<Contour>()
            .register_type::<ContourSpace>()
            .register_type::<ContourTopology>()
            .register_type::<ConvexPieceMetadata>()
            .register_type::<ImageMaskConfig>()
            .register_type::<MaskChannelMode>()
            .register_type::<ColorKey>()
            .register_type::<ContourMode>()
            .register_type::<RawImageFormat>()
            .register_type::<SimplificationConfig>()
            .register_type::<ColliderGenLod>()
            .register_type::<DecompositionConfig>()
            .configure_sets(
                Update,
                (
                    ColliderGenSystems::Extract,
                    ColliderGenSystems::Generate,
                    ColliderGenSystems::Validate,
                    ColliderGenSystems::Cache,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    systems::extract_sources.in_set(ColliderGenSystems::Extract),
                    systems::generate_geometry.in_set(ColliderGenSystems::Generate),
                    systems::publish_messages.in_set(ColliderGenSystems::Cache),
                ),
            );
    }
}

pub fn generate_collider_geometry(
    mask: &BinaryImage,
    config: &ColliderGenConfig,
) -> Result<ColliderGenResult, ColliderGenError> {
    let transform = CoordinateTransform::centered(mask.width(), mask.height(), config.scale);
    let (contours, contour_warnings) = match config.contour_mode {
        ContourMode::PixelExact => contour::extract_pixel_exact_contours(mask, transform),
        ContourMode::MarchingSquares => extract_marching_squares_contours(mask, transform),
    }?;
    let (contours, simplify_warnings) = simplify::simplify_contours(&contours, config)?;
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
    let hulls = contour::build_hulls(&contours);
    let mut convex_pieces = decompose::convex_decompose_with_config(&contours, &topology, config)?;
    sort_compound_pieces(&mut convex_pieces);
    let bounds = bounds_for_contours(&contours).unwrap_or_default();

    let mut warnings = contour_warnings;
    warnings.extend(simplify_warnings);
    warnings.extend(validate::validate_topology(&contours, &topology));
    if config.decomposition.enabled
        && topology.iter().enumerate().any(|(index, entry)| {
            !entry.is_hole && !entry.children.is_empty() && index < contours.len()
        })
    {
        warnings.push(ColliderGenWarning::HoleAwareDecompositionRecommended);
    }
    if !config.decomposition.enabled {
        convex_pieces.clear();
    }

    Ok(ColliderGenResult {
        contours,
        topology,
        convex_hulls: hulls,
        convex_pieces,
        bounds,
        warnings,
    })
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

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
