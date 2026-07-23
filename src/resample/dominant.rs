//! Dominant-color resample: top color if it clears a threshold, else per-channel
//! mean of opaque pixels. Optional hard alpha binarization.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;

pub(crate) fn resample_dominant(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);
    let threshold = config.resample_dominant_threshold;
    let binarize = config.resample_dominant_binarize_alpha;
    let (iw, ih) = (img.width() as usize, img.height() as usize);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];
            if xe <= xs || ye <= ys {
                continue;
            }

            let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
            let mut total = 0usize;
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        *counts.entry(p).or_insert(0) += 1;
                        total += 1;
                    }
                }
            }

            let pixel = if total == 0 {
                [0, 0, 0, 0]
            } else {
                // top color
                let (top_color, top_count) = counts
                    .iter()
                    .max_by(|a, b| a.1.cmp(b.1))
                    .map(|(c, n)| (*c, *n))
                    .unwrap_or(([0, 0, 0, 0], 0));
                let chosen = if (top_count as f64 / total as f64) >= threshold {
                    top_color
                } else {
                    // mean of opaque pixels (per channel)
                    let mut sums = [0u64; 4];
                    let mut n = 0u64;
                    for y in ys..ye {
                        for x in xs..xe {
                            if x < iw && y < ih {
                                let p = img.get_pixel(x as u32, y as u32).0;
                                if p[3] >= 16 {
                                    for ch in 0..4 {
                                        sums[ch] += p[ch] as u64;
                                    }
                                    n += 1;
                                }
                            }
                        }
                    }
                    if n == 0 {
                        top_color
                    } else {
                        [
                            (sums[0] / n) as u8,
                            (sums[1] / n) as u8,
                            (sums[2] / n) as u8,
                            (sums[3] / n) as u8,
                        ]
                    }
                };
                let mut out = chosen;
                if binarize {
                    out[3] = if out[3] >= 128 { 255 } else { 0 };
                }
                out
            };

            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
