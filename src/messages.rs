use bevy::prelude::*;

#[derive(Message, Clone, Debug)]
pub struct ColliderGenFinished {
    pub entity: Entity,
    pub contour_count: usize,
    pub convex_piece_count: usize,
}

#[derive(Message, Clone, Debug)]
pub struct ColliderGenFailed {
    pub entity: Entity,
    pub error: String,
}
