//! Tiled detector: 3x3 overlapping tiles, Sobel edge profile per tile,
//! autocorrelation peak-lag -> per-tile scale, mode vote.

use crate::detect::{CutMethod, DetectionCandidate, DetectStrategy};
use crate::Config;
use image::RgbaImage;
use std::collections::HashMap;

fn gray(img: &RgbaImage, x: u32, y: u32) -> f64 {
    let p = img.get_pixel(x, y);
    if p[3] == 0 {
        0.0
    } else {
        0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64
    }
}

fn stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    var.sqrt()
}

/// Autocorrelation peak lag: the lag in 2..=max_lag with highest sum of
/// v[i]*v[i+lag]. Returns the lag whose peak exceeds ratio*gmax, else None.
fn peak_lag(profile: &[f64], max_lag: usize, ratio: f64) -> Option<usize> {
    if profile.len() < 4 {
        return None;
    }
    let max_lag = max_lag.min(profile.len() / 2).min(128).max(1);
    let mut best_lag = 0usize;
    let mut best_score = 0.0f64;
    let gmax = profile.iter().cloned().fold(0.0f64, f64::max);
    let threshold = gmax * ratio;
    for lag in 2..=max_lag {
        let mut score = 0.0f64;
        let mut n = 0usize;
        for i in 0..profile.len().saturating_sub(lag) {
            score += profile[i] * profile[i + lag];
            n += 1;
        }
        if n > 0 {
            score /= n as f64;
        }
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }
    if best_score >= threshold && best_lag >= 2 {
        Some(best_lag)
    } else {
        None
    }
}

pub fn detect_tiled(img: &RgbaImage, config: &Config) -> Option<DetectionCandidate> {
    let (w, h) = img.dimensions();
    if w < 9 || h < 9 {
        return None;
    }
    let tile_w = w / 3;
    let tile_h = h / 3;
    let overlap_w = tile_w / 4;
    let overlap_h = tile_h / 4;
    if tile_w < 4 || tile_h < 4 {
        return None;
    }

    let mut votes: HashMap<usize, usize> = HashMap::new();

    let max_lag = ((tile_w.min(tile_h) / 8).max(8)) as usize;

    for ty in 0u32..3 {
        for tx in 0u32..3 {
            let x0 = tx.saturating_mul(tile_w).saturating_sub(if tx > 0 { overlap_w } else { 0 });
            let y0 = ty.saturating_mul(tile_h).saturating_sub(if ty > 0 { overlap_h } else { 0 });
            let x1 = ((tx + 1).min(3)).saturating_mul(tile_w).min(w);
            let y1 = ((ty + 1).min(3)).saturating_mul(tile_h).min(h);
            if x1 <= x0 + 2 || y1 <= y0 + 2 {
                continue;
            }
            // grays + stddev filter
            let mut grays = Vec::new();
            for y in y0..y1 {
                for x in x0..x1 {
                    grays.push(gray(img, x, y));
                }
            }
            if stddev(&grays) < config.tiled_stddev_threshold {
                continue;
            }
            // Sobel edge profile along x
            let mut profile = vec![0.0f64; (x1 - x0) as usize];
            for y in (y0 + 1)..y1.saturating_sub(1) {
                for x in (x0 + 1)..x1.saturating_sub(1) {
                    let gx = -gray(img, x - 1, y - 1) + gray(img, x + 1, y - 1)
                        - 2.0 * gray(img, x - 1, y)
                        + 2.0 * gray(img, x + 1, y)
                        - gray(img, x - 1, y + 1)
                        + gray(img, x + 1, y + 1);
                    profile[(x - x0) as usize] += gx.abs();
                }
            }
            if let Some(lag) = peak_lag(&profile, max_lag, config.tiled_peak_ratio) {
                *votes.entry(lag).or_insert(0) += 1;
            }
        }
    }

    if votes.is_empty() {
        return None;
    }
    let (scale, count) = votes.into_iter().max_by_key(|&(_, c)| c).unwrap();
    if scale < 2 {
        return None;
    }
    let confidence = (count as f64 / 9.0).min(1.0);

    Some(DetectionCandidate {
        detector: DetectStrategy::Tiled,
        scale: Some(scale),
        step: scale as f64,
        confidence,
        cut_method: CutMethod::Uniform,
    })
}
