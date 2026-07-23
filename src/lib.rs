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
pub mod resample;
mod stabilize;
mod validate;

pub use config::Config;
pub use error::{PixelSnapperError, Result};

#[cfg(not(target_arch = "wasm32"))]
pub use cli::run_cli;

use image::GenericImageView;
use detect::{detect, select_best, CutMethod, DetectionCandidate, DetectStrategy};
use palette::{apply_palette, parse_palette_hex};
use profile::{compute_profiles, estimate_step_size, resolve_step_sizes};
use stabilize::{snap_uniform_cuts, walk, stabilize_both_axes};
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
    pub(crate) selected_detector: Option<crate::detect::DetectStrategy>,
    pub(crate) candidates: Vec<crate::detect::DetectionCandidate>,
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

    let analysis_img = quantize::quantize(&rgba_img, &config)?;
    let (profile_x, profile_y) = compute_profiles(&analysis_img)?;

    let candidates = detect(
        &rgba_img,
        &profile_x,
        &profile_y,
        width,
        height,
        &config,
        config.detect_strategy,
    );
    let chosen = select_best(&candidates, config.detect_strategy)
        .map(|(best, _)| best.clone())
        .unwrap_or_else(|| {
            // fallback: synthesize elastic-style candidate so existing fallback path runs
            let (sx, _sy) = resolve_step_sizes(
                estimate_step_size(&profile_x, &config),
                estimate_step_size(&profile_y, &config),
                width,
                height,
                &config,
            );
            DetectionCandidate {
                detector: DetectStrategy::Elastic,
                scale: None,
                step: sx,
                confidence: 0.0,
                cut_method: CutMethod::Walker,
            }
        });
    let selected_detector = Some(chosen.detector);

    let (col_cuts, row_cuts) = match chosen.cut_method {
        CutMethod::Uniform => {
            let scale = chosen.scale.expect("Uniform candidate must have scale");
            let target_step = scale as f64;
            let col = snap_uniform_cuts(
                &profile_x,
                width as usize,
                target_step,
                &config,
                config.min_cuts_per_axis,
            );
            let row = snap_uniform_cuts(
                &profile_y,
                height as usize,
                target_step,
                &config,
                config.min_cuts_per_axis,
            );
            (col, row)
        }
        CutMethod::Walker => {
            let step = chosen.step;
            let raw_col_cuts = walk(&profile_x, step, width as usize, &config)?;
            let raw_row_cuts = walk(&profile_y, step, height as usize, &config)?;
            stabilize_both_axes(
                &profile_x,
                &profile_y,
                raw_col_cuts,
                raw_row_cuts,
                width as usize,
                height as usize,
                &config,
            )
        }
    };

    let snapped_img = resample::resample(&analysis_img, &col_cuts, &row_cuts, &config)?;
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
        pixel_size: chosen.step,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: (col_cuts.len() - 1) as u32,
        output_height: (row_cuts.len() - 1) as u32,
        selected_detector,
        candidates,
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
    detect_strategy: Option<String>,
    resample_method: Option<String>,
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
    if let Some(s) = detect_strategy {
        config.detect_strategy = match s.as_str() {
            "auto" => detect::DetectStrategy::Auto,
            "runs" => detect::DetectStrategy::Runs,
            "tiled" => detect::DetectStrategy::Tiled,
            "elastic" => detect::DetectStrategy::Elastic,
            _ => {
                return Err(wasm_bindgen::JsValue::from_str(
                    "detect_strategy must be auto|runs|tiled|elastic",
                ))
            }
        };
    }

    if let Some(m) = resample_method {
        config.resample_method = match m.as_str() {
            "majority" => resample::ResampleMethod::Majority,
            "median" => resample::ResampleMethod::Median,
            "dominant" => resample::ResampleMethod::Dominant,
            "mode" => resample::ResampleMethod::Mode,
            _ => return Err(wasm_bindgen::JsValue::from_str(
                "resample_method must be majority|median|dominant|mode",
            )),
        };
    }

    process_image_common(input_bytes, Some(config))
        .map(|processed| processed.output_bytes)
        .map_err(wasm_bindgen::JsValue::from)
}

/// WASM: return candidate list as a JSON string (for Web candidate UI, U2.2).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn detect_candidates(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    detect_strategy: Option<String>,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(s) = detect_strategy {
        config.detect_strategy = match s.as_str() {
            "auto" => detect::DetectStrategy::Auto,
            "runs" => detect::DetectStrategy::Runs,
            "tiled" => detect::DetectStrategy::Tiled,
            "elastic" => detect::DetectStrategy::Elastic,
            _ => {
                return Err(wasm_bindgen::JsValue::from_str(
                    "detect_strategy must be auto|runs|tiled|elastic",
                ))
            }
        };
    }
    let _ = k_colors; // detection does not need k_colors
    let img = image::load_from_memory(input_bytes)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("{}", e)))?;
    let (w, h) = img.dimensions();
    crate::validate::validate_image_dimensions(w, h)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("{}", e)))?;
    let rgba = img.to_rgba8();
    let cands = detect::detect(&rgba, &[], &[], w, h, &config, config.detect_strategy);
    let json: Vec<String> = cands
        .iter()
        .map(|c| {
            format!(
                r#"{{"detector":"{:?}","scale":{:?},"step":{},"confidence":{:.3},"cut_method":"{:?}"}}"#,
                c.detector, c.scale, c.step, c.confidence, c.cut_method
            )
        })
        .collect();
    Ok(format!("[{}]", json.join(",")))
}
