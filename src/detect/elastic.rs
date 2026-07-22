//! Elastic detector: wraps the existing profile-based step estimation.

use crate::detect::{CutMethod, DetectionCandidate, DetectStrategy};
use crate::Config;
use image::RgbaImage;

/// Detect via gradient profiles + median peak spacing + skew reconciliation.
/// `profile_x`/`profile_y` may be empty — this fn recomputes them from `img`.
pub fn detect_elastic(
    img: &RgbaImage,
    profile_x: &[f64],
    profile_y: &[f64],
    width: u32,
    height: u32,
    config: &Config,
) -> Option<DetectionCandidate> {
    let (px, py) = if profile_x.is_empty() || profile_y.is_empty() {
        let (p1, p2) = crate::profile::compute_profiles(img).ok()?;
        (p1, p2)
    } else {
        (profile_x.to_vec(), profile_y.to_vec())
    };

    let step_x_opt = crate::profile::estimate_step_size(&px, config);
    let step_y_opt = crate::profile::estimate_step_size(&py, config);
    if step_x_opt.is_none() && step_y_opt.is_none() {
        return None;
    }
    let (step_x, _step_y) =
        crate::profile::resolve_step_sizes(step_x_opt, step_y_opt, width, height, config);

    // confidence: peak strength ratio (max profile value mapped into 0..1)
    let max_x = px.iter().cloned().fold(0.0_f64, f64::max);
    let max_y = py.iter().cloned().fold(0.0_f64, f64::max);
    let max_val = max_x.max(max_y);
    let confidence = if max_val > 0.0 {
        (max_val / (max_val + 1.0)).min(1.0)
    } else {
        0.0
    };

    Some(DetectionCandidate {
        detector: DetectStrategy::Elastic,
        scale: None,
        step: step_x, // step_x == step_y after resolve
        confidence,
        cut_method: CutMethod::Walker,
    })
}
