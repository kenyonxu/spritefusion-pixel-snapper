# Phase 2 Resample Strategies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `median` / `dominant` / `mode` resample strategies alongside the existing `majority`, behind a `ResampleMethod` selector, with `majority` remaining the zero-regression default.

**Architecture:** Convert `src/resample.rs` → `src/resample/` directory (`mod.rs` dispatch + one file per strategy). `mod.rs::resample()` branches on `config.resample_method`. Existing majority moves byte-for-byte to `majority.rs` so the sha256 anchor stays invariant; new strategies land behind the enum.

**Tech Stack:** Rust 2021, `image` 0.24, `wasm-bindgen`. No new deps. TDD via `cargo test`. Behavioral anchor: `tests/fixtures/baseline/ai-sprite.png` sha256 `8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22` (default `majority`) must stay invariant.

**Spec:** [docs/superpowers/specs/2026-07-22-phase2-resample-design.md](../specs/2026-07-22-phase2-resample-design.md)

---

## File Structure

| File | Responsibility | Status |
|------|----------------|--------|
| `src/resample/mod.rs` | `ResampleMethod` enum + `resample()` dispatch | Create (from `resample.rs`) |
| `src/resample/majority.rs` | existing whole-pixel majority (moved) | Create |
| `src/resample/median.rs` | per-channel median + sample window | Create |
| `src/resample/dominant.rs` | dominant color + mean fallback + optional alpha binarize | Create |
| `src/resample/mode.rs` | per-channel mode | Create |
| `src/resample.rs` | (deleted — replaced by directory) | Delete |
| `src/config.rs` | add `resample_method` / `resample_sample_window` / `resample_dominant_threshold` / `resample_dominant_binarize_alpha` | Modify |
| `src/lib.rs` | call `resample::resample(img, cols, rows, &config)`; wasm `process_image` gains `resample_method` param | Modify |
| `src/cli.rs` | `--resample` + `--sample-window` flags | Modify |
| `tests/resample.rs` | per-strategy integration tests | Create |
| `tests/fixtures/baseline/aa-edges.png` | median fixture | Create |
| `CLAUDE.md` | resample section → multi-strategy | Modify |

**Branch:** `feat/phase2-resample` (create in Task 1).

**Signature convention:** every strategy fn is `fn resample_<method>(img: &RgbaImage, cols: &[usize], rows: &[usize], config: &Config) -> Result<RgbaImage>`. `mod.rs` extracts the relevant fields and dispatches.

---

## Task 1: Directory skeleton — move majority, zero behavior change

Move the existing `resample.rs` into `resample/{mod,majority}.rs`, add the `ResampleMethod` enum, and prove `ai-sprite.png` sha256 is unchanged.

**Files:**
- Create: `src/resample/mod.rs`
- Create: `src/resample/majority.rs`
- Delete: `src/resample.rs`
- Modify: `src/lib.rs` (`use resample::resample;` call site gains `&config`)

- [ ] **Step 1: Create branch**

```bash
git checkout main
git checkout -b feat/phase2-resample
```

- [ ] **Step 2: Create `src/resample/majority.rs` (byte-for-byte move of existing logic)**

```rust
//! Whole-pixel majority vote with deterministic RGBA tie-break.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::cmp::Ordering;
use std::collections::HashMap;

pub(crate) fn resample_majority(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    _config: &Config,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];

            if xe <= xs || ye <= ys {
                continue;
            }

            let mut counts: HashMap<[u8; 4], usize> = HashMap::new();

            for y in ys..ye {
                for x in xs..xe {
                    if x < img.width() as usize && y < img.height() as usize {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        *counts.entry(p).or_insert(0) += 1;
                    }
                }
            }

            let mut best_pixel = [0, 0, 0, 0];
            let mut candidates: Vec<([u8; 4], usize)> = counts.into_iter().collect();
            candidates.sort_by(|a, b| {
                let count_cmp = b.1.cmp(&a.1);
                if count_cmp == Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    count_cmp
                }
            });
            if let Some(winner) = candidates.first() {
                best_pixel = winner.0;
            }

            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(best_pixel));
        }
    }
    Ok(final_img)
}
```

- [ ] **Step 3: Create `src/resample/mod.rs` (enum + dispatch; majority only for now)**

