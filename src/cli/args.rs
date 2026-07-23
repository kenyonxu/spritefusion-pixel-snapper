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
            println!("pixel-game-kit {}", env!("CARGO_PKG_VERSION"));
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
            eprintln!("Run 'pixel-game-kit --help' for usage.");
            std::process::ExitCode::from(2)
        }
    }
}

pub fn print_cli_help() {
    println!(
        concat!(
            "Pixel Game Kit {version}\n",
            "Fix inconsistent pixel art by detecting and snapping it to its implicit grid.\n\n",
            "USAGE:\n",
            "  pixel-game-kit <INPUT> <OUTPUT> [COLORS] [OPTIONS]\n\n",
            "ARGUMENTS:\n",
            "  <INPUT>   Input PNG/JPEG file, or a directory for batch processing\n",
            "  <OUTPUT>  Output PNG file, or a different output directory for a batch\n",
            "  [COLORS]  Number of palette colors [default: 16]\n\n",
            "OPTIONS:\n",
            "  --pixel-size <PIXELS>                       Override the auto-detected pixel size\n",
            "  --palette <HEX,...>                         Use comma-separated 6-digit hex palette colors\n",
            "  --detect <auto|runs|tiled|elastic>          Grid detection strategy [default: auto]\n",
            "  --resample <majority|median|dominant|mode|qvote>  Grid-cell reduction [default: majority]\n",
            "  --sample-window <1-9>                       Median neighborhood [default: 3]\n",
            "  --colorspace <rgb|oklab>                    Quantize colorspace [default: oklab]\n",
            "  --dither <none|fs|bayer2|bayer4|bayer8|ordered>  Dithering [default: none]\n",
            "  --dither-strength <0-2>                     Dither strength [default: 1.0]\n",
            "  --preset <name>                             Snap to preset palette [default: none]\n",
            "  --json                                      Output detection candidates as JSON instead of processing\n",
            "  -h, --help                                  Print help\n",
            "  -V, --version                               Print version\n\n",
            "EXAMPLES:\n",
            "  pixel-game-kit input.png output.png\n",
            "  pixel-game-kit input.png output.png 16 --pixel-size 8\n",
            "  pixel-game-kit inputs outputs --palette 0d2b45,ffecd6"
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
            "--resample" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--resample requires a value".to_string(),
                    ));
                };
                config.resample_method = match val.as_str() {
                    "majority" => crate::resample::ResampleMethod::Majority,
                    "median" => crate::resample::ResampleMethod::Median,
                    "dominant" => crate::resample::ResampleMethod::Dominant,
                    "mode" => crate::resample::ResampleMethod::Mode,
                    "qvote" => crate::resample::ResampleMethod::Qvote,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --resample '{}' (expected majority|median|dominant|mode|qvote)",
                            val
                        )))
                    }
                };
                i += 2;
            }
            "--sample-window" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--sample-window requires a value".to_string(),
                    ));
                };
                match val.parse::<usize>() {
                    Ok(n) if (1..=9).contains(&n) => config.resample_sample_window = n,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --sample-window '{}' (expected 1-9)", val
                    ))),
                }
                i += 2;
            }
            "--json" => {
                config.json_output = true;
                i += 1;
            }
            "--colorspace" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--colorspace requires a value".to_string(),
                    ));
                };
                config.quantize_colorspace = match val.as_str() {
                    "rgb" => crate::quantize::Colorspace::Rgb,
                    "oklab" => crate::quantize::Colorspace::Oklab,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --colorspace '{}' (expected rgb|oklab)",
                            val
                        )))
                    }
                };
                i += 2;
            }
            "--dither" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--dither requires a value".to_string(),
                    ));
                };
                config.quantize_dither = match val.as_str() {
                    "none" => crate::quantize::DitherMethod::None,
                    "fs" => crate::quantize::DitherMethod::FloydSteinberg,
                    "bayer2" => crate::quantize::DitherMethod::Bayer2,
                    "bayer4" => crate::quantize::DitherMethod::Bayer4,
                    "bayer8" => crate::quantize::DitherMethod::Bayer8,
                    "ordered" => crate::quantize::DitherMethod::Ordered,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --dither '{}' (expected none|fs|bayer2|bayer4|bayer8|ordered)",
                            val
                        )))
                    }
                };
                i += 2;
            }
            "--dither-strength" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--dither-strength requires a value".to_string(),
                    ));
                };
                match val.parse::<f64>() {
                    Ok(s) if (0.0..=2.0).contains(&s) => config.quantize_dither_strength = s,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --dither-strength '{}' (expected 0-2)",
                            val
                        )))
                    }
                }
                i += 2;
            }
            "--preset" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--preset requires a value".to_string(),
                    ));
                };
                config.quantize_preset_palette = match val.as_str() {
                    "none" => crate::quantize::PresetPalette::None,
                    "nes" => crate::quantize::PresetPalette::Nes,
                    "gameboy" => crate::quantize::PresetPalette::GameBoy,
                    "sgb" => crate::quantize::PresetPalette::Sgb,
                    "snes" => crate::quantize::PresetPalette::Snes,
                    "pc9801" => crate::quantize::PresetPalette::Pc9801,
                    "msx1" => crate::quantize::PresetPalette::Msx1,
                    "pico8" => crate::quantize::PresetPalette::Pico8,
                    "sweetie16" => crate::quantize::PresetPalette::Sweetie16,
                    "endesga32" => crate::quantize::PresetPalette::Endesga32,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --preset '{}'",
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
