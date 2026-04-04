use bevy::math::Rect;
use bevy::prelude::*;

use crate::{Contour, ContourTopology};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum ColliderGenError {
    EmptyImage,
    UnsupportedImageFormat(String),
    InvalidSubRegion,
    InvalidPolygon(String),
    TriangulationFailed(String),
    MarchingSquaresFailed(String),
}

impl std::fmt::Display for ColliderGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyImage => write!(f, "binary image is empty"),
            Self::UnsupportedImageFormat(format) => {
                write!(f, "unsupported image format: {format}")
            }
            Self::InvalidSubRegion => write!(f, "requested sub-region is outside the image"),
            Self::InvalidPolygon(message) => write!(f, "invalid polygon: {message}"),
            Self::TriangulationFailed(message) => write!(f, "triangulation failed: {message}"),
            Self::MarchingSquaresFailed(message) => {
                write!(f, "marching squares failed: {message}")
            }
        }
    }
}

impl std::error::Error for ColliderGenError {}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColliderGenWarning {
    DegenerateContourSkipped { index: usize },
    SimplificationRetried { index: usize, retry_count: u32 },
    SimplificationFallback { index: usize },
    DirtyRegionEmpty,
    HoleAwareDecompositionRecommended,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColliderGenResult {
    pub contours: Vec<Contour>,
    pub topology: Vec<ContourTopology>,
    pub convex_hulls: Vec<Contour>,
    pub convex_pieces: Vec<crate::CompoundPolygon>,
    pub bounds: Rect,
    pub warnings: Vec<ColliderGenWarning>,
}
