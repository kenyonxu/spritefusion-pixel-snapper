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

## Architecture: dual-target via conditional compilation

This is the central architectural fact. Almost every function and struct in [src/lib.rs](src/lib.rs) is split by `cfg`:

- **WASM target** (`cfg(target_arch = "wasm32")`): `process_image` is `#[wasm_bindgen]`, `Config` derives `wasm_bindgen`, `PixelSnapperError` converts to `JsValue`, `main` is empty. Filesystem, `env`, `rayon`, CLI parsing, batch processing — all excluded.
- **Native target** (`cfg(not(target_arch = "wasm32"))`): `run_cli` parses args by hand (no `clap`), drives single-file or batch processing, uses `rayon` for parallelism, emits `BatchEvent`s through a reporter callback.

[src/main.rs](src/main.rs) is a 7-line shim that picks the right path. Don't add `clap` or other CLI deps without considering the WASM build — they'd be dead weight under `cfg(wasm32)`.

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

`Config` (default impl at [src/lib.rs:46](src/lib.rs#L46)) holds ~11 parameters that control detection stability. The public CLI only exposes `k_colors`, `pixel_size_override`, and `palette`; everything else is internal. When debugging "wrong grid detected on this image," the usual suspects are `max_step_ratio` (skew), `walker_search_window_ratio` / `walker_strength_threshold` (peak sensitivity), and `fallback_target_segments` (last-resort grid density).

## Constraints enforced in code

- Image dimensions: `3x3` minimum (profiles need neighbors), `10000x10000` maximum.
- `pixel_size_override`: must be finite and within `[1, min(w,h)/2]`.
- `k_colors`: must be `> 0`; effectively capped at `MAX_PALETTE_COLORS = 256`.
- Custom palette: comma-separated 6-digit hex, deduped, ≤ 256 distinct colors.
- Batch: input and output directories **must differ**; supported extensions are `png`/`jpg`/`jpeg` only; outputs are always written as `.png` named by the input stem.

## Determinism

All randomness flows through `ChaCha8Rng::seed_from_u64(42)`. The same input image + same `Config` always produces byte-identical output — preserve this when refactoring k-means or the walker.
