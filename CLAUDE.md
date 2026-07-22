# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A single-crate Rust library+binary that fixes AI-generated pixel art by detecting its implicit pixel grid and re-snapping to it. The same source compiles to **two targets**: a native CLI binary and a WASM module consumed by the web app at spritefusion.com/pixel-snapper.

## Build / test / run

```bash
cargo build --release                                  # Native CLI → target/release/spritefusion-pixel-snapper
cargo test                                             # Unit tests live in src/lib.rs under #[cfg(all(test, not(target_arch = "wasm32")))]
cargo test cli_tests                                   # Run only the CLI argument-parsing tests
cargo run --release -- <input> <output> [COLORS] [opts]  # Run without installing
cargo install --path .                                 # Installs the `spritefusion-pixel-snapper` binary

wasm-pack build --target web --out-dir pkg --release   # WASM build → pkg/spritefusion_pixel_snapper.js
```

Binary name (and crate name) is `spritefusion-pixel-snapper`; the WASM JS export is `process_image`. No linter/formatter config exists in-repo — `cargo fmt` / `cargo clippy` work but aren't wired to CI.

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
| Stabilize (walker + cuts) | [stabilize.rs](src/stabilize.rs) | `walk`, `stabilize_both_axes`, `stabilize_cuts`, `snap_uniform_cuts`, `sanitize_cuts` |
| Resample (majority vote) | [resample.rs](src/resample.rs) | grid-cell majority, deterministic RGBA tie-break |
| Palette | [palette.rs](src/palette.rs) | `parse_palette_hex` / `apply_palette` / `nearest_palette_color` + `MAX_PALETTE_COLORS` |
| Validate | [validate.rs](src/validate.rs) | dimension checks |

Don't add `clap` or other CLI deps — they'd be dead weight under `cfg(wasm32)`.

The crate is `cdylib` + `rlib` ([Cargo.toml](Cargo.toml)): `cdylib` for the WASM `.wasm`/`.js`, `rlib` so `cargo test` can link against it natively.

## The processing pipeline

`process_image_common` is the single entry point for both targets. The stages run strictly in order and each one's output feeds the next — changing an early stage silently perturbs everything downstream:

1. **`quantize_image`** — k-means++ (seeded `ChaCha8Rng`, `k_seed = 42`) reduces the image to `k_colors` centroids. Result is used *only as analysis input* for grid detection; final colors come from a separate path. Note: the TODO comment flags `kmeans_colors` crate as a faster alternative.
2. **`compute_profiles`** — projects a `[-1, 0, 1]` gradient kernel across rows and columns → two 1-D edge-strength profiles. Transparent pixels contribute 0.
3. **`estimate_step_size`** — finds peaks above `peak_threshold_multiplier * max`, dedups with `peak_distance_filter`, takes the **median** peak spacing. Returns `Option<f64>` — `None` means detection failed.
4. **`resolve_step_sizes`** — reconciles X/Y. If the ratio exceeds `max_step_ratio` (currently 1.8, lowered from 3.0 to catch skew), it snaps both to the smaller step; otherwise averages them. This is the primary anti-skew mechanism.
5. **`walk`** — an elastic walker that advances by `step_size` and snaps each cut to the strongest profile peak within a `walker_search_window_ratio` window, but only if the peak exceeds `mean * walker_strength_threshold`. A comment notes uniform-grid was tried and rejected as worse.
6. **`stabilize_both_axes` → `stabilize_cuts` → `snap_uniform_cuts`** — two-pass stabilization that cross-validates one axis against the other and falls back to a uniform grid (`fallback_target_segments`) when detection is incoherent or below `min_cuts_per_axis`.
7. **`resample`** — for each grid cell, picks the most common pixel by **majority vote** (deterministic tie-break by RGBA ordering).
8. **`apply_palette`** (optional) — snaps every pixel to its nearest palette color (squared-Euclidean), cached per unique color.

## Tuning knobs

`Config` (default impl in [src/config.rs](src/config.rs)) holds ~11 parameters that control detection stability. The public CLI only exposes `k_colors`, `pixel_size_override`, and `palette`; everything else is internal. When debugging "wrong grid detected on this image," the usual suspects are `max_step_ratio` (skew), `walker_search_window_ratio` / `walker_strength_threshold` (peak sensitivity), and `fallback_target_segments` (last-resort grid density).

## Constraints enforced in code

- Image dimensions: `3x3` minimum (profiles need neighbors), `10000x10000` maximum.
- `pixel_size_override`: must be finite and within `[1, min(w,h)/2]`.
- `k_colors`: must be `> 0`; effectively capped at `MAX_PALETTE_COLORS = 256`.
- Custom palette: comma-separated 6-digit hex, deduped, ≤ 256 distinct colors.
- Batch: input and output directories **must differ**; supported extensions are `png`/`jpg`/`jpeg` only; outputs are always written as `.png` named by the input stem.

## Determinism

All randomness flows through `ChaCha8Rng::seed_from_u64(42)`. The same input image + same `Config` always produces byte-identical output — preserve this when refactoring k-means or the walker.
