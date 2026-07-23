# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A single-crate Rust library+binary that fixes AI-generated pixel art by detecting its implicit pixel grid and re-snapping to it. The same source compiles to **two targets**: a native CLI binary and a WASM module consumed by the web app at spritefusion.com/pixel-snapper.

## Build / test / run

```bash
cargo build --release                                  # Native CLI → target/release/pixel-game-kit
cargo test                                             # Unit tests live in src/lib.rs under #[cfg(all(test, not(target_arch = "wasm32")))]
cargo test cli_tests                                   # Run only the CLI argument-parsing tests
cargo run --release -- <input> <output> [COLORS] [opts]  # Run without installing
cargo install --path .                                 # Installs the `pixel-game-kit` binary

wasm-pack build --target web --out-dir pkg --release   # WASM build → pkg/pixel_game_kit.js
```

Binary name (and crate name) is `pixel-game-kit`; the WASM JS export is `process_image`. No linter/formatter config exists in-repo — `cargo fmt` / `cargo clippy` work but aren't wired to CI.

## Architecture: dual-target + modular pipeline

The crate compiles to two targets from the same source:

- **WASM** (`cfg(target_arch = "wasm32")`): the `#[wasm_bindgen]` export `process_image` in [src/lib.rs](src/lib.rs).
- **Native CLI**: [src/cli.rs](src/cli.rs) is gated by `#![cfg(not(target_arch = "wasm32"))]` — the whole file is native-only, holding `run_cli`, hand-rolled arg parsing (no `clap`), single-file/batch processing (`rayon`-parallel, `BatchEvent` reporter), and the `cli_tests`. [src/main.rs](src/main.rs) is a 7-line shim calling `run_cli`.

The shared pipeline entry [`process_image_common`](src/lib.rs) (pub(crate)) is used by both targets. Each pipeline stage lives in its own module — keep `lib.rs` as orchestration only, add new stages as new modules:

| Stage | Module | Notes |
|-------|--------|-------|
| Config + Default | [config.rs](src/config.rs) | fields `pub(crate)`; `seed` (renamed from `k_seed`) drives all RNG |
| Errors | [error.rs](src/error.rs) | `PixelSnapperError` + `Result`; `JsValue` conv under wasm |
| Quantize (k-means++) | [quantize.rs](src/quantize.rs) | analysis-only color reduction |
| Profiles + step estimate | [profile.rs](src/profile.rs) | `compute_profiles` / `estimate_step_size` / `resolve_step_sizes` |
| Detect (runs/tiled/elastic) | [detect/mod.rs](src/detect/mod.rs) | candidate-returning detectors + Auto select |
| Stabilize (walker + cuts) | [stabilize.rs](src/stabilize.rs) | `walk`, `stabilize_both_axes`, `stabilize_cuts`, `snap_uniform_cuts`, `sanitize_cuts` |
| Resample (multi-strategy) | [resample/mod.rs](src/resample/mod.rs) | `majority`/`median`/`dominant`/`mode` dispatch |
| Palette | [palette.rs](src/palette.rs) | `parse_palette_hex` / `apply_palette` / `nearest_palette_color` + `MAX_PALETTE_COLORS` |
| Validate | [validate.rs](src/validate.rs) | dimension checks |

Don't add `clap` or other CLI deps — they'd be dead weight under `cfg(wasm32)`.

The crate is `cdylib` + `rlib` ([Cargo.toml](Cargo.toml)): `cdylib` for the WASM `.wasm`/`.js`, `rlib` so `cargo test` can link against it natively.

## The processing pipeline

`process_image_common` is the single entry point for both targets. The stages run strictly in order and each one's output feeds the next — changing an early stage silently perturbs everything downstream:

1. **`quantize_image`** — k-means++ (seeded `ChaCha8Rng`, `k_seed = 42`) reduces the image to `k_colors` centroids. Result is used *only as analysis input* for grid detection; final colors come from a separate path. Note: the TODO comment flags `kmeans_colors` crate as a faster alternative.
2. **`compute_profiles`** — projects a `[-1, 0, 1]` gradient kernel across rows and columns → two 1-D edge-strength profiles. Transparent pixels contribute 0.
3. **`detect`** — runs `runs` (GCD + posterize), `tiled` (Sobel + autocorrelation), and/or `elastic` (gradient walker) per `DetectStrategy`. Returns ranked `DetectionCandidate`s (detector, scale, step, confidence, cut_method). Auto runs all three; selection priority Runs>Tiled>Elastic then confidence.
4. **`cut`** — branches on the selected candidate's `cut_method`: `Uniform` → `snap_uniform_cuts` (integer grid); `Walker` → `walk` + `stabilize_both_axes` (skew/continuous).
5. **`resample`** — for each grid cell, reduce to one pixel per `ResampleMethod` (default `majority` = whole-pixel mode + RGBA tie-break). Alternatives: `median` (per-channel median + sample window, AA removal), `dominant` (top color if ≥ threshold, else mean), `mode` (per-channel mode; may emit colors not in source — use `majority` for strict palette preservation).
6. **`apply_palette`** (optional) — snaps every pixel to its nearest palette color (squared-Euclidean), cached per unique color.

## Tuning knobs

`Config` (default impl in [src/config.rs](src/config.rs)) holds ~15 parameters that control detection stability and resampling. The public CLI exposes `k_colors`, `pixel_size_override`, `palette`, `--detect` (strategy), `--resample` (strategy), `--sample-window` (median only), and `--json` (candidate output); everything else is internal. When debugging "wrong grid detected on this image," the usual suspects are `max_step_ratio` (skew), `walker_search_window_ratio` / `walker_strength_threshold` (peak sensitivity), and `fallback_target_segments` (last-resort grid density). For resampling, the internal fields are `resample_method`, `resample_sample_window`, `resample_dominant_threshold`, and `resample_dominant_binarize_alpha`.

## Constraints enforced in code

- Image dimensions: `3x3` minimum (profiles need neighbors), `10000x10000` maximum.
- `pixel_size_override`: must be finite and within `[1, min(w,h)/2]`.
- `k_colors`: must be `> 0`; effectively capped at `MAX_PALETTE_COLORS = 256`.
- Custom palette: comma-separated 6-digit hex, deduped, ≤ 256 distinct colors.
- Batch: input and output directories **must differ**; supported extensions are `png`/`jpg`/`jpeg` only; outputs are always written as `.png` named by the input stem.

## Determinism

All randomness flows through `ChaCha8Rng::seed_from_u64(42)`. The same input image + same `Config` always produces byte-identical output — preserve this when refactoring k-means or the walker.
