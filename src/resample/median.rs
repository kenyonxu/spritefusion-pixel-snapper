//! Per-channel median resample with sample window; suppresses anti-aliasing.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};

pub(crate) fn resample_median(
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
    let window = config.resample_sample_window.max(1);
    let half = (window as i32) / 2;
    let (iw, ih) = (img.width() as i32, img.height() as i32);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let (ys, ye) = (w_y[0] as i32, w_y[1] as i32);
            let (xs, xe) = (w_x[0] as i32, w_x[1] as i32);
            if xe <= xs || ye <= ys {
                continue;
            }
            let cx = (xs + xe) / 2;
            let cy = (ys + ye) / 2;

            // Pass 1: opaque pixels in the window
            let mut chans: [Vec<u8>; 4] = Default::default();
            for dy in -half..=half {
                for dx in -half..=half {
                    let (x, y) = (cx + dx, cy + dy);
                    if x < xs || x >= xe || y < ys || y >= ye || x < 0 || y < 0 || x >= iw || y >= ih {
                        continue;
                    }
                    let p = img.get_pixel(x as u32, y as u32).0;
                    if p[3] < 16 {
                        continue;
                    }
                    for ch in 0..4 {
                        chans[ch].push(p[ch]);
                    }
                }
            }

            // Fallback: all pixels in the cell (incl. transparent)
            if chans[0].is_empty() {
                for y in ys..ye {
                    for x in xs..xe {
                        if x < 0 || y < 0 || x >= iw || y >= ih {
                            continue;
                        }
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
                let mut med = [0u8; 4];
                for ch in 0..4 {
                    chans[ch].sort_unstable();
                    med[ch] = chans[ch][chans[ch].len() / 2];
                }
                med
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