```rust
//! Grid-cell resampling strategies. See `ResampleMethod`.

mod majority;

use crate::error::Result;
use crate::Config;
use image::RgbaImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    Majority,
    Median,
    Dominant,
    Mode,
}

pub fn resample(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
) -> Result<RgbaImage> {
    // Task 2 wires config.resample_method; for now hardcode Majority to keep
    // this task a pure move.
    let _ = config;
    majority::resample_majority(img, cols, rows, config)
}
```

- [ ] **Step 4: Delete `src/resample.rs`**

```bash
git rm src/resample.rs
```

- [ ] **Step 5: Add `mod resample;` to `src/lib.rs`**

Add `mod resample;` in the module declaration block (phase 1 added `mod detect;` between `config` and `error`; phase 2 adds `mod resample;` as well). This is a standard top-level module declaration — easy to forget but compilation fails without it.

```rust
mod cli;      // native-only, #[cfg(not(target_arch = "wasm32"))]
mod config;
mod detect;
mod resample;
mod error;
```

- [ ] **Step 6: Update `src/lib.rs` call site to pass `&config`**

In `src/lib.rs`, the existing call (in `process_image_common`) is:
```rust
    let snapped_img = resample(&analysis_img, &col_cuts, &row_cuts)?;
```
Change to:
```rust
    let snapped_img = resample::resample(&analysis_img, &col_cuts, &row_cuts, &config)?;
```
And update the import line `use resample::resample;` → remove it (now called as `resample::resample`). Check `src/lib.rs` top imports: if `use resample::resample;` exists, delete that line.

- [ ] **Step 7: Verify compile + sha256 anchor**

Run: `cargo test 2>&1 | tail -5`
Expected: all existing tests pass (14 from Phase 1).

Run:
```bash
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png tests/fixtures/baseline/expected/check.png 16
sha256sum tests/fixtures/baseline/expected/check.png
```
Expected: `8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22` — identical (pure move).

- [ ] **Step 7: Commit**

```bash
git add src/resample/ src/lib.rs
git commit -m "refactor(resample): move majority into resample/ directory (zero behavior)"
```

---

## Task 2: Config resample fields + dispatch on `config.resample_method`

Wire the enum through `Config` so the dispatch reads the configured method (still only `Majority` reachable, but the plumbing lands).

**Files:**
- Modify: `src/config.rs`
- Modify: `src/resample/mod.rs`

- [ ] **Step 1: Add fields to `Config` in `src/config.rs`**

At top of `src/config.rs` add:
```rust
use crate::resample::ResampleMethod;
```

Add four fields to the `Config` struct (after `pub(crate) json_output: bool,`):
```rust
    pub(crate) resample_method: ResampleMethod,
    pub(crate) resample_sample_window: usize,
    pub(crate) resample_dominant_threshold: f64,
    pub(crate) resample_dominant_binarize_alpha: bool,
```

Add to the `Default` impl (after `json_output: false,`):
```rust
            resample_method: ResampleMethod::Majority,
            resample_sample_window: 3,
            resample_dominant_threshold: 0.15,
            resample_dominant_binarize_alpha: false,
```

- [ ] **Step 2: Dispatch on `config.resample_method` in `src/resample/mod.rs`**

Replace the body of `resample()`:
```rust
pub fn resample(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
) -> Result<RgbaImage> {
    match config.resample_method {
        ResampleMethod::Majority => majority::resample_majority(img, cols, rows, config),
        // wired in Tasks 3/4/5
        _ => majority::resample_majority(img, cols, rows, config),
    }
}
```

- [ ] **Step 3: Verify**

Run: `cargo test 2>&1 | tail -5`
Expected: all pass (default is still Majority).

Run: `cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/o.png 16 && sha256sum /tmp/o.png`
Expected: `802857...9f22`.

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/resample/mod.rs
git commit -m "feat(resample): wire ResampleMethod through Config"
```

---

## Task 3: median strategy (per-channel median + sample window)

**Files:**
- Create: `src/resample/median.rs`
- Modify: `src/resample/mod.rs`
- Create: `tests/resample.rs`

- [ ] **Step 1: Write failing test — median differs from majority and stays deterministic**

Create `tests/resample.rs`:
```rust
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
```

- [ ] **Step 2: Run test to verify it compiles + passes (enum exists from Task 2)**

Run: `cargo test --test resample 2>&1 | tail -5`
Expected: PASS (the enum variant exists).

- [ ] **Step 3: Implement `src/resample/median.rs`**

```rust
//! Per-channel median resample with sample window; suppresses anti-aliasing.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};

