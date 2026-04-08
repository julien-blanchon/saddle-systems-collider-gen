use bevy::math::URect;
use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColliderGenGenerationKind {
    FullRebuild,
    DirtyRegionMerged,
    DirtyRegionFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColliderGenGenerationSummary {
    pub kind: ColliderGenGenerationKind,
    pub dirty_source_region: Option<URect>,
}

impl ColliderGenGenerationSummary {
    pub const fn full_rebuild() -> Self {
        Self {
            kind: ColliderGenGenerationKind::FullRebuild,
            dirty_source_region: None,
        }
    }

    pub const fn dirty_region(kind: ColliderGenGenerationKind, region: URect) -> Self {
        Self {
            kind,
            dirty_source_region: Some(region),
        }
    }
}
