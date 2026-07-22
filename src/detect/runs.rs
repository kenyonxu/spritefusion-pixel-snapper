//! Runs detector: GCD of same-color run lengths (integer scale), with posterize
//! preprocessing to suppress single-pixel noise that would collapse the GCD.

use crate::detect::{CutMethod, DetectionCandidate, DetectStrategy};
use crate::Config;
use image::{Rgba, RgbaImage};

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Quantize each channel to step-sized buckets (posterize). `step=4` ~ 64 levels.
fn posterize(img: &RgbaImage, step: u8) -> RgbaImage {
    let mut out = RgbaImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] == 0 {
            out.put_pixel(x, y, *p);
            continue;
        }
        let q = |c: u8| (c / step) * step;
        out.put_pixel(x, y, Rgba([q(p[0]), q(p[1]), q(p[2]), p[3]]));
    }
    out
}

fn pixel_key(img: &RgbaImage, x: u32, y: u32) -> u32 {
    let p = img.get_pixel(x, y);
    ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32)
}

pub fn detect_runs(img: &RgbaImage, config: &Config) -> Option<DetectionCandidate> {
    let posterized = posterize(img, 4);
    let (w, h) = img.dimensions();
    let mut runs: Vec<usize> = Vec::new();

    // horizontal runs
    for y in 0..h {
        let mut prev = pixel_key(&posterized, 0, y);
        let mut len = 1;
        for x in 1..w {
            let cur = pixel_key(&posterized, x, y);
            if cur == prev {
                len += 1;
            } else {
                runs.push(len);
                len = 1;
                prev = cur;
            }
        }
        runs.push(len);
    }
    // vertical runs
    for x in 0..w {
        let mut prev = pixel_key(&posterized, x, 0);
        let mut len = 1;
        for y in 1..h {
            let cur = pixel_key(&posterized, x, y);
            if cur == prev {
                len += 1;
            } else {
                runs.push(len);
                len = 1;
                prev = cur;
            }
        }
        runs.push(len);
    }

    if (runs.len() as usize) < config.runs_min_runs {
        return None;
    }

    let scale = runs.iter().copied().fold(0usize, gcd);
    if scale < 2 {
        return None;
    }

    // confidence: fraction of runs that are multiples of scale
    let matching = runs.iter().filter(|r| **r % scale == 0).count();
    let confidence = (matching as f64 / runs.len() as f64).min(1.0);

    Some(DetectionCandidate {
        detector: DetectStrategy::Runs,
        scale: Some(scale),
        step: scale as f64,
        confidence,
        cut_method: CutMethod::Uniform,
    })
}
