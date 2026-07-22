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
