//! Color quantization: k-means (RGB or Oklab), dithering, palettes.

mod kmeans;
mod dither;
mod palettes;
pub(crate) mod oklab;

use crate::error::Result;
use crate::palette::apply_palette;
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
    let mut img = img.clone();
    dither::apply(&mut img, config.quantize_dither, config.quantize_dither_strength);
    let mut out = kmeans::quantize_kmeans(&img, config)?;
    // Preset palette snap runs only when a non-None preset is selected. The
    // custom `--palette` (Config.palette) is applied later in
    // `process_image_common` and therefore wins on precedence — see CLAUDE.md.
    if let Some(pal) = palettes::palette(config.quantize_preset_palette) {
        out = apply_palette(&out, pal)?;
    }
    Ok(out)
}
