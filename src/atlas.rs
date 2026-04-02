use bevy::math::{URect, UVec2};
use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{BinaryImage, ColliderGenError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AtlasRegion {
    pub index: usize,
    pub column: u32,
    pub row: u32,
    pub rect: URect,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AtlasSlicer {
    source: BinaryImage,
    tile_size: UVec2,
    columns: u32,
    rows: u32,
    padding: UVec2,
    offset: UVec2,
}

impl AtlasSlicer {
    pub fn from_grid(
        source: BinaryImage,
        tile_size: UVec2,
        columns: u32,
        rows: u32,
        padding: Option<UVec2>,
        offset: Option<UVec2>,
    ) -> Self {
        Self {
            source,
            tile_size,
            columns,
            rows,
            padding: padding.unwrap_or_default(),
            offset: offset.unwrap_or_default(),
        }
    }

    pub fn tile_size(&self) -> UVec2 {
        self.tile_size
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    pub fn len(&self) -> usize {
        (self.columns as usize) * (self.rows as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn region_for_index(&self, index: usize) -> Option<AtlasRegion> {
        if index >= self.len() {
            return None;
        }
        let column = (index as u32) % self.columns;
        let row = (index as u32) / self.columns;
        let rect = self.rect_for_cell(column, row);
        Some(AtlasRegion {
            index,
            column,
            row,
            rect,
        })
    }

    pub fn slice_index(&self, index: usize) -> Result<BinaryImage, ColliderGenError> {
        let region = self.region_for_index(index).ok_or_else(|| {
            ColliderGenError::InvalidPolygon(format!("atlas index {index} is out of bounds"))
        })?;
        self.slice_rect(region.rect)
    }

    pub fn slice_rect(&self, rect: URect) -> Result<BinaryImage, ColliderGenError> {
        if rect.max.x > self.source.width() || rect.max.y > self.source.height() {
            return Err(ColliderGenError::InvalidSubRegion);
        }
        Ok(self.source.crop(rect))
    }

    pub fn iter_regions(&self) -> impl Iterator<Item = AtlasRegion> + '_ {
        (0..self.len()).filter_map(|index| self.region_for_index(index))
    }

    fn rect_for_cell(&self, column: u32, row: u32) -> URect {
        let x = self.offset.x + column * (self.tile_size.x + self.padding.x);
        let y = self.offset.y + row * (self.tile_size.y + self.padding.y);
        URect::from_corners(
            UVec2::new(x, y),
            UVec2::new(x + self.tile_size.x, y + self.tile_size.y),
        )
    }
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;
