//! Pipeline configuration.
//!
//! `Config` is constructed by both the CLI (`run_cli`) and the WASM entry
//! (`process_image`). Internal tuning fields are `pub(crate)` so every pipeline
//! module can read them; `k_colors` / `pixel_size_override` are fully `pub` for
//! the wasm_bindgen export consumed by JS.

use crate::detect::DetectStrategy;

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Config {
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub(crate) palette: Option<Vec<[u8; 3]>>,
    pub(crate) seed: u64,
    /// Input image path only used for CLI use
    #[allow(dead_code)]
    pub(crate) input_path: String,
    /// Output image path only used for CLI use
    #[allow(dead_code)]
    pub(crate) output_path: String,
    pub(crate) max_kmeans_iterations: usize,
    pub(crate) peak_threshold_multiplier: f64,
    pub(crate) peak_distance_filter: usize,
    pub(crate) walker_search_window_ratio: f64,
    pub(crate) walker_min_search_window: f64,
    pub(crate) walker_strength_threshold: f64,
    pub(crate) min_cuts_per_axis: usize,
    pub(crate) fallback_target_segments: usize,
    pub(crate) max_step_ratio: f64,
    pub(crate) detect_strategy: DetectStrategy,
    pub(crate) runs_min_runs: usize,
    pub(crate) tiled_stddev_threshold: f64,
    pub(crate) tiled_peak_ratio: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            k_colors: 16,
            seed: 42,
            input_path: "samples/2/skeleton.png".to_string(),
            output_path: "samples/2/skeleton_fixed_clean2.png".to_string(),
            max_kmeans_iterations: 15,
            peak_threshold_multiplier: 0.2,
            peak_distance_filter: 4,
            walker_search_window_ratio: 0.35,
            walker_min_search_window: 2.0,
            walker_strength_threshold: 0.5,
            min_cuts_per_axis: 4,
            fallback_target_segments: 64,
            max_step_ratio: 1.8, // Lowered from 3.0 to catch more skew cases
            detect_strategy: DetectStrategy::Auto,
            runs_min_runs: 10,
            tiled_stddev_threshold: 5.0,
            tiled_peak_ratio: 0.6,
            pixel_size_override: None,
            palette: None,
        }
    }
}
