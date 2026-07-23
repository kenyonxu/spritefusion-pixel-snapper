//! Qvote resample (lite). Whole-pixel vote with deterministic tie-break.
//!
//! NOTE: this is currently equivalent in spirit to `majority`. The full spec
//! qvote — per-cell Oklab k-means (k≈4), then vote on the dominant cluster —
//! is deferred. This variant exists so `ResampleMethod` is complete and
//! `--resample qvote` works; upgrade to per-cell Oklab clustering later.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;

pub(crate) fn resample_qvote(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    _config: &Config,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);
    let (iw, ih) = (img.width() as usize, img.height() as usize);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let (ys, ye) = (w_y[0], w_y[1]);
            let (xs, xe) = (w_x[0], w_x[1]);
            if xe <= xs || ye <= ys {
                continue;
            }
            let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        *counts.entry(p).or_insert(0) += 1;
                    }
                }
            }
            // deterministic: highest count, tie → lowest pixel value
            let pixel = counts
                .into_iter()
                .max_by(|(pa, ca), (pb, cb)| ca.cmp(cb).then(pb.cmp(pa)))
                .map(|(p, _)| p)
                .unwrap_or([0, 0, 0, 0]);
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
