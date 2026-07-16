#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    spritefusion_pixel_snapper::run_cli()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
