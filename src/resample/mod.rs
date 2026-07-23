//! Grid-cell resampling strategies. See `ResampleMethod`.

mod majority;

use crate::error::Result;
use crate::Config;
use image::RgbaImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    Majority,
    Median,
    Dominant,
    Mode,
}

pub fn resample(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
) -> Result<RgbaImage> {
    // Task 2 wires config.resample_method; for now hardcode Majority to keep
    // this task a pure move.
    let _ = config;
    majority::resample_majority(img, cols, rows, config)
}
