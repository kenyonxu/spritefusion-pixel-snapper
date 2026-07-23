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
    match config.resample_method {
        ResampleMethod::Majority => majority::resample_majority(img, cols, rows, config),
        // wired in Tasks 3/4/5
        _ => majority::resample_majority(img, cols, rows, config),
    }
}
