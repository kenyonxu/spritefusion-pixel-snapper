//! Grid detection: runs / tiled / elastic detectors returning ranked candidates.

use crate::Config;
use image::RgbaImage;

pub mod elastic;
pub mod runs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectStrategy {
    Auto,
    Runs,
    Tiled,
    Elastic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutMethod {
    Uniform,
    Walker,
}

#[derive(Debug, Clone)]
pub struct DetectionCandidate {
    pub detector: DetectStrategy, // never Auto
    pub scale: Option<usize>,
    pub step: f64,
    pub confidence: f64,
    pub cut_method: CutMethod,
}

/// Run detectors per `strategy`, return all candidates (Auto = all three).
/// Implemented across Tasks 2/3/4; stub returns empty for now.
pub fn detect(
    img: &RgbaImage,
    profile_x: &[f64],
    profile_y: &[f64],
    width: u32,
    height: u32,
    config: &Config,
    strategy: DetectStrategy,
) -> Vec<DetectionCandidate> {
    let mut out = Vec::new();
    let run_runs = matches!(strategy, DetectStrategy::Auto | DetectStrategy::Runs);
    if run_runs {
        if let Some(c) = runs::detect_runs(img, config) {
            out.push(c);
        }
    }
    let run_elastic = matches!(strategy, DetectStrategy::Auto | DetectStrategy::Elastic);
    if run_elastic {
        if let Some(c) = elastic::detect_elastic(img, profile_x, profile_y, width, height, config) {
            out.push(c);
        }
    }
    out
}

/// Select the best candidate: Auto sorts by priority Runs>Tiled>Elastic then
/// confidence desc; manual filters to that detector. Returns (best, all).
pub fn select_best(
    candidates: &[DetectionCandidate],
    strategy: DetectStrategy,
) -> Option<(&DetectionCandidate, Vec<&DetectionCandidate>)> {
    if candidates.is_empty() {
        return None;
    }
    let filtered: Vec<&DetectionCandidate> = match strategy {
        DetectStrategy::Auto => candidates.iter().collect(),
        specific => candidates.iter().filter(|c| c.detector == specific).collect(),
    };
    if filtered.is_empty() {
        return None;
    }
    let priority = |d: DetectStrategy| match d {
        DetectStrategy::Runs => 0,
        DetectStrategy::Tiled => 1,
        DetectStrategy::Elastic => 2,
        DetectStrategy::Auto => 3,
    };
    let mut sorted = filtered.clone();
    sorted.sort_by(|a, b| {
        priority(a.detector)
            .cmp(&priority(b.detector))
            .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
    });
    let best = sorted.first().copied();
    best.map(|b| (b, sorted))
}
