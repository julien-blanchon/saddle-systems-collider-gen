use bevy::image::Image;
use bevy::math::{IRect, IVec2, Rect, URect, Vec2};
use bevy::prelude::*;

#[cfg(feature = "image")]
use image::DynamicImage;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::config::{ImageMaskConfig, MaskChannelMode, RawImageFormat};
use crate::errors::ColliderGenError;

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BinaryImage {
    width: u32,
    height: u32,
    pixels: Vec<bool>,
}

impl BinaryImage {
    pub fn new(width: u32, height: u32) -> Self {
        let len = width
            .checked_mul(height)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        Self {
            width,
            height,
            pixels: vec![false; len],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn filled_count(&self) -> usize {
        self.pixels.iter().copied().filter(|filled| *filled).count()
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        self.index_of(x, y).is_some_and(|index| self.pixels[index])
    }

    pub(crate) fn get_i32(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && self.get(x as u32, y as u32)
    }

    pub fn set(&mut self, x: u32, y: u32, value: bool) {
        if let Some(index) = self.index_of(x, y) {
            self.pixels[index] = value;
        }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(false);
    }

    pub fn invert(&mut self) {
        for pixel in &mut self.pixels {
            *pixel = !*pixel;
        }
    }

    pub fn crop(&self, rect: URect) -> Self {
        let min_x = rect.min.x.min(self.width);
        let min_y = rect.min.y.min(self.height);
        let max_x = rect.max.x.min(self.width);
        let max_y = rect.max.y.min(self.height);
        let mut cropped = Self::new(max_x.saturating_sub(min_x), max_y.saturating_sub(min_y));

        for y in min_y..max_y {
            for x in min_x..max_x {
                cropped.set(x - min_x, y - min_y, self.get(x, y));
            }
        }

        cropped
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        let max_x = x.saturating_add(width).min(self.width);
        let max_y = y.saturating_add(height).min(self.height);
        for yy in y.min(self.height)..max_y {
            for xx in x.min(self.width)..max_x {
                self.set(xx, yy, true);
            }
        }
    }

    pub fn carve_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        let max_x = x.saturating_add(width).min(self.width);
        let max_y = y.saturating_add(height).min(self.height);
        for yy in y.min(self.height)..max_y {
            for xx in x.min(self.width)..max_x {
                self.set(xx, yy, false);
            }
        }
    }

    pub fn fill_circle(&mut self, center: IVec2, radius: i32) {
        self.paint_circle(center, radius, true);
    }

    pub fn carve_circle(&mut self, center: IVec2, radius: i32) {
        self.paint_circle(center, radius, false);
    }

    pub fn fill_polygon(&mut self, polygon: &[Vec2]) {
        if polygon.len() < 3 {
            return;
        }

        let bounds = polygon_bounds(polygon);
        let min_x = bounds.min.x.floor().max(0.0) as u32;
        let min_y = bounds.min.y.floor().max(0.0) as u32;
        let max_x = bounds.max.x.ceil().min(self.width as f32) as u32;
        let max_y = bounds.max.y.ceil().min(self.height as f32) as u32;

        for y in min_y..max_y {
            for x in min_x..max_x {
                let sample = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if point_in_polygon(sample, polygon) {
                    self.set(x, y, true);
                }
            }
        }
    }

    pub fn dilate(&self, radius: u32) -> Self {
        if radius == 0 {
            return self.clone();
        }
        let mut output = Self::new(self.width, self.height);
        let radius = radius as i32;

        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let mut filled = false;
                for oy in -radius..=radius {
                    for ox in -radius..=radius {
                        if self.get_i32(x + ox, y + oy) {
                            filled = true;
                            break;
                        }
                    }
                    if filled {
                        break;
                    }
                }
                output.set(x as u32, y as u32, filled);
            }
        }

