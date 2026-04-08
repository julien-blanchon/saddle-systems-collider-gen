use bevy::prelude::*;

use crate::ColliderGenGenerationSummary;

#[derive(Message, Clone, Debug)]
pub struct ColliderGenFinished {
    pub entity: Entity,
    pub contour_count: usize,
    pub convex_piece_count: usize,
    pub generation: ColliderGenGenerationSummary,
}

#[derive(Message, Clone, Debug)]
pub struct ColliderGenFailed {
    pub entity: Entity,
    pub error: String,
}
