mod error;
mod palette;
mod profile;
mod stabilize;
mod validate;

pub use error::{PixelSnapperError, Result};
use palette::{apply_palette, parse_palette_hex};
use profile::{compute_profiles, estimate_step_size, resolve_step_sizes};
use stabilize::{walk, stabilize_both_axes};
use validate::validate_image_dimensions;

use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage};
use rand::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, WeightedIndex};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Config {
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    palette: Option<Vec<[u8; 3]>>,
    k_seed: u64,
    /// Input image path only used for CLI use
    #[allow(dead_code)]
    input_path: String,
    /// Output image path only used for CLI use
    #[allow(dead_code)]
    output_path: String,
    max_kmeans_iterations: usize,
    peak_threshold_multiplier: f64,
    peak_distance_filter: usize,
    walker_search_window_ratio: f64,
    walker_min_search_window: f64,
    walker_strength_threshold: f64,
    min_cuts_per_axis: usize,
    fallback_target_segments: usize,
    max_step_ratio: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            k_colors: 16,
            k_seed: 42,
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
            pixel_size_override: None,
            palette: None,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct ProcessedImage {
    output_bytes: Vec<u8>,
    pixel_size: f64,
    pixel_size_override: bool,
    output_width: u32,
    output_height: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub palette: Option<Vec<[u8; 3]>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&Config> for BatchConfig {
    fn from(config: &Config) -> Self {
        Self {
            input_dir: PathBuf::from(&config.input_path),
            output_dir: PathBuf::from(&config.output_path),
            k_colors: config.k_colors,
            pixel_size_override: config.pixel_size_override,
            palette: config.palette.clone(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&BatchConfig> for Config {
    fn from(config: &BatchConfig) -> Self {
        Self {
            k_colors: config.k_colors,
            pixel_size_override: config.pixel_size_override,
            palette: config.palette.clone(),
            ..Default::default()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub enum BatchEvent {
    BatchStarted {
        input_dir: PathBuf,
        total: usize,
    },
    Started {
        input: PathBuf,
        index: usize,
        total: usize,
    },
    Finished {
        input: PathBuf,
        output: PathBuf,
        index: usize,
        total: usize,
    },
    Failed {
        input: PathBuf,
        output: PathBuf,
        error: String,
        index: usize,
        total: usize,
    },
    BatchFinished {
        input_dir: PathBuf,
        total: usize,
    },
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
enum CliCommand {
    Run(Config),
    Help,
    Version,
}

/// Internal entry point used by the packaged CLI binary.
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub fn run_cli() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    match parse_cli_args(&args) {
        Ok(CliCommand::Help) => {
            print_cli_help();
            std::process::ExitCode::SUCCESS
        }
        Ok(CliCommand::Version) => {
            println!("spritefusion-pixel-snapper {}", env!("CARGO_PKG_VERSION"));
            std::process::ExitCode::SUCCESS
        }
        Ok(CliCommand::Run(config)) => match process(&config) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Error: {error}");
                std::process::ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Run 'spritefusion-pixel-snapper --help' for usage.");
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn print_cli_help() {
    println!(
        concat!(
            "Sprite Fusion Pixel Snapper {version}\n",
            "Fix inconsistent pixel art by detecting and snapping it to its implicit grid.\n\n",
            "USAGE:\n",
            "  spritefusion-pixel-snapper <INPUT> <OUTPUT> [COLORS] [OPTIONS]\n\n",
            "ARGUMENTS:\n",
            "  <INPUT>   Input PNG/JPEG file, or a directory for batch processing\n",
            "  <OUTPUT>  Output PNG file, or a different output directory for a batch\n",
            "  [COLORS]  Number of palette colors [default: 16]\n\n",
            "OPTIONS:\n",
            "  --pixel-size <PIXELS>  Override the auto-detected pixel size\n",
            "  --palette <HEX,...>    Use comma-separated 6-digit hex palette colors\n",
            "  -h, --help             Print help\n",
            "  -V, --version          Print version\n\n",
            "EXAMPLES:\n",
            "  spritefusion-pixel-snapper input.png output.png\n",
            "  spritefusion-pixel-snapper input.png output.png 16 --pixel-size 8\n",
            "  spritefusion-pixel-snapper inputs outputs --palette 0d2b45,ffecd6"
        ),
        version = env!("CARGO_PKG_VERSION")
    );
}

fn process_image_common(input_bytes: &[u8], config: Option<Config>) -> Result<ProcessedImage> {
    let config = config.unwrap_or_default();

    let img = image::load_from_memory(input_bytes)?;
    let (width, height) = img.dimensions();

    validate_image_dimensions(width, height)?;

    if let Some(px) = config.pixel_size_override {
        if !px.is_finite() || px < 1.0 || px > (width.min(height) as f64 / 2.0) {
            return Err(PixelSnapperError::InvalidInput(format!(
                "pixel_size_override {:.1} is out of valid range [1, {}]",
                px,
                width.min(height) / 2
            )));
        }
    }

    let rgba_img = img.to_rgba8();

    let analysis_img = quantize_image(&rgba_img, &config)?;
    let (profile_x, profile_y) = compute_profiles(&analysis_img)?;

    // Estimate step sizes
    let step_x_opt = estimate_step_size(&profile_x, &config);
    let step_y_opt = estimate_step_size(&profile_y, &config);

    // Resolve step sizes. Some instabilities so use sibling axis if one fails, or fallback if both fail
    let (step_x, step_y) = resolve_step_sizes(step_x_opt, step_y_opt, width, height, &config);

    let raw_col_cuts = walk(&profile_x, step_x, width as usize, &config)?;
    let raw_row_cuts = walk(&profile_y, step_y, height as usize, &config)?;

    // Two-pass stabilization: first pass with raw cuts, then cross-validate
    let (col_cuts, row_cuts) = stabilize_both_axes(
        &profile_x,
        &profile_y,
        raw_col_cuts,
        raw_row_cuts,
        width as usize,
        height as usize,
        &config,
    );

    let snapped_img = resample(&analysis_img, &col_cuts, &row_cuts)?;
    let output_img = match config.palette.as_deref() {
        Some(palette) => apply_palette(&snapped_img, palette)?,
        None => snapped_img,
    };

    // Returns bytes for both implementations
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    output_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(PixelSnapperError::ImageError)?;

    Ok(ProcessedImage {
        output_bytes,
        pixel_size: step_x,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: (col_cuts.len() - 1) as u32,
        output_height: (row_cuts.len() - 1) as u32,
    })
}

/// WASM entry point
/// `palette_hex` is a comma-separated list of hex colors: `"0d2b45,ffecd6"`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    palette_hex: Option<String>,
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(k) = k_colors {
        if k == 0 {
            return Err(wasm_bindgen::JsValue::from_str(
                "k_colors must be greater than 0",
            ));
        }
        config.k_colors = k as usize;
    }

    config.pixel_size_override = pixel_size_override;
    config.palette = palette_hex
        .as_deref()
        .map(parse_palette_hex)
        .transpose()
        .map_err(wasm_bindgen::JsValue::from)?;

    process_image_common(input_bytes, Some(config))
        .map(|processed| processed.output_bytes)
        .map_err(wasm_bindgen::JsValue::from)
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_cli_args(args: &[String]) -> Result<CliCommand> {
    if args.is_empty()
        || args
            .iter()
            .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        return Ok(CliCommand::Help);
    }
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-V" | "--version"))
    {
        return Ok(CliCommand::Version);
    }
    if args.len() < 2 {
        return Err(PixelSnapperError::InvalidInput(
            "missing output path".to_string(),
        ));
    }

    let mut config = Config {
        input_path: args[0].clone(),
        output_path: args[1].clone(),
        ..Default::default()
    };

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--pixel-size" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--pixel-size requires a value".to_string(),
                    ));
                };

                match val.parse::<f64>() {
                    Ok(px) if px.is_finite() && px > 0.0 => config.pixel_size_override = Some(px),
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --pixel-size '{}': expected a positive number",
                            val
                        )))
                    }
                }
                i += 2;
            }
            "--palette" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--palette requires a value".to_string(),
                    ));
                };

                config.palette = Some(parse_palette_hex(val)?);
                i += 2;
            }
            arg if arg.starts_with("--") => {
                return Err(PixelSnapperError::InvalidInput(format!(
                    "unknown argument '{}'",
                    arg
                )));
            }
            k_arg => {
                match k_arg.parse::<usize>() {
                    Ok(k) if k > 0 => config.k_colors = k,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid color count '{}': expected a positive integer",
                            k_arg
                        )))
                    }
                }
                i += 1;
            }
        }
    }

    Ok(CliCommand::Run(config))
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn process(config: &Config) -> Result<()> {
    let input_path = Path::new(&config.input_path);
    if input_path.is_dir() {
        process_batch(config)
    } else {
        process_single(config)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn process_single(config: &Config) -> Result<()> {
    let input_path = Path::new(&config.input_path);
    let output_path = Path::new(&config.output_path);
    let processed = process_file(input_path, output_path, config)?;
    println!("Processing: {}", config.input_path);
    print_processed_image(
        processed.pixel_size,
        processed.pixel_size_override,
        processed.output_width,
        processed.output_height,
    );
    println!("Saved to: {}", config.output_path);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn process_batch(config: &Config) -> Result<()> {
    process_batch_with_reporter(&BatchConfig::from(config), |event| match event {
        BatchEvent::BatchStarted { input_dir, total } => {
            println!(
                "Batch processing {} image{} from: {}",
                total,
                if total == 1 { "" } else { "s" },
                input_dir.display()
            );
        }
        BatchEvent::Started {
            input,
            index,
            total,
        } => {
            println!("Processing {}/{}: {}", index + 1, total, input.display());
        }
        BatchEvent::Finished {
            input,
            output,
            index,
            total,
        } => {
            println!(
                "Done {}/{}: {} -> {}",
                index + 1,
                total,
                input.display(),
                output.display()
            );
        }
        BatchEvent::Failed {
            input,
            output,
            error,
            index,
            total,
        } => {
            eprintln!(
                "Failed {}/{}: {} -> {} ({})",
                index + 1,
                total,
                input.display(),
                output.display(),
                error
            );
        }
        BatchEvent::BatchFinished { input_dir, total } => {
            println!(
                "Processed {} image{} in: {}",
                total,
                if total == 1 { "" } else { "s" },
                input_dir.display()
            );
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_batch_with_reporter<F>(config: &BatchConfig, reporter: F) -> Result<()>
where
    F: Fn(BatchEvent) + Send + Sync,
{
    let input_dir = &config.input_dir;
    let output_dir = &config.output_dir;

    // Do not silently replace inputs; maybe that's ok though
    if input_dir == output_dir {
        return Err(PixelSnapperError::InvalidInput(
            "Batch output directory must be different from the input directory".to_string(),
        ));
    }

    if output_dir.exists() && !output_dir.is_dir() {
        return Err(PixelSnapperError::InvalidInput(format!(
            "Batch output path must be a directory: {}",
            output_dir.display()
        )));
    }

    std::fs::create_dir_all(output_dir).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to create output directory '{}': {}",
            output_dir.display(),
            e
        ))
    })?;

    let mut inputs = collect_batch_inputs(input_dir)?;
    inputs.sort();

    if inputs.is_empty() {
        return Err(PixelSnapperError::InvalidInput(format!(
            "No supported images found in '{}'",
            input_dir.display()
        )));
    }

    let items: Vec<(PathBuf, PathBuf)> = inputs
        .iter()
        .map(|input| Ok((input.clone(), get_output_path(output_dir, input)?)))
        .collect::<Result<_>>()?;

    reporter(BatchEvent::BatchStarted {
        input_dir: input_dir.clone(),
        total: items.len(),
    });

    let results: Vec<(PathBuf, Result<()>)> = items
        .par_iter()
        .enumerate()
        .map(|(index, (input, output))| {
            reporter(BatchEvent::Started {
                input: input.clone(),
                index,
                total: items.len(),
            });
            let item_config = Config::from(config);
            let result = process_file(input, output, &item_config).map(|_| ());
            match &result {
                Ok(()) => reporter(BatchEvent::Finished {
                    input: input.clone(),
                    output: output.clone(),
                    index,
                    total: items.len(),
                }),
                Err(err) => reporter(BatchEvent::Failed {
                    input: input.clone(),
                    output: output.clone(),
                    error: err.to_string(),
                    index,
                    total: items.len(),
                }),
            }
            (input.clone(), result)
        })
        .collect();

    let mut failures = Vec::new();
    for (input, result) in results {
        match result {
            Ok(()) => {}
            Err(err) => failures.push(format!("{} ({})", input.display(), err)),
        }
    }

    if failures.is_empty() {
        reporter(BatchEvent::BatchFinished {
            input_dir: input_dir.clone(),
            total: items.len(),
        });
        Ok(())
    } else {
        Err(PixelSnapperError::ProcessingError(format!(
            "Batch completed with {} failure{}: {}",
            failures.len(),
            if failures.len() == 1 { "" } else { "s" },
            failures.join("; ")
        )))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn process_file(input_path: &Path, output_path: &Path, config: &Config) -> Result<ProcessedImage> {
    let img_bytes = std::fs::read(input_path).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to read input file '{}': {}",
            input_path.display(),
            e
        ))
    })?;

    let processed = process_image_common(&img_bytes, Some(config.clone()))?;

    std::fs::write(output_path, &processed.output_bytes).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to write output file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    Ok(processed)
}

#[cfg(not(target_arch = "wasm32"))]
fn print_processed_image(
    pixel_size: f64,
    pixel_size_override: bool,
    output_width: u32,
    output_height: u32,
) {
    println!(
        "Pixel size: {:.1}px ({})",
        pixel_size,
        if pixel_size_override {
            "override"
        } else {
            "auto-detected"
        }
    );
    println!("Output size: {}x{}", output_width, output_height);
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_batch_inputs(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(input_dir).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to read input directory '{}': {}",
            input_dir.display(),
            e
        ))
    })?;

    let mut inputs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to read an entry from '{}': {}",
                input_dir.display(),
                e
            ))
        })?;
        let path = entry.path();
        if path.is_file() && is_supported_image_path(&path) {
            inputs.push(path);
        }
    }

    Ok(inputs)
}

