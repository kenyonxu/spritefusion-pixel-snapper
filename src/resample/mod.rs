//! Grid-cell resampling strategies. See `ResampleMethod`.

mod majority;
mod median;
mod dominant;
mod mode;

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
        ResampleMethod::Median => median::resample_median(img, cols, rows, config),
        ResampleMethod::Dominant => dominant::resample_dominant(img, cols, rows, config),
        ResampleMethod::Mode => mode::resample_mode(img, cols, rows, config),
    }
}
