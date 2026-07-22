use crate::palette::parse_palette_hex;
use crate::{Config, PixelSnapperError, Result};
use std::env;

#[derive(Debug)]
pub enum CliCommand {
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
        Ok(CliCommand::Run(config)) => match crate::cli::process(&config) {
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

pub fn print_cli_help() {
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
            "  --json                 Output detection candidates as JSON instead of processing\n",
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

pub fn parse_cli_args(args: &[String]) -> Result<CliCommand> {
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
            "--json" => {
                config.json_output = true;
                i += 1;
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
