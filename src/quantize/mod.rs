//! Color quantization: k-means (RGB now, Oklab in Task 4), dithering, palettes.

mod kmeans;

use crate::error::Result;
use crate::Config;
use image::RgbaImage;

pub fn quantize(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    kmeans::quantize_kmeans(img, config)
}
