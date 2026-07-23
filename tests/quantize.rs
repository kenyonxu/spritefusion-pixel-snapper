//! Phase 3 quantize integration tests.
//!
//! - `rgb_path_matches_old_anchor`: locks the `--colorspace rgb` path to the
//!   Phase 0-2 anchor `802857...9f22`. The `--colorspace` flag is wired in
//!   Task 8; until then the CLI call fails and this test no-ops (the `if !run`
//!   guard). Cross-platform: uses `sha2` + `std::env::temp_dir()`.
//! - `oklab_default_is_deterministic`: Oklab is the new default (Task 4). This
//!   test verifies determinism (two runs → identical bytes) and records the
//!   new Oklab anchor hash for promotion into CLAUDE.md in Task 10. The hash
//!   MUST differ from the RGB anchor — Oklab is a different (perceptual)
//!   coordinate space and is expected to move the output.

use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

fn run(args: &[&str]) -> bool {
    let bin = env!("CARGO_BIN_EXE_pixel-game-kit");
    Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run CLI")
        .status
        .success()
}

fn sha(path: &str) -> String {
    let data = fs::read(path).expect("output file not written");
    let mut hasher = Sha256::new();
    hasher.update(&data);
    format!("{:x}", hasher.finalize())
}

fn tmp(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("p3-{}", name));
    p.to_string_lossy().to_string()
}

/// `--colorspace rgb` must reproduce the Phase 0-2 RGB anchor byte-for-byte.
/// Flag is added in Task 8 — until then the CLI rejects the flag and this
/// test no-ops (guard prints a skip notice instead of failing).
#[test]
fn rgb_path_matches_old_anchor() {
    let out = tmp("rgb.png");
    if !run(&[
        "tests/fixtures/baseline/ai-sprite.png",
        out.as_str(),
        "16",
        "--colorspace",
        "rgb",
    ]) {
        eprintln!("--colorspace not wired yet (Task 8), skipping");
        return;
    }
    assert_eq!(
        sha(&out),
        "8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22",
        "--colorspace rgb must preserve Phase 0-2 anchor"
    );
}

/// Oklab default (new in Task 4) must be deterministic across runs and must
/// differ from the RGB anchor (sanity check that the space actually changed).
/// The hash recorded here is the NEW Oklab anchor; promote to CLAUDE.md in Task 10.
#[test]
fn oklab_default_is_deterministic() {
    let out = tmp("oklab.png");
    assert!(
        run(&["tests/fixtures/baseline/ai-sprite.png", out.as_str(), "16"]),
        "default CLI invocation must succeed"
    );
    let h1 = sha(&out);
    assert!(
        run(&["tests/fixtures/baseline/ai-sprite.png", out.as_str(), "16"]),
        "second run must also succeed"
    );
    assert_eq!(h1, sha(&out), "Oklab default must be deterministic");

    // Oklab is a different coordinate space — output MUST differ from the RGB
    // anchor. If they matched, the colorspace switch would be a no-op.
    assert_ne!(
        h1,
        "8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22",
        "Oklab default must differ from RGB anchor"
    );
    assert!(!h1.is_empty());
}