#[cfg(not(target_arch = "wasm32"))]
fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_output_path(output_dir: &Path, input_path: &Path) -> Result<PathBuf> {
    let stem = input_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| {
            PixelSnapperError::InvalidInput(format!(
                "Input path has no file stem: {}",
                input_path.display()
            ))
        })?;

    Ok(output_dir.join(format!("{}.png", stem)))
}

fn quantize_image(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    if config.k_colors == 0 {
        return Err(PixelSnapperError::InvalidInput(
            "Number of colors must be greater than 0".to_string(),
        ));
    }

    let opaque_pixels: Vec<[f32; 3]> = img
        .pixels()
        .filter_map(|p| {
            if p[3] == 0 {
                None
            } else {
                Some([p[0] as f32, p[1] as f32, p[2] as f32])
            }
        })
        .collect();
    let n_pixels = opaque_pixels.len();
    if n_pixels == 0 {
        return Ok(img.clone());
    }

    let mut rng = ChaCha8Rng::seed_from_u64(config.k_seed);
    let k = config.k_colors.min(n_pixels);

    fn sample_index(rng: &mut ChaCha8Rng, upper: usize) -> usize {
        debug_assert!(upper > 0);
        let upper = upper as u64;
        rng.gen_range(0..upper) as usize
    }

    fn dist_sq(p: &[f32; 3], c: &[f32; 3]) -> f32 {
        let dr = p[0] - c[0];
        let dg = p[1] - c[1];
        let db = p[2] - c[2];
        dr * dr + dg * dg + db * db
    }

    let mut centroids: Vec<[f32; 3]> = Vec::with_capacity(k);
    let first_idx = sample_index(&mut rng, n_pixels);
    centroids.push(opaque_pixels[first_idx]);
    let mut distances = vec![f32::MAX; n_pixels];

    // Maybe try a faster algorithm for this? like https://crates.io/crates/kmeans_colors
    for _ in 1..k {
        let last_c = centroids.last().unwrap();
        let mut sum_sq_dist = 0.0;

        for (i, p) in opaque_pixels.iter().enumerate() {
            let d_sq = dist_sq(p, last_c);
            if d_sq < distances[i] {
                distances[i] = d_sq;
            }
            sum_sq_dist += distances[i];
        }

        if sum_sq_dist <= 0.0 {
            let idx = sample_index(&mut rng, n_pixels);
            centroids.push(opaque_pixels[idx]);
        } else {
            let dist = WeightedIndex::new(&distances).map_err(|e| {
                PixelSnapperError::ProcessingError(format!("Failed to sample new centroid: {}", e))
            })?;
            let idx = dist.sample(&mut rng);
            centroids.push(opaque_pixels[idx]);
        }
    }

    let mut prev_centroids = centroids.clone();
    for iteration in 0..config.max_kmeans_iterations {
        let mut sums = vec![[0.0f32; 3]; k];
        let mut counts = vec![0usize; k];

        for p in &opaque_pixels {
            let mut min_dist = f32::MAX;
            let mut best_k = 0;

            for (i, c) in centroids.iter().enumerate() {
                let d = dist_sq(p, c);
                if d < min_dist {
                    min_dist = d;
                    best_k = i;
                }
            }
            sums[best_k][0] += p[0];
            sums[best_k][1] += p[1];
            sums[best_k][2] += p[2];
            counts[best_k] += 1;
        }

        for i in 0..k {
            if counts[i] > 0 {
                let fcount = counts[i] as f32;
                centroids[i] = [
                    sums[i][0] / fcount,
                    sums[i][1] / fcount,
                    sums[i][2] / fcount,
                ];
            }
        }

        if iteration > 0 {
            let mut max_movement = 0.0f32;
            for (new_c, old_c) in centroids.iter().zip(prev_centroids.iter()) {
                let movement = dist_sq(new_c, old_c);
                if movement > max_movement {
                    max_movement = movement;
                }
            }

            if max_movement < 0.01 {
                break;
            }
        }

        prev_centroids.copy_from_slice(&centroids);
    }

    let mut new_img = RgbaImage::new(img.width(), img.height());
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] == 0 {
            new_img.put_pixel(x, y, *pixel);
            continue;
        }
        let p = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];
        let mut min_dist = f32::MAX;
        let mut best_c = [pixel[0], pixel[1], pixel[2]];

        for c in &centroids {
            let d = dist_sq(&p, c);
            if d < min_dist {
                min_dist = d;
                best_c = [c[0].round() as u8, c[1].round() as u8, c[2].round() as u8];
            }
        }
        new_img.put_pixel(x, y, Rgba([best_c[0], best_c[1], best_c[2], pixel[3]]));
    }
    Ok(new_img)
}

