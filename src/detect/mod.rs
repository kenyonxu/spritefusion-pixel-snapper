//! Grid detection: runs / tiled / elastic detectors returning ranked candidates.

use crate::Config;
use image::RgbaImage;

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
    _img: &RgbaImage,
    _profile_x: &[f64],
    _profile_y: &[f64],
    _width: u32,
    _height: u32,
    _config: &Config,
    _strategy: DetectStrategy,
) -> Vec<DetectionCandidate> {
    Vec::new()
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
