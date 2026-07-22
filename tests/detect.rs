use spritefusion_pixel_snapper::detect::{
    detect, select_best, CutMethod, DetectStrategy,
};

fn load_fixture(name: &str) -> image::RgbaImage {
    let bytes = std::fs::read(format!("tests/fixtures/baseline/{}", name)).unwrap();
    image::load_from_memory(&bytes).unwrap().to_rgba8()
}

#[test]
fn elastic_returns_walker_candidate_for_ai_sprite() {
    let img = load_fixture("ai-sprite.png");
    let (w, h) = img.dimensions();
    // profiles computed by the lib's internal pipeline; for a unit test we
    // pass empty profiles and let detect_elastic recompute via profile module.
    let config = spritefusion_pixel_snapper::Config::default();
    let cands = detect(&img, &[], &[], w, h, &config, DetectStrategy::Elastic);
    assert!(cands.iter().any(|c| {
        c.detector == DetectStrategy::Elastic && c.cut_method == CutMethod::Walker
    }));
}
#[test]
#[ignore = "fixture added in Task 10"]
fn runs_detects_clean_fixture() {
    let img = load_fixture("clean.png");
    let (w, h) = img.dimensions();
    let config = spritefusion_pixel_snapper::Config::default();
    let cands = detect(&img, &[], &[], w, h, &config, DetectStrategy::Runs);
    assert!(cands.iter().any(|c| {
        c.detector == DetectStrategy::Runs
            && c.cut_method == CutMethod::Uniform
            && c.scale.unwrap_or(0) >= 2
    }));
}

#[test]
fn runs_returns_none_on_tiny_noise() {
    let mut img = image::RgbaImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, image::Rgba([(x * 31) as u8, (y * 17) as u8, 0, 255]));
        }
    }
    let config = spritefusion_pixel_snapper::Config::default();
    let (w, h) = img.dimensions();
    let cands = detect(&img, &[], &[], w, h, &config, DetectStrategy::Runs);
    // pure noise has no consistent run gcd; accept either None or low-confidence
    assert!(cands.is_empty() || cands[0].confidence < 0.9);
}
