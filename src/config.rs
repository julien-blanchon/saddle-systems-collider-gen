use bevy::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MaskChannelMode {
    Alpha,
    Brightness,
    Luma,
    Red,
    Green,
    Blue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColorKey {
    pub rgba: [u8; 4],
    pub tolerance: u8,
}

impl ColorKey {
    pub fn matches(self, sample: [u8; 4]) -> bool {
        self.rgba
            .into_iter()
            .zip(sample)
            .all(|(expected, actual)| expected.abs_diff(actual) <= self.tolerance)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImageMaskConfig {
    pub alpha_threshold: u8,
    pub brightness_threshold: u8,
    pub channel_mode: MaskChannelMode,
    pub invert_mask: bool,
    pub color_key: Option<ColorKey>,
}

impl Default for ImageMaskConfig {
    fn default() -> Self {
        Self {
            alpha_threshold: 128,
            brightness_threshold: 128,
            channel_mode: MaskChannelMode::Alpha,
            invert_mask: false,
            color_key: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ContourMode {
    PixelExact,
    MarchingSquares,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColliderGenLod {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SimplificationConfig {
    pub collinear_epsilon: f32,
    pub rdp_epsilon: f32,
    pub visvalingam_area_threshold: f32,
    pub retry_scale: f32,
    pub max_retries: u32,
}

impl Default for SimplificationConfig {
    fn default() -> Self {
        Self {
            collinear_epsilon: 1.0e-5,
            rdp_epsilon: 0.0,
            visvalingam_area_threshold: 0.0,
            retry_scale: 0.5,
            max_retries: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DecompositionConfig {
    pub enabled: bool,
    pub max_piece_count: usize,
    pub min_piece_area: f32,
}

impl Default for DecompositionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_piece_count: 256,
            min_piece_area: 0.25,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RawImageFormat {
    R8,
    Rg8,
    Rgb8,
    Rgba8,
    Bgra8,
}

impl RawImageFormat {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::R8 => 1,
            Self::Rg8 => 2,
            Self::Rgb8 => 3,
            Self::Rgba8 | Self::Bgra8 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColliderGenConfig {
    pub image: ImageMaskConfig,
    pub contour_mode: ContourMode,
    pub scale: Vec2,
    pub simplification: SimplificationConfig,
    pub minimum_area: f32,
    pub minimum_vertices: usize,
    pub dirty_region_margin: u32,
    pub lod: ColliderGenLod,
    pub decomposition: DecompositionConfig,
}

impl Default for ColliderGenConfig {
    fn default() -> Self {
        Self {
            image: ImageMaskConfig::default(),
            contour_mode: ContourMode::PixelExact,
            scale: Vec2::ONE,
            simplification: SimplificationConfig::default(),
            minimum_area: 0.5,
            minimum_vertices: 3,
            dirty_region_margin: 2,
            lod: ColliderGenLod::High,
            decomposition: DecompositionConfig::default(),
        }
    }
}

impl ColliderGenConfig {
    pub fn with_lod(mut self, lod: ColliderGenLod) -> Self {
        self.lod = lod;
        self.simplification = match lod {
            ColliderGenLod::High => SimplificationConfig {
                collinear_epsilon: 1.0e-5,
                rdp_epsilon: 0.0,
                visvalingam_area_threshold: 0.0,
                retry_scale: 0.5,
                max_retries: 3,
            },
            ColliderGenLod::Medium => SimplificationConfig {
                collinear_epsilon: 1.0e-4,
                rdp_epsilon: 0.4,
                visvalingam_area_threshold: 0.05,
                retry_scale: 0.5,
                max_retries: 3,
            },
            ColliderGenLod::Low => SimplificationConfig {
                collinear_epsilon: 1.0e-4,
                rdp_epsilon: 1.0,
                visvalingam_area_threshold: 0.2,
                retry_scale: 0.5,
                max_retries: 4,
            },
        };
        self
    }
}
