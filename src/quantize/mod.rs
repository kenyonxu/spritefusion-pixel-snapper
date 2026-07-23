//! Color quantization: k-means (RGB or Oklab), dithering, palettes.

mod kmeans;
pub(crate) mod oklab;

use crate::error::Result;
use crate::Config;
use image::RgbaImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colorspace {
    Rgb,
    Oklab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMethod {
    None,
    FloydSteinberg,
    Bayer2,
    Bayer4,
    Bayer8,
    Ordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetPalette {
    None,
    Nes,
    GameBoy,
    Sgb,
    Snes,
    Pc9801,
    Msx1,
    Pico8,
    Sweetie16,
    Endesga32,
}

pub fn quantize(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    kmeans::quantize_kmeans(img, config)
}