        output
    }

    pub fn erode(&self, radius: u32) -> Self {
        if radius == 0 {
            return self.clone();
        }
        let mut output = Self::new(self.width, self.height);
        let radius = radius as i32;

        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let mut filled = true;
                'outer: for oy in -radius..=radius {
                    for ox in -radius..=radius {
                        if !self.get_i32(x + ox, y + oy) {
                            filled = false;
                            break 'outer;
                        }
                    }
                }
                output.set(x as u32, y as u32, filled);
            }
        }

        output
    }

    pub fn open(&self, radius: u32) -> Self {
        self.erode(radius).dilate(radius)
    }

    pub fn close(&self, radius: u32) -> Self {
        self.dilate(radius).erode(radius)
    }

    pub fn grow(&self, radius: u32) -> Self {
        self.dilate(radius)
    }

    pub fn shrink(&self, radius: u32) -> Self {
        self.erode(radius)
    }

    pub fn stamp_mask(&mut self, source: &Self, top_left: UVec2) {
        for y in 0..source.height {
            for x in 0..source.width {
                if !source.get(x, y) {
                    continue;
                }

                let target_x = top_left.x.saturating_add(x);
                let target_y = top_left.y.saturating_add(y);
                if target_x >= self.width || target_y >= self.height {
                    continue;
                }
                self.set(target_x, target_y, true);
            }
        }
    }

    pub fn carve_mask(&mut self, source: &Self, top_left: UVec2) {
        for y in 0..source.height {
            for x in 0..source.width {
                if !source.get(x, y) {
                    continue;
                }

                let target_x = top_left.x.saturating_add(x);
                let target_y = top_left.y.saturating_add(y);
                if target_x >= self.width || target_y >= self.height {
                    continue;
                }
                self.set(target_x, target_y, false);
            }
        }
    }

    pub fn dirty_region_union(&self, other: &Self) -> Option<IRect> {
        if self.width != other.width || self.height != other.height {
            return Some(IRect::new(0, 0, self.width as i32, self.height as i32));
        }

        let mut min = IVec2::splat(i32::MAX);
        let mut max = IVec2::splat(i32::MIN);

        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) == other.get(x, y) {
                    continue;
                }
                min.x = min.x.min(x as i32);
                min.y = min.y.min(y as i32);
                max.x = max.x.max(x as i32 + 1);
                max.y = max.y.max(y as i32 + 1);
            }
        }

        (min.x <= max.x && min.y <= max.y).then(|| IRect::from_corners(min, max))
    }

    pub fn from_raw_bytes(
        width: u32,
        height: u32,
        bytes: &[u8],
        format: RawImageFormat,
        config: &ImageMaskConfig,
    ) -> Result<Self, ColliderGenError> {
        let expected = width as usize * height as usize * format.bytes_per_pixel();
        if bytes.len() != expected {
            return Err(ColliderGenError::UnsupportedImageFormat(format!(
                "expected {expected} bytes for {width}x{height} {format:?}, got {}",
                bytes.len()
            )));
        }

        let mut image = Self::new(width, height);
        for (index, pixel) in bytes.chunks_exact(format.bytes_per_pixel()).enumerate() {
            let sample = match format {
                RawImageFormat::R8 => [pixel[0], pixel[0], pixel[0], u8::MAX],
                RawImageFormat::Rg8 => [pixel[0], pixel[1], 0, u8::MAX],
                RawImageFormat::Rgb8 => [pixel[0], pixel[1], pixel[2], u8::MAX],
                RawImageFormat::Rgba8 => [pixel[0], pixel[1], pixel[2], pixel[3]],
                RawImageFormat::Bgra8 => [pixel[2], pixel[1], pixel[0], pixel[3]],
            };
            let x = (index as u32) % width;
            let y = (index as u32) / width;
            image.set(x, y, threshold_sample(sample, config));
        }

        Ok(image)
    }

    pub fn from_bevy_image(
        image: &Image,
        config: &ImageMaskConfig,
    ) -> Result<Self, ColliderGenError> {
        let size = image.texture_descriptor.size;
        if size.width == 0 || size.height == 0 {
            return Err(ColliderGenError::EmptyImage);
        }

        let mut binary = Self::new(size.width, size.height);
        for y in 0..size.height {
            for x in 0..size.width {
                let color = image
                    .get_color_at(x, y)
                    .map_err(|error| ColliderGenError::UnsupportedImageFormat(error.to_string()))?;
                let srgb = bevy::color::Srgba::from(color);
                let sample = [
                    channel_to_byte(srgb.red),
                    channel_to_byte(srgb.green),
                    channel_to_byte(srgb.blue),
                    channel_to_byte(srgb.alpha),
                ];
                binary.set(x, y, threshold_sample(sample, config));
            }
        }

        Ok(binary)
    }

    pub fn from_bevy_image_region(
        image: &Image,
        region: URect,
        config: &ImageMaskConfig,
    ) -> Result<Self, ColliderGenError> {
        let binary = Self::from_bevy_image(image, config)?;
        if region.max.x > binary.width || region.max.y > binary.height {
            return Err(ColliderGenError::InvalidSubRegion);
        }
        Ok(binary.crop(region))
    }

    #[cfg(feature = "image")]
    pub fn from_dynamic_image(
        image: &DynamicImage,
        config: &ImageMaskConfig,
    ) -> Result<Self, ColliderGenError> {
        let rgba = image.to_rgba8();
        Self::from_raw_bytes(
            rgba.width(),
            rgba.height(),
            rgba.as_raw(),
            RawImageFormat::Rgba8,
            config,
        )
    }

    pub(crate) fn index_of(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    fn paint_circle(&mut self, center: IVec2, radius: i32, value: bool) {
        if radius < 0 {
            return;
        }

        let min_x = (center.x - radius).max(0);
        let min_y = (center.y - radius).max(0);
        let max_x = (center.x + radius + 1).min(self.width as i32);
        let max_y = (center.y + radius + 1).min(self.height as i32);

        for y in min_y..max_y {
            for x in min_x..max_x {
                let delta = IVec2::new(x, y) - center;
                if delta.x * delta.x + delta.y * delta.y <= radius * radius {
                    self.set(x as u32, y as u32, value);
                }
            }
        }
    }
}

fn channel_to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * u8::MAX as f32).round() as u8
}

fn threshold_sample(sample: [u8; 4], config: &ImageMaskConfig) -> bool {
    if config.color_key.is_some_and(|key| key.matches(sample)) {
        return false;
    }

    let brightness = sample[0].max(sample[1]).max(sample[2]);
    let luma = (0.2126 * sample[0] as f32 + 0.7152 * sample[1] as f32 + 0.0722 * sample[2] as f32)
        .round() as u8;

    let filled = match config.channel_mode {
        MaskChannelMode::Alpha => sample[3] >= config.alpha_threshold,
        MaskChannelMode::Brightness => brightness >= config.brightness_threshold,
        MaskChannelMode::Luma => luma >= config.brightness_threshold,
        MaskChannelMode::Red => sample[0] >= config.brightness_threshold,
        MaskChannelMode::Green => sample[1] >= config.brightness_threshold,
        MaskChannelMode::Blue => sample[2] >= config.brightness_threshold,
    };

    if config.invert_mask {
        !filled
    } else {
        filled
    }
}

fn polygon_bounds(points: &[Vec2]) -> Rect {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for point in points {
        min = min.min(*point);
        max = max.max(*point);
    }
    Rect { min, max }
}

fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let intersects = ((a.y > point.y) != (b.y > point.y))
            && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y + f32::EPSILON) + a.x);
        if intersects {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
#[path = "binary_image_tests.rs"]
mod tests;
