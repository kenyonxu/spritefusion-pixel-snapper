use super::args::{parse_cli_args, CliCommand};
use crate::PixelSnapperError;

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

#[test]
fn parses_detect_strategy_flag() {
    let command =
        parse_cli_args(&args(&["input.png", "output.png", "--detect", "tiled"])).unwrap();
    let CliCommand::Run(config) = command else {
        panic!("expected Run");
    };
    assert_eq!(config.detect_strategy, crate::detect::DetectStrategy::Tiled);
}

#[test]
fn parses_resample_flag() {
    let command = parse_cli_args(&args(&[
        "input.png", "output.png", "--resample", "median",
    ])).unwrap();
    let CliCommand::Run(config) = command else { panic!("expected Run"); };
    assert_eq!(config.resample_method, crate::resample::ResampleMethod::Median);
}

#[test]
fn parses_sample_window_flag() {
    let command = parse_cli_args(&args(&[
        "input.png", "output.png", "--sample-window", "5",
    ])).unwrap();
    let CliCommand::Run(config) = command else { panic!("expected Run"); };
    assert_eq!(config.resample_sample_window, 5);
}

#[test]
fn parses_colorspace_flag() {
    let cmd = parse_cli_args(&args(&["i.png", "o.png", "--colorspace", "rgb"])).unwrap();
    let CliCommand::Run(c) = cmd else { panic!("expected Run") };
    assert_eq!(c.quantize_colorspace, crate::quantize::Colorspace::Rgb);
}

#[test]
fn parses_dither_flag() {
    let cmd = parse_cli_args(&args(&["i.png", "o.png", "--dither", "bayer4"])).unwrap();
    let CliCommand::Run(c) = cmd else { panic!("expected Run") };
    assert_eq!(c.quantize_dither, crate::quantize::DitherMethod::Bayer4);
}

#[test]
fn parses_preset_flag() {
    let cmd = parse_cli_args(&args(&["i.png", "o.png", "--preset", "pico8"])).unwrap();
    let CliCommand::Run(c) = cmd else { panic!("expected Run") };
    assert_eq!(c.quantize_preset_palette, crate::quantize::PresetPalette::Pico8);
}
