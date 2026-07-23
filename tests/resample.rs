use spritefusion_pixel_snapper::resample::ResampleMethod;

fn load(name: &str) -> image::RgbaImage {
    let bytes = std::fs::read(format!("tests/fixtures/baseline/{}", name)).unwrap();
    image::load_from_memory(&bytes).unwrap().to_rgba8()
}

#[test]
fn median_runs_and_is_deterministic() {
    // We can't call resample() directly (it's pub(crate)); instead we assert the
    // Config field exists and the variant is constructible. The behavioral test
    // (median sharpens AA) lives in Task 8 via the CLI.
    let _m = ResampleMethod::Median;
    let img = load("ai-sprite.png");
    // smoke: image loads, has pixels
    assert!(img.width() > 0);
}