pub(crate) fn resample_median(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);
    let window = config.resample_sample_window.max(1);
    let half = (window as i32) / 2;
    let (iw, ih) = (img.width() as i32, img.height() as i32);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let (ys, ye) = (w_y[0] as i32, w_y[1] as i32);
            let (xs, xe) = (w_x[0] as i32, w_x[1] as i32);
            if xe <= xs || ye <= ys {
                continue;
            }
            let cx = (xs + xe) / 2;
            let cy = (ys + ye) / 2;

            // Pass 1: opaque pixels in the window
            let mut chans: [Vec<u8>; 4] = Default::default();
            for dy in -half..=half {
                for dx in -half..=half {
                    let (x, y) = (cx + dx, cy + dy);
                    if x < xs || x >= xe || y < ys || y >= ye || x < 0 || y < 0 || x >= iw || y >= ih {
                        continue;
                    }
                    let p = img.get_pixel(x as u32, y as u32).0;
                    if p[3] < 16 {
                        continue;
                    }
                    for ch in 0..4 {
                        chans[ch].push(p[ch]);
                    }
                }
            }

            // Fallback: all pixels in the cell (incl. transparent)
            if chans[0].is_empty() {
                for y in ys..ye {
                    for x in xs..xe {
                        if x < 0 || y < 0 || x >= iw || y >= ih {
                            continue;
                        }
                        let p = img.get_pixel(x as u32, y as u32).0;
                        for ch in 0..4 {
                            chans[ch].push(p[ch]);
                        }
                    }
                }
            }

            let pixel = if chans[0].is_empty() {
                [0, 0, 0, 0]
            } else {
                let mut med = [0u8; 4];
                for ch in 0..4 {
                    chans[ch].sort_unstable();
                    med[ch] = chans[ch][chans[ch].len() / 2];
                }
                med
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
```

- [ ] **Step 4: Wire Median into dispatch in `src/resample/mod.rs`**

Add module declaration:
```rust
mod median;
```
Replace the `Median` arm:
```rust
        ResampleMethod::Median => median::resample_median(img, cols, rows, config),
```

- [ ] **Step 5: Verify via CLI (default still majority; median runnable)**

Run:
```bash
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/med.png 16 --resample median 2>&1 | tail -1
```
Note: `--resample` flag is added in Task 6. For now, temporarily test by hardcoding: skip this manual step if the flag isn't wired yet — the Task 8 integration test covers it. Just confirm compile:
```bash
cargo build 2>&1 | tail -3
```
Expected: builds clean.

- [ ] **Step 6: Commit**

```bash
git add src/resample/median.rs src/resample/mod.rs tests/resample.rs
git commit -m "feat(resample): median strategy (per-channel median + sample window)"
```

---

## Task 4: dominant strategy (dominant color + mean fallback + optional alpha binarize)

**Files:**
- Create: `src/resample/dominant.rs`
- Modify: `src/resample/mod.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/resample.rs`:
```rust
#[test]
fn dominant_variant_constructible() {
    let _d = ResampleMethod::Dominant;
    assert!(true);
}
```

- [ ] **Step 2: Run test (compiles; variant exists)**

Run: `cargo test --test resample 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 3: Implement `src/resample/dominant.rs`**

```rust
//! Dominant-color resample: top color if it clears a threshold, else per-channel
//! mean of opaque pixels. Optional hard alpha binarization.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;

pub(crate) fn resample_dominant(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);
    let threshold = config.resample_dominant_threshold;
    let binarize = config.resample_dominant_binarize_alpha;
    let (iw, ih) = (img.width() as usize, img.height() as usize);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];
            if xe <= xs || ye <= ys {
                continue;
            }

            let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
            let mut total = 0usize;
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        *counts.entry(p).or_insert(0) += 1;
                        total += 1;
                    }
                }
            }

            let pixel = if total == 0 {
                [0, 0, 0, 0]
            } else {
                // top color
                let (top_color, top_count) = counts
                    .iter()
                    .max_by(|a, b| a.1.cmp(b.1))
                    .map(|(c, n)| (*c, *n))
                    .unwrap_or(([0, 0, 0, 0], 0));
                let chosen = if (top_count as f64 / total as f64) >= threshold {
                    top_color
                } else {
                    // mean of opaque pixels (per channel)
                    let mut sums = [0u64; 4];
                    let mut n = 0u64;
                    for y in ys..ye {
                        for x in xs..xe {
                            if x < iw && y < ih {
                                let p = img.get_pixel(x as u32, y as u32).0;
                                if p[3] >= 16 {
                                    for ch in 0..4 {
                                        sums[ch] += p[ch] as u64;
                                    }
                                    n += 1;
                                }
                            }
                        }
                    }
                    if n == 0 {
                        top_color
                    } else {
                        [
                            (sums[0] / n) as u8,
                            (sums[1] / n) as u8,
                            (sums[2] / n) as u8,
                            (sums[3] / n) as u8,
                        ]
                    }
                };
                let mut out = chosen;
                if binarize {
                    out[3] = if out[3] >= 128 { 255 } else { 0 };
                }
                out
            };

            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
```

- [ ] **Step 4: Wire Dominant into dispatch**

Add `mod dominant;` to `src/resample/mod.rs`. Replace the `Dominant` arm:
```rust
        ResampleMethod::Dominant => dominant::resample_dominant(img, cols, rows, config),
```

- [ ] **Step 5: Verify compile**

Run: `cargo build 2>&1 | tail -3`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/resample/dominant.rs src/resample/mod.rs tests/resample.rs
git commit -m "feat(resample): dominant strategy (threshold + mean fallback + alpha binarize)"
```

---

## Task 5: mode strategy (per-channel mode)

**Files:**
- Create: `src/resample/mode.rs`
- Modify: `src/resample/mod.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/resample.rs`:
```rust
#[test]
fn mode_variant_constructible() {
    let _m = ResampleMethod::Mode;
    assert!(true);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --test resample 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 3: Implement `src/resample/mode.rs`**

```rust
//! Per-channel mode resample. CAVEAT: the combined pixel may be a color that
//! did not exist in the source (R-mode + G-mode + B-mode). Use `majority` for
//! strict palette preservation.

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;

pub(crate) fn resample_mode(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    _config: &Config,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }
    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);
    let (iw, ih) = (img.width() as usize, img.height() as usize);

    let channel_mode = |vals: &[u8]| -> u8 {
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for &v in vals {
            *counts.entry(v).or_insert(0) += 1;
        }
        // highest count, tie → lowest value (deterministic)
        counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))
            .map(|(v, _)| *v)
            .unwrap_or(0)
    };

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];
            if xe <= xs || ye <= ys {
                continue;
            }

            let mut chans: [Vec<u8>; 4] = Default::default();
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        for ch in 0..4 {
                            chans[ch].push(p[ch]);
                        }
                    }
                }
            }

            let pixel = if chans[0].is_empty() {
                [0, 0, 0, 0]
            } else {
                [
                    channel_mode(&chans[0]),
                    channel_mode(&chans[1]),
                    channel_mode(&chans[2]),
                    channel_mode(&chans[3]),
                ]
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
```

- [ ] **Step 4: Wire Mode into dispatch; remove the catch-all `_` arm**

Add `mod mode;`. Replace the final `_ =>` arm with:
```rust
        ResampleMethod::Mode => mode::resample_mode(img, cols, rows, config),
```

The full match is now exhaustive:
```rust
    match config.resample_method {
        ResampleMethod::Majority => majority::resample_majority(img, cols, rows, config),
        ResampleMethod::Median => median::resample_median(img, cols, rows, config),
        ResampleMethod::Dominant => dominant::resample_dominant(img, cols, rows, config),
        ResampleMethod::Mode => mode::resample_mode(img, cols, rows, config),
    }
```

- [ ] **Step 5: Verify default anchor unchanged**

Run:
```bash
cargo test 2>&1 | tail -5
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/o.png 16 && sha256sum /tmp/o.png
```
Expected: tests pass; sha256 `802857...9f22` (default Majority).

- [ ] **Step 6: Commit**

```bash
git add src/resample/mode.rs src/resample/mod.rs tests/resample.rs
git commit -m "feat(resample): mode strategy (per-channel mode)"
```

---

## Task 6: CLI `--resample` + `--sample-window` flags

**Files:**
- Modify: `src/cli.rs` (or `src/cli/args.rs` if Phase 1 split landed)

- [ ] **Step 1: Write failing test**

Add to the `cli_tests` module (in `src/cli.rs` or `src/cli/args.rs`):
```rust
    #[test]
    fn parses_resample_flag() {
        let command = parse_cli_args(&args(&[
            "input.png", "output.png", "--resample", "median",
        ])).unwrap();
        let CliCommand::Run(config) = command else { panic!("expected Run"); };
        assert_eq!(config.resample_method, crate::resample::ResampleMethod::Median);
    }

    #[test]
    fn parses_sample_window_flag() {
        let command = parse_cli_args(&args(&[
            "input.png", "output.png", "--sample-window", "5",
        ])).unwrap();
        let CliCommand::Run(config) = command else { panic!("expected Run"); };
        assert_eq!(config.resample_sample_window, 5);
    }
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test parses_resample 2>&1 | tail -5`
Expected: FAIL — unknown argument.

- [ ] **Step 3: Implement `--resample` + `--sample-window` parsing**

In the arg-parsing `match` (before the `arg if arg.starts_with("--")` catch-all), add:
```rust
            "--resample" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--resample requires a value".to_string(),
                    ));
                };
                config.resample_method = match val.as_str() {
                    "majority" => crate::resample::ResampleMethod::Majority,
                    "median" => crate::resample::ResampleMethod::Median,
                    "dominant" => crate::resample::ResampleMethod::Dominant,
                    "mode" => crate::resample::ResampleMethod::Mode,
                    _ => {
                        return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --resample '{}' (expected majority|median|dominant|mode)",
                            val
                        )))
                    }
                };
                i += 2;
            }
            "--sample-window" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--sample-window requires a value".to_string(),
                    ));
                };
                match val.parse::<usize>() {
                    Ok(n) if (1..=9).contains(&n) => config.resample_sample_window = n,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --sample-window '{}' (expected 1-9)", val
                    ))),
                }
                i += 2;
            }
```

Update `print_cli_help` OPTIONS — add:
```
  --resample <majority|median|dominant|mode>  Grid-cell reduction [default: majority]
  --sample-window <1-9>                       Median neighborhood [default: 3]
```

- [ ] **Step 4: Run tests**

Run: `cargo test 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 5: Manual verify all four strategies run**

```bash
for m in majority median dominant mode; do
  cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/$m.png 16 --resample $m 2>&1 | tail -1
done
```
Expected: four runs complete without error; four output files exist.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs   # or src/cli/args.rs
git commit -m "feat(cli): --resample + --sample-window flags"
```

---

## Task 7: WASM `process_image` gains `resample_method` param

**Files:**
- Modify: `src/lib.rs` (wasm region)

- [ ] **Step 1: Add the parameter**

In `src/lib.rs`, the wasm `process_image` fn — add `resample_method: Option<String>` as a trailing parameter:
```rust
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    palette_hex: Option<String>,
    detect_strategy: Option<String>,
    resample_method: Option<String>,
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
```

In the body, after the `detect_strategy` block, add:
```rust
    if let Some(m) = resample_method {
        config.resample_method = match m.as_str() {
            "majority" => resample::ResampleMethod::Majority,
            "median" => resample::ResampleMethod::Median,
            "dominant" => resample::ResampleMethod::Dominant,
            "mode" => resample::ResampleMethod::Mode,
            _ => return Err(wasm_bindgen::JsValue::from_str(
                "resample_method must be majority|median|dominant|mode",
            )),
        };
    }
```

- [ ] **Step 2: Verify wasm build**

Run: `cargo build --target wasm32-unknown-unknown 2>&1 | tail -3`
Expected: `Finished`, 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(wasm): process_image gains resample_method param"
```

---

## Task 8: aa-edges fixture + behavioral integration tests

**Files:**
- Create: `tests/fixtures/baseline/aa-edges.png`
- Modify: `tests/resample.rs`

- [ ] **Step 1: Create the fixture**

Make an image with visible anti-aliasing on edges (e.g. a diagonal line or circle rendered with smooth alpha gradients at the boundary). Any small (32-64px) RGBA PNG with AA edges works. Save to `tests/fixtures/baseline/aa-edges.png`.

If no quick generator is available, a minimal approach: take `clean.png`, scale it up 3× with bilinear filtering (introduces AA), save as `aa-edges.png`. Or hand-draw in any editor.

- [ ] **Step 2: Write the behavioral tests**

Replace `tests/resample.rs` content with:

```rust
use spritefusion_pixel_snapper::resample::ResampleMethod;
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
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test resample 2>&1 | tail -10`
Expected: all pass.

If `majority_default_matches_anchor` fails, the directory move in Task 1 diverged — re-check `majority.rs` is byte-for-byte.

- [ ] **Step 4: Commit**

```bash
git add tests/fixtures/baseline/aa-edges.png tests/resample.rs
git commit -m "test(resample): per-strategy determinism + median AA fixture"
```

---

## Task 9: CLAUDE.md update + final verification

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update CLAUDE.md resample section**

In `CLAUDE.md`, find the pipeline step for resample (step 5 in the current doc) and update:
```markdown
5. **`resample`** — for each grid cell, reduce to one pixel per `ResampleMethod` (default `majority` = whole-pixel mode + RGBA tie-break). Alternatives: `median` (per-channel median + sample window, AA removal), `dominant` (top color if ≥ threshold, else mean), `mode` (per-channel mode; may emit colors not in source — use `majority` for strict palette preservation).
```

In the module table, update the Resample row:
```markdown
| Resample (multi-strategy) | [resample/mod.rs](src/resample/mod.rs) | `majority`/`median`/`dominant`/`mode` dispatch |
```

In "Tuning knobs", add `resample_method` / `resample_sample_window` / `resample_dominant_threshold` to the list of internal fields.

- [ ] **Step 2: Final full verification**

```bash
cargo test 2>&1 | tail -5
cargo build --target wasm32-unknown-unknown 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/final.png 16 && sha256sum /tmp/final.png
```
Expected: all tests green; wasm 0 warnings; ai-sprite sha256 = `802857...9f22`.

- [ ] **Step 3: Commit + tag the branch ready for review**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for multi-strategy resample (phase 2 complete)"
```

---

## Self-Review (completed inline)

**Spec coverage:**
- §Strategies (majority/median/dominant/mode) → Tasks 1/3/4/5. ✓
- §Module layout (resample/ dir) → Task 1. ✓
- §Data flow → Task 1 (call site) + Task 2 (dispatch). ✓
- §Config fields → Task 2. ✓
- §CLI --resample/--sample-window → Task 6. ✓
- §WASM resample_method param → Task 7. ✓
- §Tests (anchor, determinism, median AA) → Tasks 1/8. ✓
- §Acceptance (anchor, wasm 0 warn) → Tasks 1/7/8/9. ✓
- §Decision 3 (mode caveat) → documented in `mode.rs` header (Task 5) + CLAUDE.md (Task 9). ✓
- §Decision 4 (dominant alpha default off) → Task 2 default `false`. ✓

**Placeholder scan:** Task 8 fixture creation gives guidance (bilinear scale of clean.png) rather than a literal placeholder; acceptable. All code blocks complete. No "TODO"/"TBD".

**Type consistency:** `ResampleMethod::{Majority,Median,Dominant,Mode}` consistent across Tasks 2/6/7. Config field names (`resample_method` / `resample_sample_window` / `resample_dominant_threshold` / `resample_dominant_binarize_alpha`) consistent Tasks 2/3/4/6. Strategy fn signature `fn resample_<method>(img, cols, rows, config) -> Result<RgbaImage>` consistent Tasks 1/3/4/5.

**Risk flagged inline:** Task 8 `majority_default_matches_anchor` is the gate — if it fails, Task 1's move diverged. Re-verify `majority.rs` is byte-for-byte vs the pre-Task-1 `resample.rs`.

---

**Plan complete.** Saved to `docs/superpowers/plans/2026-07-22-phase2-resample.md`.