fn resample(img: &RgbaImage, cols: &[usize], rows: &[usize]) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];

            if xe <= xs || ye <= ys {
                continue;
            }

            let mut counts: HashMap<[u8; 4], usize> = HashMap::new();

            for y in ys..ye {
                for x in xs..xe {
                    if x < img.width() as usize && y < img.height() as usize {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        *counts.entry(p).or_insert(0) += 1;
                    }
                }
            }

            let mut best_pixel = [0, 0, 0, 0];

            let mut candidates: Vec<([u8; 4], usize)> = counts.into_iter().collect();
            candidates.sort_by(|a, b| {
                let count_cmp = b.1.cmp(&a.1);
                if count_cmp == Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    count_cmp
                }
            });

            if let Some(winner) = candidates.first() {
                best_pixel = winner.0;
            }

            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(best_pixel));
        }
    }
    Ok(final_img)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cli_tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn no_arguments_show_help() {
        assert!(matches!(parse_cli_args(&[]).unwrap(), CliCommand::Help));
    }

    #[test]
    fn help_and_version_flags_are_recognized() {
        assert!(matches!(
            parse_cli_args(&args(&["--help"])).unwrap(),
            CliCommand::Help
        ));
        assert!(matches!(
            parse_cli_args(&args(&["--version"])).unwrap(),
            CliCommand::Version
        ));
    }

    #[test]
    fn output_path_is_required() {
        let error = parse_cli_args(&args(&["input.png"])).unwrap_err();
        assert!(matches!(error, PixelSnapperError::InvalidInput(_)));
        assert!(error.to_string().contains("missing output path"));
    }

    #[test]
    fn parses_all_supported_cli_options() {
        let command = parse_cli_args(&args(&[
            "input.png",
            "output.png",
            "24",
            "--pixel-size",
            "8",
            "--palette",
            "0d2b45,ffecd6",
        ]))
        .unwrap();

        let CliCommand::Run(config) = command else {
            panic!("expected a processing command");
        };

        assert_eq!(config.input_path, "input.png");
        assert_eq!(config.output_path, "output.png");
        assert_eq!(config.k_colors, 24);
        assert_eq!(config.pixel_size_override, Some(8.0));
        assert_eq!(config.palette, Some(vec![[13, 43, 69], [255, 236, 214]]));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = parse_cli_args(&args(&["input.png", "output.png", "--unknown"])).unwrap_err();
        assert!(error.to_string().contains("unknown argument '--unknown'"));
    }
}
