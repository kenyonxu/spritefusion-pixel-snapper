//! Pixel Snapper: detect an image's implicit pixel grid and re-snap to it.
//!
//! Dual-target crate: native CLI (via [`run_cli`]) and WASM (via
//! [`process_image`]). The shared pipeline lives in [`process_image_common`].

mod cli;
mod config;
pub mod detect;
mod error;
mod palette;
mod profile;
mod quantize;
mod resample;
mod stabilize;
mod validate;

pub use config::Config;
pub use error::{PixelSnapperError, Result};

#[cfg(not(target_arch = "wasm32"))]
pub use cli::run_cli;

use image::GenericImageView;
use palette::{apply_palette, parse_palette_hex};
use profile::{compute_profiles, estimate_step_size, resolve_step_sizes};
use quantize::quantize_image;
use resample::resample;
use stabilize::{walk, stabilize_both_axes};
use validate::validate_image_dimensions;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) struct ProcessedImage {
    pub(crate) output_bytes: Vec<u8>,
    pub(crate) pixel_size: f64,
    pub(crate) pixel_size_override: bool,
    pub(crate) output_width: u32,
    pub(crate) output_height: u32,
}

/// Shared pipeline entry point for both the CLI and WASM targets.
pub(crate) fn process_image_common(input_bytes: &[u8], config: Option<Config>) -> Result<ProcessedImage> {
    let config = config.unwrap_or_default();

    let img = image::load_from_memory(input_bytes)?;
    let (width, height) = img.dimensions();

    validate_image_dimensions(width, height)?;

    if let Some(px) = config.pixel_size_override {
        if !px.is_finite() || px < 1.0 || px > (width.min(height) as f64 / 2.0) {
            return Err(PixelSnapperError::InvalidInput(format!(
                "pixel_size_override {:.1} is out of valid range [1, {}]",
                px,
                width.min(height) / 2
            )));
        }
    }

    let rgba_img = img.to_rgba8();

    let analysis_img = quantize_image(&rgba_img, &config)?;
    let (profile_x, profile_y) = compute_profiles(&analysis_img)?;

    // Estimate step sizes
    let step_x_opt = estimate_step_size(&profile_x, &config);
    let step_y_opt = estimate_step_size(&profile_y, &config);

    // Resolve step sizes. Some instabilities so use sibling axis if one fails, or fallback if both fail
    let (step_x, step_y) = resolve_step_sizes(step_x_opt, step_y_opt, width, height, &config);

    let raw_col_cuts = walk(&profile_x, step_x, width as usize, &config)?;
    let raw_row_cuts = walk(&profile_y, step_y, height as usize, &config)?;

    // Two-pass stabilization: first pass with raw cuts, then cross-validate
    let (col_cuts, row_cuts) = stabilize_both_axes(
        &profile_x,
        &profile_y,
        raw_col_cuts,
        raw_row_cuts,
        width as usize,
        height as usize,
        &config,
    );

    let snapped_img = resample(&analysis_img, &col_cuts, &row_cuts)?;
    let output_img = match config.palette.as_deref() {
        Some(palette) => apply_palette(&snapped_img, palette)?,
        None => snapped_img,
    };

    // Returns bytes for both implementations
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    output_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(PixelSnapperError::ImageError)?;

    Ok(ProcessedImage {
        output_bytes,
        pixel_size: step_x,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: (col_cuts.len() - 1) as u32,
        output_height: (row_cuts.len() - 1) as u32,
    })
}

/// WASM entry point
/// `palette_hex` is a comma-separated list of hex colors: `"0d2b45,ffecd6"`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    palette_hex: Option<String>,
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(k) = k_colors {
        if k == 0 {
            return Err(wasm_bindgen::JsValue::from_str(
                "k_colors must be greater than 0",
            ));
        }
        config.k_colors = k as usize;
    }

    config.pixel_size_override = pixel_size_override;
    config.palette = palette_hex
        .as_deref()
        .map(parse_palette_hex)
        .transpose()
        .map_err(wasm_bindgen::JsValue::from)?;

    process_image_common(input_bytes, Some(config))
        .map(|processed| processed.output_bytes)
        .map_err(wasm_bindgen::JsValue::from)
}
