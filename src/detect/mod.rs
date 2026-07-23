//! Grid detection: runs / tiled / elastic detectors returning ranked candidates.

use crate::Config;
use image::RgbaImage;

pub mod elastic;
pub mod runs;
pub mod tiled;

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
    let run_tiled = matches!(strategy, DetectStrategy::Auto | DetectStrategy::Tiled);
    if run_tiled {
        if let Some(c) = tiled::detect_tiled(img, config) {
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

/// Minimum confidence for a Runs/Tiled candidate to be trusted over the Elastic
/// fallback under Auto. Elastic's confidence is peak-strength-based and
/// saturates near 1.0 on anything with edges, so a bare confidence comparison
/// would let Elastic win everywhere. A Runs/Tiled hit must clear this bar to
/// count as a confident integer-scale detection; otherwise we fall back to
/// whichever candidate has the highest confidence (typically Elastic, which
/// correctly handles skew / non-integer grids).
const AUTO_STRONG_CONFIDENCE: f64 = 0.6;

/// Select the best candidate.
///
/// - **Manual strategy**: among that detector's candidates, pick by priority
///   then confidence.
/// - **Auto**: if any Runs/Tiled candidate clears `AUTO_STRONG_CONFIDENCE`,
///   pick the highest-priority confident one (Runs > Tiled). Otherwise fall
///   back to the highest-confidence candidate (usually Elastic).
///
/// Returns `(best, all_sorted_by_confidence)`.
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
    let best = if strategy == DetectStrategy::Auto {
        let has_strong = filtered.iter().any(|c| {
            matches!(c.detector, DetectStrategy::Runs | DetectStrategy::Tiled)
                && c.confidence >= AUTO_STRONG_CONFIDENCE
        });
        if has_strong {
            filtered
                .iter()
                .copied()
                .filter(|c| {
                    matches!(c.detector, DetectStrategy::Runs | DetectStrategy::Tiled)
                        && c.confidence >= AUTO_STRONG_CONFIDENCE
                })
                .min_by(|a, b| {
                    priority(a.detector)
                        .cmp(&priority(b.detector))
                        .then_with(|| {
                            b.confidence
                                .partial_cmp(&a.confidence)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                })
        } else {
            filtered.iter().copied().max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }
    } else {
        filtered.iter().copied().min_by(|a, b| {
            priority(a.detector)
                .cmp(&priority(b.detector))
                .then_with(|| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
    };

    let best = best?;
    let mut sorted = filtered.clone();
    sorted.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some((best, sorted))
}
