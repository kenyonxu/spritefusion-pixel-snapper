//! Per-channel mode resample. CAVEAT: the combined pixel may be a color that
//! did not exist in the source (R-mode + G-mode + B-mode). Use `majority` for
//! strict palette preservation.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;

pub(crate) fn resample_mode(
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

    let channel_mode = |vals: &[u8]| -> u8 {
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for &v in vals {
            *counts.entry(v).or_insert(0) += 1;
        }
        // highest count, tie → lowest value (deterministic)
        counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))
            .map(|(v, _)| *v)
            .unwrap_or(0)
    };

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];
            if xe <= xs || ye <= ys {
                continue;
            }

            let mut chans: [Vec<u8>; 4] = Default::default();
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        for ch in 0..4 {
                            chans[ch].push(p[ch]);
                        }
                    }
                }
            }

            let pixel = if chans[0].is_empty() {
                [0, 0, 0, 0]
            } else {
                [
                    channel_mode(&chans[0]),
                    channel_mode(&chans[1]),
                    channel_mode(&chans[2]),
                    channel_mode(&chans[3]),
                ]
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
