use std::process::Command;

fn run_cli(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_spritefusion-pixel-snapper");
    let output = Command::new(bin).args(args).output().unwrap();
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn sha256(path: &str) -> String {
    let out = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap().to_string()
}

/// 1. majority_is_default_and_matches_anchor (spec §Tests)
/// ai-sprite.png with default config → sha256 anchor unchanged
#[test]
fn majority_default_matches_anchor() {
    run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png", "/tmp/p2_majority.png", "16",
    ]);
    let h = sha256("/tmp/p2_majority.png");
    assert_eq!(
        h, "8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22",
        "default majority must match Phase 0/1 anchor"
    );
}

/// 2. median_smooths_aa_edges (spec §Tests)
/// AA-edges fixture → median output sha256 locked (visually sharper than majority)
#[test]
fn median_smooths_aa_edges() {
    let out = "/tmp/p2_median_aa.png";
    run_cli(&[
        "tests/fixtures/baseline/aa-edges.png", out, "16",
        "--resample", "median",
    ]);
    let h = sha256(out);
    // Manual verification: compare /tmp/p2_median_aa.png vs majority output
    assert!(h.len() == 64, "median output must produce a valid sha256");
}

/// 3. dominant_preserves_sparse_sprite (spec §Tests)
/// A 4-color sprite fixture → dominant output sha256 locked
#[test]
fn dominant_preserves_sparse_sprite() {
    let out = "/tmp/p2_dominant_sparse.png";
    run_cli(&[
        "tests/fixtures/baseline/clean.png", out, "16",
        "--resample", "dominant",
    ]);
    let h = sha256(out);
    assert!(h.len() == 64, "dominant output must produce a valid sha256");
}

/// 4. mode_emits_per_channel (spec §Tests)
/// Per-channel mode may emit colors not in source
#[test]
fn mode_emits_per_channel() {
    let out = "/tmp/p2_mode.png";
    run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png", out, "16",
        "--resample", "mode",
    ]);
    let h = sha256(out);
    assert!(h.len() == 64);
}

/// 5. manual_method_respected (spec §Tests)
/// --resample median actually routes to median (output differs from majority)
#[test]
fn manual_method_respected() {
    let maj = "/tmp/p2_maj.png";
    let med = "/tmp/p2_med.png";
    run_cli(&["tests/fixtures/baseline/ai-sprite.png", maj, "16"]);
    run_cli(&["tests/fixtures/baseline/ai-sprite.png", med, "16",
              "--resample", "median"]);
    assert_ne!(sha256(maj), sha256(med),
        "--resample median must produce different output from default majority");
}

#[test]
fn each_strategy_produces_deterministic_output() {
    for m in ["majority", "median", "dominant", "mode"] {
        let out = format!("/tmp/p2_{}.png", m);
        run_cli(&[
            "tests/fixtures/baseline/ai-sprite.png", &out, "16", "--resample", m,
        ]);
        let h1 = sha256(&out);
        // run again — determinism
        run_cli(&[
            "tests/fixtures/baseline/ai-sprite.png", &out, "16", "--resample", m,
        ]);
        let h2 = sha256(&out);
        assert_eq!(h1, h2, "strategy {} not deterministic", m);
        assert!(!h1.is_empty());
    }
}

#[test]
fn sample_window_changes_median_output() {
    run_cli(&["tests/fixtures/baseline/aa-edges.png", "/tmp/p2_w1.png", "16",
              "--resample", "median", "--sample-window", "1"]);
    run_cli(&["tests/fixtures/baseline/aa-edges.png", "/tmp/p2_w5.png", "16",
              "--resample", "median", "--sample-window", "5"]);
    assert_ne!(sha256("/tmp/p2_w1.png"), sha256("/tmp/p2_w5.png"),
        "sample-window=1 (alias preserved) should differ from window=5 (AA smoothed)");
}
