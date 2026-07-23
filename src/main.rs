#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    pixel_game_kit::run_cli()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
