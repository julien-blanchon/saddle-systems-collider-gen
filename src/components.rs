use bevy::math::{IRect, URect};
use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{BinaryImage, ColliderGenConfig, ColliderGenResult, ConvexPieceMetadata};

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub enum ColliderGenSourceKind {
    Binary(BinaryImage),
    Image {
        handle: Handle<Image>,
        region: Option<URect>,
    },
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct ColliderGenSource {
    pub kind: ColliderGenSourceKind,
    pub config: ColliderGenConfig,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColliderGenDirty {
    pub region: Option<IRect>,
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColliderGenOutput {
    pub result: ColliderGenResult,
    pub source_region: Option<URect>,
    pub piece_summary: ConvexPieceMetadata,
}
