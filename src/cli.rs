//! CLI entry: argument parsing, single-file and batch processing (native only).

#![cfg(not(target_arch = "wasm32"))]

use crate::palette::parse_palette_hex;
use crate::{process_image_common, Config, PixelSnapperError, ProcessedImage, Result};
use rayon::prelude::*;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub palette: Option<Vec<[u8; 3]>>,
    #[test]
    fn parses_detect_strategy_flag() {
        let command = parse_cli_args(&args(&[
            "input.png",
            "output.png",
            "--detect",
            "tiled",
        ]))
        .unwrap();
        let CliCommand::Run(config) = command else {
            panic!("expected Run");
        };
        assert_eq!(config.detect_strategy, crate::detect::DetectStrategy::Tiled);
    }
}

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

#[derive(Debug)]
enum CliCommand {
    Run(Config),
    Help,
    Version,
}

/// Internal entry point used by the packaged CLI binary.
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
            "  --detect <auto|runs|tiled|elastic>  Grid detection strategy [default: auto]\n",
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
            "--detect" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--detect requires a value".to_string(),
                    ));
                };
                config.detect_strategy = match val.as_str() {
                    "auto" => crate::detect::DetectStrategy::Auto,
                    "runs" => crate::detect::DetectStrategy::Runs,
                    "tiled" => crate::detect::DetectStrategy::Tiled,
                    "elastic" => crate::detect::DetectStrategy::Elastic,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --detect '{}' (expected auto|runs|tiled|elastic)",
                            val
                        )))
                    }
                };
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

#[allow(dead_code)]
fn process(config: &Config) -> Result<()> {
    let input_path = Path::new(&config.input_path);
    if input_path.is_dir() {
        process_batch(config)
    } else {
        process_single(config)
    }
}

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

fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
        .unwrap_or(false)
}

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
