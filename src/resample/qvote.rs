//! Qvote resample: per-cell Oklab k-means (k≈4), then vote on the dominant
//! cluster's centroid.
//!
//! For each grid cell: collect opaque pixels → convert to Oklab → k-means
//! (k = min(4, n), k-means++ init seeded from `config.seed` mixed with the
//! cell index → deterministic per R1) → the cluster with the most members
//! wins → its centroid is converted back to RGB and emitted (alpha 255).
//! Cells with no opaque pixels emit transparent black.

use crate::error::{PixelSnapperError, Result};
use crate::quantize::oklab;
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use rand::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

const MAX_CLUSTERS: usize = 4;
const MAX_ITERATIONS: usize = 16;
const CONVERGENCE_EPS_SQ: f32 = 0.0001;

fn dist_sq(p: &[f32; 3], c: &[f32; 3]) -> f32 {
    let dl = p[0] - c[0];
    let da = p[1] - c[1];
    let db = p[2] - c[2];
    dl * dl + da * da + db * db
}

/// Deterministic per-cell k-means over Oklab points. Returns the centroid of
/// the largest cluster (ties → lowest cluster index).
fn dominant_centroid(points: &[[f32; 3]], cell_index: usize, seed: u64) -> [f32; 3] {
    let n = points.len();
    debug_assert!(n > 0);
    if n == 1 {
        return points[0];
    }
    let k = MAX_CLUSTERS.min(n);

    // Per-cell deterministic seed: mix the global seed with the cell index so
    // cells don't all share one RNG stream, yet reruns are byte-identical.
    let mut rng = ChaCha8Rng::seed_from_u64(
        seed ^ (cell_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
    );

    // k-means++ init: first centroid uniform at random, the rest ∝ D².
    let mut centroids: Vec<[f32; 3]> = Vec::with_capacity(k);
    centroids.push(points[rng.gen_range(0..n)]);
    let mut distances = vec![f32::MAX; n];
    for _ in 1..k {
        let last = *centroids.last().unwrap();
        let mut sum_sq = 0.0f32;
        for (i, p) in points.iter().enumerate() {
            let d = dist_sq(p, &last);
            if d < distances[i] {
                distances[i] = d;
            }
            sum_sq += distances[i];
        }
        if sum_sq <= 0.0 {
            // All remaining points coincide with existing centroids — reuse a
            // deterministic point rather than sampling.
            centroids.push(points[0]);
        } else {
            // Deterministic weighted pick: one uniform draw against the CDF.
            let mut target = rng.gen::<f32>() * sum_sq;
            let mut idx = n - 1;
            for (i, d) in distances.iter().enumerate() {
                target -= d;
                if target <= 0.0 {
                    idx = i;
                    break;
                }
            }
            centroids.push(points[idx]);
        }
    }

    let mut assignments = vec![0usize; n];
    for _ in 0..MAX_ITERATIONS {
        // Assign each point to its nearest centroid.
        for (i, p) in points.iter().enumerate() {
            let mut best = 0;
            let mut best_d = f32::MAX;
            for (ci, c) in centroids.iter().enumerate() {
                let d = dist_sq(p, c);
                if d < best_d {
                    best_d = d;
                    best = ci;
                }
            }
            assignments[i] = best;
        }
        // Recompute centroids; track max movement for convergence.
        let mut sums = vec![[0.0f32; 3]; k];
        let mut counts = vec![0usize; k];
        for (p, &a) in points.iter().zip(assignments.iter()) {
            sums[a][0] += p[0];
            sums[a][1] += p[1];
            sums[a][2] += p[2];
            counts[a] += 1;
        }
        let mut max_move = 0.0f32;
        for ci in 0..k {
            if counts[ci] > 0 {
                let new_c = [
                    sums[ci][0] / counts[ci] as f32,
                    sums[ci][1] / counts[ci] as f32,
                    sums[ci][2] / counts[ci] as f32,
                ];
                max_move = max_move.max(dist_sq(&new_c, &centroids[ci]));
                centroids[ci] = new_c;
            }
        }
        if max_move < CONVERGENCE_EPS_SQ {
            break;
        }
    }

    // Vote: largest cluster wins; tie → lowest index (deterministic).
    let mut counts = vec![0usize; k];
    for &a in &assignments {
        counts[a] += 1;
    }
    let winner = counts
        .iter()
        .enumerate()
        .max_by(|(ia, ca), (ib, cb)| ca.cmp(cb).then(ib.cmp(ia)))
        .map(|(i, _)| i)
        .unwrap_or(0);
    centroids[winner]
}

pub(crate) fn resample_qvote(
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
    let (iw, ih) = (img.width() as usize, img.height() as usize);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let (ys, ye) = (w_y[0], w_y[1]);
            let (xs, xe) = (w_x[0], w_x[1]);
            if xe <= xs || ye <= ys {
                continue;
            }
            let mut points: Vec<[f32; 3]> = Vec::new();
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        if p[3] > 0 {
                            points.push(oklab::rgb_to_oklab(p[0], p[1], p[2]));
                        }
                    }
                }
            }
            let pixel = if points.is_empty() {
                [0, 0, 0, 0]
            } else {
                let cell_index = y_i * (cols.len() - 1) + x_i;
                let c = dominant_centroid(&points, cell_index, config.seed);
                let rgb = oklab::oklab_to_rgb(c[0], c[1], c[2]);
                [rgb[0], rgb[1], rgb[2], 255]
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
