//! Dithering: Floyd-Steinberg error diffusion + Bayer threshold matrices + Ordered.
//!
//! All methods are RNG-free → deterministic (R1 holds). Applied to the analysis
//! image before k-means (same approach as PixelRefiner). Caveat: because this
//! runs on the analysis image, dithering also perturbs what detect/resample see
//! downstream when enabled. Default is None so the anchor is unaffected; if
//! dither+Auto-detect misbehaves in practice, move apply() to post-resample.

use image::{Rgba, RgbaImage};

fn bayer_matrix(size: usize) -> Vec<Vec<f32>> {
    let raw = match size {
        2 => vec![vec![0.0, 2.0], vec![3.0, 1.0]],
        4 => vec![
            vec![0.0, 8.0, 2.0, 10.0],
            vec![12.0, 4.0, 14.0, 6.0],
            vec![3.0, 11.0, 1.0, 9.0],
            vec![15.0, 7.0, 13.0, 5.0],
        ],
        _ => {
            // 8x8 standard Bayer (recursive from 4x4)
            let b4 = bayer_matrix(4);
            let mut m = vec![vec![0.0; 8]; 8];
            for y in 0..8 {
                for x in 0..8 {
                    let bx = (x % 2) + 2 * (x / 2);
                    let by = (y % 2) + 2 * (y / 2);
                    m[y][x] = 4.0 * b4[by / 2][bx / 2] + b4[y % 2][x % 2];
                }
            }
            m
        }
    };
    raw.into_iter()
        .map(|row| row.into_iter().map(|v| v / (size * size) as f32).collect())
        .collect()
}

fn apply_threshold(img: &mut RgbaImage, strength: f64, matrix: Vec<Vec<f32>>) {
    let n = matrix.len();
    let bias = (strength * 255.0) as f32;
    for y in 0..img.height() {
        for x in 0..img.width() {
            let mut p = img.get_pixel(x, y).0;
            if p[3] == 0 {
                continue;
            }
            let t = matrix[(y as usize) % n][(x as usize) % n] - 0.5;
            for ch in 0..3 {
                p[ch] = ((p[ch] as f32 + t * bias).round().clamp(0.0, 255.0)) as u8;
            }
            img.put_pixel(x, y, Rgba(p));
        }
    }
}

pub fn floyd_steinberg(img: &mut RgbaImage, strength: f64) {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mut buf: Vec<[f32; 4]> = img
        .pixels()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32])
        .collect();
    let idx = |x: usize, y: usize| y * w + x;
    for y in 0..h {
        for x in 0..w {
            if buf[idx(x, y)][3] < 1.0 {
                continue;
            }
            let old = buf[idx(x, y)];
            let new = [
                old[0].round().clamp(0.0, 255.0),
                old[1].round().clamp(0.0, 255.0),
                old[2].round().clamp(0.0, 255.0),
                old[3],
            ];
            let s = strength as f32;
            let err = [(old[0] - new[0]) * s, (old[1] - new[1]) * s, (old[2] - new[2]) * s];
            buf[idx(x, y)] = new;
            let diffs: [(isize, isize, f32); 4] = [
                (1, 0, 7.0 / 16.0),
                (-1, 1, 3.0 / 16.0),
                (0, 1, 5.0 / 16.0),
                (1, 1, 1.0 / 16.0),
            ];
            for (dx, dy, w_) in diffs {
                let nx = (x as isize + dx) as usize;
                let ny = (y as isize + dy) as usize;
                if nx < w && ny < h && buf[idx(nx, ny)][3] >= 1.0 {
                    for ch in 0..3 {
                        buf[idx(nx, ny)][ch] += err[ch] * w_;
                    }
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let v = buf[idx(x, y)];
            img.put_pixel(
                x as u32,
                y as u32,
                Rgba([v[0] as u8, v[1] as u8, v[2] as u8, v[3] as u8]),
            );
        }
    }
}

pub fn apply(img: &mut RgbaImage, method: crate::quantize::DitherMethod, strength: f64) {
    use crate::quantize::DitherMethod::*;
    match method {
        None => {}
        FloydSteinberg => floyd_steinberg(img, strength),
        Bayer2 => apply_threshold(img, strength, bayer_matrix(2)),
        Bayer4 => apply_threshold(img, strength, bayer_matrix(4)),
        Bayer8 => apply_threshold(img, strength, bayer_matrix(8)),
        Ordered => apply_threshold(img, strength, bayer_matrix(4)),
    }
}
