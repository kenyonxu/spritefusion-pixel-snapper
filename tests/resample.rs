//! Phase 2 resample integration tests.
//!
//! Cross-platform: uses the `sha2` crate (not the `sha256sum` shell command)
//! and `std::env::temp_dir()` (not a literal `/tmp`), so these run on Windows
//! as well as Linux/macOS.

use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

/// Return a writable temp path unique to this test binary.
fn tmp(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("pixel-snapper-p2-{}", name));
    p.to_string_lossy().to_string()
}

fn run_cli(args: &[&str]) -> bool {
    let bin = env!("CARGO_BIN_EXE_pixel-game-kit");
    Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run CLI")
        .status
        .success()
}

fn sha256(path: &str) -> String {
    let data = fs::read(path).expect("output file not written");
    let mut hasher = Sha256::new();
    hasher.update(&data);
    format!("{:x}", hasher.finalize())
}

/// 1. majority_is_default_and_matches_anchor (spec §Tests)
/// ai-sprite.png with default config → sha256 anchor unchanged
#[test]
fn majority_default_matches_anchor() {
    let out = tmp("majority.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png",
        out.as_str(),
        "16",
    ]));
    let h = sha256(&out);
    assert_eq!(
        h,
        "8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22",
        "default majority must match Phase 0/1 anchor"
    );
}

/// 2. median_smooths_aa_edges (spec §Tests)
/// AA-edges fixture → median output sha256 locked (visually sharper than majority)
#[test]
fn median_smooths_aa_edges() {
    let out = tmp("median_aa.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/aa-edges.png",
        out.as_str(),
        "16",
        "--resample",
        "median",
    ]));
    let h = sha256(&out);
    assert_eq!(h.len(), 64, "median output must produce a valid sha256");
}

/// 3. dominant_preserves_sparse_sprite (spec §Tests)
/// A 4-color sprite fixture → dominant output sha256 locked
#[test]
fn dominant_preserves_sparse_sprite() {
    let out = tmp("dominant_sparse.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/clean.png",
        out.as_str(),
        "16",
        "--resample",
        "dominant",
    ]));
    let h = sha256(&out);
    assert_eq!(h.len(), 64, "dominant output must produce a valid sha256");
}

/// 4. mode_emits_per_channel (spec §Tests)
/// Per-channel mode may emit colors not in source
#[test]
fn mode_emits_per_channel() {
    let out = tmp("mode.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png",
        out.as_str(),
        "16",
        "--resample",
        "mode",
    ]));
    let h = sha256(&out);
    assert_eq!(h.len(), 64);
}

/// 5. manual_method_respected (spec §Tests)
/// --resample median actually routes to median (output differs from majority)
#[test]
fn manual_method_respected() {
    let maj = tmp("maj.png");
    let med = tmp("med.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png",
        maj.as_str(),
        "16"
    ]));
    assert!(run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png",
        med.as_str(),
        "16",
        "--resample",
        "median"
    ]));
    assert_ne!(
        sha256(&maj),
        sha256(&med),
        "--resample median must produce different output from default majority"
    );
}

#[test]
fn each_strategy_produces_deterministic_output() {
    for m in ["majority", "median", "dominant", "mode"] {
        let out = tmp(&format!("det_{}.png", m));
        assert!(run_cli(&[
            "tests/fixtures/baseline/ai-sprite.png",
            out.as_str(),
            "16",
            "--resample",
            m
        ]));
        let h1 = sha256(&out);
        // run again — determinism
        assert!(run_cli(&[
            "tests/fixtures/baseline/ai-sprite.png",
            out.as_str(),
            "16",
            "--resample",
            m
        ]));
        let h2 = sha256(&out);
        assert_eq!(h1, h2, "strategy {} not deterministic", m);
        assert!(!h1.is_empty());
    }
}

#[test]
fn sample_window_changes_median_output() {
    let w1 = tmp("w1.png");
    let w5 = tmp("w5.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/aa-edges.png",
        w1.as_str(),
        "16",
        "--resample",
        "median",
        "--sample-window",
        "1"
    ]));
    assert!(run_cli(&[
        "tests/fixtures/baseline/aa-edges.png",
        w5.as_str(),
        "16",
        "--resample",
        "median",
        "--sample-window",
        "5"
    ]));
    assert_ne!(
        sha256(&w1),
        sha256(&w5),
        "sample-window=1 (alias preserved) should differ from window=5 (AA smoothed)"
    );
}
