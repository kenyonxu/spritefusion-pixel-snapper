# Phase 4 — Post-processing Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional post-process stage (background removal, outline, morphology, alpha binarize) to the pixel-game-kit pipeline, off by default so existing anchors are unchanged.

**Architecture:** New `src/postprocess/` module mirroring `resample/` — one file per op + `mod.rs` fixed-order dispatch. Four independent on/off toggles on the flat `Config` (prefix `post_*`). Pipeline call inserted in `process_image_common` after `apply_palette`. CLI gets 9 independent flags; WASM gets one `post_config` JSON param (Option B).

**Tech Stack:** Rust, `image` crate (existing), `serde`+`serde_json` (new, wasm-target-only for the JSON parse). All ops are RNG-free → determinism (R1) is trivial.

**Spec:** [docs/superpowers/specs/2026-07-24-phase4-postprocess-design.md](../specs/2026-07-24-phase4-postprocess-design.md)

---

## File Structure

**Create:**
- `src/postprocess/mod.rs` — 4 enums + `postprocess()` dispatch
- `src/postprocess/alpha.rs` — `binarize_alpha` + `otsu_threshold`
- `src/postprocess/morphology.rs` — `morph_open_close` (alpha-only)
- `src/postprocess/outline.rs` — `apply_outline`
- `src/postprocess/floodfill.rs` — `remove_background` + `remove_small_floating_components`
- `tests/postprocess.rs` — integration tests (anchor lock, determinism, multi-op)

**Modify:**
- `src/lib.rs` — `mod postprocess;` + call in `process_image_common` + final-dim reporting in `ProcessedImage` + wasm `post_config` param
- `src/config.rs` — 9 `post_*` fields + defaults + enum imports
- `src/cli/args.rs` — parse 9 flags + `--help` text
- `CLAUDE.md` — pipeline table + postprocess stage row
- `PLAN.md` — tick Phase 4 checkboxes on completion

**Testing strategy:** inline `#[cfg(test)] mod tests` in each postprocess module (fast, synthetic tiny images — no binary fixtures needed); `tests/postprocess.rs` for end-to-end CLI/anchor/determinism. Synthetic images are built in-test and written to `std::env::temp_dir()` (mirrors `tests/resample.rs` cross-platform pattern). The default-anchor lock reuses `tests/fixtures/baseline/ai-sprite.png` (default config → all post flags off → output unchanged).

**Zero-regression gate (every task):** all `post_*` defaults are off, so after each task `cargo test` must stay green and the Oklab anchor `3a589ee93b8cd2e493baa0d6fb314d279b54a1104165ad754ad4ff6d359e4420` (ai-sprite, default config) must be unchanged.

---

### Task 1: Config enums + postprocess module skeleton (zero-behavior)

Add the 4 enums and 9 config fields, plus a stub `postprocess()` that returns the image unchanged. Nothing calls it yet, so behavior is byte-identical.

**Files:**
- Create: `src/postprocess/mod.rs`
- Modify: `src/config.rs`

- [ ] **Step 1: Create `src/postprocess/mod.rs` with enums + stub**

```rust
//! Post-processing stage: bg removal, outline, morphology, alpha binarize.
//! See `Config.post_*`. All ops off by default -> zero behavior change.
//!
//! NOTE: sub-module declarations (`mod alpha;`, `mod morphology;`, etc.) are
//! added incrementally in Tasks 3-7 as each op file is created. Declaring them
//! here would fail to compile (files don't exist yet).

use crate::Config;
use image::RgbaImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgConnectivity {
    Conn4,
    Conn8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgScope {
    Outer,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineStyle {
    None,
    Rounded,
    Sharp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaThreshold {
    None,
    Fixed(u8),
    Auto,
}

/// Run enabled postprocess ops in fixed order:
/// flood-fill -> floating cleanup -> morphology -> alpha binarize -> outline.
/// Each op is gated by its own config flag; ops are infallible (pure pixel ops
/// on a validated, non-empty image).
pub fn postprocess(img: RgbaImage, _config: &Config) -> RgbaImage {
    // Stub: Task 2 wires this into the pipeline; Tasks 3-8 fill in the ops.
    img
}
```

- [ ] **Step 2: Add `mod postprocess;` to `src/lib.rs`**

In `src/lib.rs`, add to the module declarations near the others (after `mod palette;`):

```rust
mod postprocess;
```

- [ ] **Step 3: Add the 9 `post_*` fields to `src/config.rs`**

Add the import at the top of `src/config.rs` (next to the existing `use crate::...` lines):

```rust
use crate::postprocess::{AlphaThreshold, BgConnectivity, BgScope, OutlineStyle};
```

Add these fields to the `Config` struct (after `quantize_preset_palette`):

```rust
    pub(crate) post_bg_remove: bool,
    pub(crate) post_bg_tolerance: u8,
    pub(crate) post_bg_connectivity: BgConnectivity,
    pub(crate) post_bg_scope: BgScope,
    pub(crate) post_bg_floating_max_pixels: usize,
    pub(crate) post_outline: OutlineStyle,
    pub(crate) post_outline_color: [u8; 3],
    pub(crate) post_morph: bool,
    pub(crate) post_alpha_threshold: AlphaThreshold,
```

Add the defaults to `impl Default for Config` (after `quantize_preset_palette: PresetPalette::None,`):

```rust
            post_bg_remove: false,
            post_bg_tolerance: 64,
            post_bg_connectivity: BgConnectivity::Conn4,
            post_bg_scope: BgScope::Outer,
            post_bg_floating_max_pixels: 0,
            post_outline: OutlineStyle::None,
            post_outline_color: [0, 0, 0],
            post_morph: false,
            post_alpha_threshold: AlphaThreshold::None,
```

- [ ] **Step 4: Verify compile + tests green + wasm builds**

Run: `cargo test`
Expected: all existing tests pass (stub is unused dead code; may emit a `dead_code` warning for `postprocess` — that is fine, Task 2 uses it).

Run: `cargo build --target wasm32-unknown-unknown`
Expected: builds with 0 warnings (or only the expected `dead_code` on the stub).

- [ ] **Step 5: Commit**

```bash
git add src/postprocess/mod.rs src/lib.rs src/config.rs
git commit -m "feat(phase4): postprocess enums + config fields (stub, zero-behavior)"
```

---

### Task 2: Wire postprocess into the pipeline (still zero-behavior)

Call `postprocess()` after `apply_palette`, report final dims in `ProcessedImage`. The stub returns the image unchanged, so output is byte-identical.

**Files:**
- Modify: `src/lib.rs:137-158` (the tail of `process_image_common`)

- [ ] **Step 1: Insert the postprocess call + capture final dims**

In `src/lib.rs`, replace the tail of `process_image_common` (from the `apply_palette` match through the `Ok(ProcessedImage { ... })`):

```rust
    let snapped_img = resample::resample(&analysis_img, &col_cuts, &row_cuts, &config)?;
    let palette_img = match config.palette.as_deref() {
        Some(palette) => apply_palette(&snapped_img, palette)?,
        None => snapped_img,
    };
    let output_img = postprocess::postprocess(palette_img, &config);
    let (out_w, out_h) = output_img.dimensions();

    // Returns bytes for both implementations
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    output_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(PixelSnapperError::ImageError)?;

    Ok(ProcessedImage {
        output_bytes,
        pixel_size: chosen.step,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: out_w,
        output_height: out_h,
        selected_detector,
        candidates,
    })
```

- [ ] **Step 2: Verify anchor unchanged**

Run: `cargo test`
Expected: all green.

Run the anchor check (matches `tests/resample.rs::majority_default_matches_anchor`):
```bash
cargo run --release -- tests/fixtures/baseline/ai-sprite.png target/phase4-anchor-check.png 16
sha256sum target/phase4-anchor-check.png
```
Expected: `3a589ee93b8cd2e493baa0d6fb314d279b54a1104165ad754ad4ff6d359e4420` (unchanged — stub is a no-op).

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(phase4): wire postprocess into pipeline + report final dims"
```

---

### Task 3: alpha binarize (Fixed + Otsu)

Implement `binarize_alpha` with fixed strict `>` threshold and Otsu adaptive mode. Spec §alpha binarize.

**Files:**
- Modify: `src/postprocess/alpha.rs` (created as empty file by Task 1's `mod alpha;`)

- [ ] **Step 1: Write the failing test at the bottom of `src/postprocess/alpha.rs`**

```rust
//! Alpha binarization: fixed strict threshold or Otsu adaptive.

use crate::postprocess::AlphaThreshold;
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};

pub fn binarize_alpha(img: RgbaImage, config: &Config) -> RgbaImage {
    let threshold: u8 = match config.post_alpha_threshold {
        AlphaThreshold::None => return img,
        AlphaThreshold::Fixed(t) => t,
        AlphaThreshold::Auto => otsu_threshold(&img).unwrap_or(128),
    };
    let mut out = img;
    for p in out.pixels_mut() {
        p[3] = if p[3] > threshold { 255 } else { 0 };
    }
    out
}

/// Classic Otsu on the alpha-channel histogram. Returns None when the image
/// is empty or the best threshold is degenerate (0 or 255 -> single peak).
fn otsu_threshold(img: &RgbaImage) -> Option<u8> {
    let mut hist = [0u32; 256];
    for p in img.pixels() {
        hist[p[3] as usize] += 1;
    }
    let total = (img.width() as usize * img.height() as usize) as f64;
    if total == 0.0 {
        return None;
    }
    let mut sum: u64 = 0;
    for i in 0..256 {
        sum += (i as u64) * (hist[i] as u64);
    }
    let mut sum_b: u64 = 0;
    let mut w_b: u32 = 0;
    let mut max_var: f64 = 0.0;
    let mut threshold: u8 = 0;
    for t in 0..256u32 {
        w_b += hist[t as usize];
        if w_b == 0 {
            continue;
        }
        let w_f = (total as u32) - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += (t as u64) * (hist[t as usize] as u64);
        let m_b = sum_b as f64 / w_b as f64;
        let m_f = (sum as f64 - sum_b as f64) / w_f as f64;
        let var = w_b as f64 * w_f as f64 * (m_b - m_f).powi(2);
        if var > max_var {
            max_var = var;
            threshold = t as u8;
        }
    }
    if threshold == 0 || threshold == 255 {
        None
    } else {
        Some(threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img4(a: [u8; 4]) -> RgbaImage {
        ImageBuffer::from_pixel(2, 2, Rgba(a))
    }

    fn config_with_alpha(t: AlphaThreshold) -> Config {
        let mut c = Config::default();
        c.post_alpha_threshold = t;
        c
    }

    #[test]
    fn fixed_threshold_strict_greater() {
        // alpha 128 with threshold 128 -> strict > -> maps to 0
        let c = config_with_alpha(AlphaThreshold::Fixed(128));
        let out = binarize_alpha(img4([10, 20, 30, 128]), &c);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        // alpha 129 -> 255
        let out = binarize_alpha(img4([10, 20, 30, 129]), &c);
        assert_eq!(out.get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn none_is_noop() {
        let c = config_with_alpha(AlphaThreshold::None);
        let out = binarize_alpha(img4([10, 20, 30, 200]), &c);
        assert_eq!(out.get_pixel(0, 0)[3], 200);
    }

    #[test]
    fn rgb_preserved() {
        let c = config_with_alpha(AlphaThreshold::Fixed(128));
        let out = binarize_alpha(img4([10, 20, 30, 200]), &c);
        let p = out.get_pixel(0, 0);
        assert_eq!([p[0], p[1], p[2]], [10, 20, 30]);
    }

    #[test]
    fn otsu_bimodal_picks_intermediate() {
        // half opaque (255), half semi (64) -> bimodal, threshold between
        let mut img: RgbaImage = ImageBuffer::new(2, 1);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 0, 0, 64]));
        let t = otsu_threshold(&img).expect("bimodal should yield a threshold");
        assert!(t > 64 && t < 255);
    }

    #[test]
    fn otsu_single_peak_returns_none() {
        // all opaque -> single peak -> degenerate -> None -> fallback 128
        let img = img4([0, 0, 0, 255]);
        assert_eq!(otsu_threshold(&img), None);
    }

    #[test]
    fn auto_falls_back_to_128_on_single_peak() {
        let c = config_with_alpha(AlphaThreshold::Auto);
        // all opaque 255 > 128 -> 255 (fallback path exercised, no panic)
        let out = binarize_alpha(img4([0, 0, 0, 255]), &c);
        assert_eq!(out.get_pixel(0, 0)[3], 255);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail (then pass — impl is inline above)**

Run: `cargo test postprocess::alpha`
Expected: 6 tests pass (implementation is already complete above; if you wrote the test first against an empty file, it would fail to compile — the inline impl makes it pass).

Note: this task ships impl + test together because the function is small. The TDD discipline is preserved by the assertions encoding the spec (strict `>`, None no-op, RGB preserved, Otsu bimodal, single-peak fallback).

- [ ] **Step 3: Wire `AlphaThreshold::None`/`Auto`/`Fixed` dispatch into the postprocess entry**

In `src/postprocess/mod.rs`, add the sub-module declaration after the doc comment (first one — the file didn't declare any sub-modules in Task 1):

```rust
mod alpha;
```

Then replace the `postprocess` body:

```rust
pub fn postprocess(img: RgbaImage, config: &Config) -> RgbaImage {
    let mut img = img;
    if !matches!(config.post_alpha_threshold, AlphaThreshold::None) {
        img = alpha::binarize_alpha(img, config);
    }
    img
}
```

- [ ] **Step 4: Verify anchor unchanged + new tests pass**

Run: `cargo test`
Expected: all green (anchor unchanged — alpha off by default).

- [ ] **Step 5: Commit**

```bash
git add src/postprocess/alpha.rs src/postprocess/mod.rs
git commit -m "feat(phase4): alpha binarize (fixed strict + Otsu auto)"
```

---

### Task 4: morphology (alpha-only open→close)

Implement 2×2 open→close on alpha only; RGB untouched (palette-fidelity). Spec §morphology.

**Files:**
- Modify: `src/postprocess/morphology.rs`

- [ ] **Step 1: Write impl + failing/passing tests in `src/postprocess/morphology.rs`**

```rust
//! 2x2 open->close morphology, alpha-only (palette-preserving).
//! Semantics from unfake.js morphology.rs (clean-room): 2x2 kernel, top-left
//! anchor, replicate border. open = dilate(erode); close = erode(dilate).

use image::RgbaImage;

pub fn morph_open_close(img: RgbaImage) -> RgbaImage {
    let opened = dilate_alpha(&erode_alpha(&img));
    let closed = erode_alpha(&dilate_alpha(&opened));
    closed
}

fn erode_alpha(img: &RgbaImage) -> RgbaImage {
    op_alpha(img, u8::min, 255)
}

fn dilate_alpha(img: &RgbaImage) -> RgbaImage {
    op_alpha(img, u8::max, 0)
}

/// Apply `combine` over the 2x2 window (top-left anchored, replicate border)
/// of the alpha channel. RGB is copied unchanged.
fn op_alpha<F: Fn(u8, u8) -> u8>(img: &RgbaImage, combine: F, init: u8) -> RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut out = img.clone();
    for y in 0..h {
        for x in 0..w {
            let mut acc = init;
            for dy in 0..=1usize {
                for dx in 0..=1usize {
                    let xx = (x + dx).min(w - 1);
                    let yy = (y + dy).min(h - 1);
                    let a = img.get_pixel(xx as u32, yy as u32)[3];
                    acc = combine(acc, a);
                }
            }
            out.get_pixel_mut(x as u32, y as u32)[3] = acc;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_removes_isolated_transparent_speckle() {
        // 3x3 fully opaque, single transparent hole at center.
        // open = dilate(erode): erode shrinks opaque region (hole grows),
        // dilate regrows -> the 1px hole is filled.
        let mut img: RgbaImage = image::ImageBuffer::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                img.put_pixel(x, y, image::Rgba([10, 20, 30, 255]));
            }
        }
        img.put_pixel(1, 1, image::Rgba([10, 20, 30, 0]));
        let out = morph_open_close(img);
        assert_eq!(out.get_pixel(1, 1)[3], 255, "1px hole should be filled by open");
    }

    #[test]
    fn rgb_is_preserved() {
        let mut img: RgbaImage = image::ImageBuffer::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([11, 22, 33, 255]));
        img.put_pixel(1, 0, image::Rgba([44, 55, 66, 255]));
        img.put_pixel(0, 1, image::Rgba([77, 88, 99, 255]));
        img.put_pixel(1, 1, image::Rgba([100, 110, 120, 0]));
        let out = morph_open_close(img);
        // every pixel keeps its original RGB (morph is alpha-only)
        assert_eq!([out.get_pixel(0, 0)[0], out.get_pixel(0, 0)[1], out.get_pixel(0, 0)[2]], [11, 22, 33]);
        assert_eq!([out.get_pixel(1, 1)[0], out.get_pixel(1, 1)[1], out.get_pixel(1, 1)[2]], [100, 110, 120]);
    }

    #[test]
    fn open_removes_single_opaque_speckle() {
        // fully transparent, single opaque speckle -> open removes it
        let mut img: RgbaImage = image::ImageBuffer::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
        img.put_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
        let out = morph_open_close(img);
        assert_eq!(out.get_pixel(1, 1)[3], 0, "1px speckle should be removed by open");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test postprocess::morphology`
Expected: 3 pass.

- [ ] **Step 3: Wire morph into the postprocess entry**

Add `mod morphology;` to the top of `src/postprocess/mod.rs` (next to `mod alpha;`).

In `src/postprocess/mod.rs` `postprocess()`, add (after the alpha block, before the final `img`):

```rust
    if config.post_morph {
        img = morphology::morph_open_close(img);
    }
```

- [ ] **Step 4: Verify anchor unchanged**

Run: `cargo test`
Expected: all green (morph off by default).

- [ ] **Step 5: Commit**

```bash
git add src/postprocess/morphology.rs src/postprocess/mod.rs
git commit -m "feat(phase4): alpha-only 2x2 open->close morphology"
```

---

### Task 5: outline (pad +1/side, 4/8-way)

Implement 1px outline. Spec §outline.

**Files:**
- Modify: `src/postprocess/outline.rs`

- [ ] **Step 1: Write impl + tests in `src/postprocess/outline.rs`**

```rust
//! 1px outline: pad canvas +1px per side, draw into transparent pixels
//! adjacent to opaque. Semantics from PixelRefiner outline.ts (clean-room).

use crate::postprocess::OutlineStyle;
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};

pub fn apply_outline(img: RgbaImage, config: &Config) -> RgbaImage {
    if matches!(config.post_outline, OutlineStyle::None) {
        return img;
    }
    let (w, h) = (img.width() as usize, img.height() as usize);
    let dw = (w + 2) as u32;
    let dh = (h + 2) as u32;
    let mut out: RgbaImage = ImageBuffer::new(dw, dh);
    let [r, g, b] = config.post_outline_color;

    // copy original to offset (1,1)
    for y in 0..h {
        for x in 0..w {
            let p = *img.get_pixel(x as u32, y as u32);
            out.put_pixel(x as u32 + 1, y as u32 + 1, p);
        }
    }

    let neighbors: &[(i32, i32)] = match config.post_outline {
        OutlineStyle::Sharp => &[(0, -1), (0, 1), (-1, 0), (1, 0)],
        OutlineStyle::Rounded => &[
            (0, -1), (0, 1), (-1, 0), (1, 0),
            (-1, -1), (1, -1), (-1, 1), (1, 1),
        ],
        OutlineStyle::None => &[],
    };

    // For each destination transparent pixel, check source-space neighbors.
    // dst (dx,dy) maps to source (dx-1, dy-1); neighbor offsets are source-space.
    for dy in 0..(dh as usize) {
        for dx in 0..(dw as usize) {
            if out.get_pixel(dx as u32, dy as u32)[3] != 0 {
                continue;
            }
            let sx = dx as i32 - 1;
            let sy = dy as i32 - 1;
            let mut draw = false;
            for &(nx, ny) in neighbors {
                let xx = sx + nx;
                let yy = sy + ny;
                if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                    if img.get_pixel(xx as u32, yy as u32)[3] > 0 {
                        draw = true;
                        break;
                    }
                }
            }
            if draw {
                out.put_pixel(dx as u32, dy as u32, Rgba([r, g, b, 255]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_opaque_pixel() -> RgbaImage {
        let mut img: RgbaImage = ImageBuffer::new(1, 1);
        img.put_pixel(0, 0, Rgba([200, 200, 200, 255]));
        img
    }

    fn config_outline(style: OutlineStyle, color: [u8; 3]) -> Config {
        let mut c = Config::default();
        c.post_outline = style;
        c.post_outline_color = color;
        c
    }

    #[test]
    fn dims_grow_by_two() {
        let c = config_outline(OutlineStyle::Sharp, [0, 0, 0]);
        let out = apply_outline(single_opaque_pixel(), &c);
        assert_eq!(out.dimensions(), (3, 3));
    }

    #[test]
    fn sharp_draws_four_neighbors_in_default_color() {
        let c = config_outline(OutlineStyle::Sharp, [0, 0, 0]);
        let out = apply_outline(single_opaque_pixel(), &c);
        // center opaque pixel preserved
        assert_eq!(out.get_pixel(1, 1)[3], 255);
        // 4 cardinal neighbors filled black, diagonals stay transparent
        assert_eq!(out.get_pixel(1, 0), &Rgba([0, 0, 0, 255]));
        assert_eq!(out.get_pixel(0, 1), &Rgba([0, 0, 0, 255]));
        assert_eq!(out.get_pixel(2, 1), &Rgba([0, 0, 0, 255]));
        assert_eq!(out.get_pixel(1, 2), &Rgba([0, 0, 0, 255]));
        assert_eq!(out.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn rounded_draws_eight_neighbors() {
        let c = config_outline(OutlineStyle::Rounded, [255, 0, 0]);
        let out = apply_outline(single_opaque_pixel(), &c);
        // all 8 surrounding pixels filled red
        for &dy in &[0usize, 1, 2] {
            for &dx in &[0usize, 1, 2] {
                if dx == 1 && dy == 1 {
                    continue;
                }
                assert_eq!(out.get_pixel(dx as u32, dy as u32), &Rgba([255, 0, 0, 255]));
            }
        }
    }

    #[test]
    fn custom_color_respected() {
        let c = config_outline(OutlineStyle::Sharp, [12, 34, 56]);
        let out = apply_outline(single_opaque_pixel(), &c);
        assert_eq!(out.get_pixel(1, 0), &Rgba([12, 34, 56, 255]));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test postprocess::outline`
Expected: 4 pass.

- [ ] **Step 3: Wire outline into the postprocess entry (LAST in order)**

Add `mod outline;` to the top of `src/postprocess/mod.rs` (next to `mod morphology;`).

In `src/postprocess/mod.rs` `postprocess()`, add at the end (outline is last because it grows the canvas):

```rust
    if !matches!(config.post_outline, OutlineStyle::None) {
        img = outline::apply_outline(img, config);
    }
```

- [ ] **Step 4: Verify anchor unchanged**

Run: `cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src/postprocess/outline.rs src/postprocess/mod.rs
git commit -m "feat(phase4): 1px outline (sharp=4-way, rounded=8-way)"
```

---

### Task 6: flood-fill background removal (Outer + All)

Implement `remove_background` with stack-based flood-fill. Spec §flood-fill.

**Refinement of spec §All scope:** `All` erases every opaque pixel whose color is within tolerance of any **distinct opaque border color** (auto-derived target set — no user-supplied bgTargets needed). This avoids the source's "erase everything" degeneracy and is the documented aggressive mode. `Outer` flood-fills from each border pixel using that pixel's own color as the seed.

**Files:**
- Modify: `src/postprocess/floodfill.rs`

- [ ] **Step 1: Write impl + tests in `src/postprocess/floodfill.rs`**

```rust
//! Background flood-fill + floating-island cleanup.
//! Semantics from PixelRefiner floodfill.ts + processor.ts (clean-room).

use crate::postprocess::{BgConnectivity, BgScope};
use crate::Config;
use image::RgbaImage;

const NEIGHBORS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const NEIGHBORS_8: [(i32, i32); 8] = [
    (-1, 0), (1, 0), (0, -1), (0, 1),
    (-1, -1), (-1, 1), (1, -1), (1, 1),
];

fn neighbors_for(c: BgConnectivity) -> &'static [(i32, i32)] {
    match c {
        BgConnectivity::Conn4 => &NEIGHBORS_4,
        BgConnectivity::Conn8 => &NEIGHBORS_8,
    }
}

fn within_tolerance(p: &[u8; 4], seed: &[u8; 3], tol: u8) -> bool {
    (p[0] as i32 - seed[0] as i32).abs() <= tol as i32
        && (p[1] as i32 - seed[1] as i32).abs() <= tol as i32
        && (p[2] as i32 - seed[2] as i32).abs() <= tol as i32
}

/// Iterative stack-based flood-fill from (sx, sy). Zeroes alpha of connected
/// opaque pixels within `tol` (per-channel) of the seed pixel's RGB.
fn flood_fill(
    img: &mut RgbaImage,
    visited: &mut [bool],
    w: usize,
    h: usize,
    sx: usize,
    sy: usize,
    tol: u8,
    conn: BgConnectivity,
) {
    let seed = {
        let p = *img.get_pixel(sx as u32, sy as u32);
        if p[3] == 0 {
            return; // transparent seed -> no-op
        }
        [p[0], p[1], p[2]]
    };
    let nbrs = neighbors_for(conn);
    let mut stack = vec![(sx, sy)];
    while let Some((x, y)) = stack.pop() {
        if x >= w || y >= h {
            continue;
        }
        let idx = y * w + x;
        if visited[idx] {
            continue;
        }
        let p = *img.get_pixel(x as u32, y as u32);
        if p[3] == 0 {
            continue;
        }
        if !within_tolerance(&p.0, &seed, tol) {
            continue;
        }
        visited[idx] = true;
        img.get_pixel_mut(x as u32, y as u32)[3] = 0;
        for &(nx, ny) in nbrs {
            let xx = x as i32 + nx;
            let yy = y as i32 + ny;
            if xx >= 0 && yy >= 0 {
                stack.push((xx as usize, yy as usize));
            }
        }
    }
}

/// Distinct opaque colors touching the image border (the auto bg-target set).
fn border_colors(img: &RgbaImage, w: usize, h: usize) -> Vec<[u8; 3]> {
    let mut set = std::collections::HashSet::new();
    for x in 0..w {
        for &y in &[0usize, h - 1] {
            let p = img.get_pixel(x as u32, y as u32);
            if p[3] != 0 {
                set.insert([p[0], p[1], p[2]]);
            }
        }
    }
    for y in 0..h {
        for &x in &[0usize, w - 1] {
            let p = img.get_pixel(x as u32, y as u32);
            if p[3] != 0 {
                set.insert([p[0], p[1], p[2]]);
            }
        }
    }
    set.into_iter().collect()
}

pub fn remove_background(img: RgbaImage, config: &Config) -> RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut out = img;
    let mut visited = vec![false; w * h];
    let tol = config.post_bg_tolerance;
    let conn = config.post_bg_connectivity;

    match config.post_bg_scope {
        BgScope::Outer => {
            // flood from every border pixel (uses each seed's own color)
            for x in 0..w {
                flood_fill(&mut out, &mut visited, w, h, x, 0, tol, conn);
                flood_fill(&mut out, &mut visited, w, h, x, h - 1, tol, conn);
            }
            for y in 0..h {
                flood_fill(&mut out, &mut visited, w, h, 0, y, tol, conn);
                flood_fill(&mut out, &mut visited, w, h, w - 1, y, tol, conn);
            }
        }
        BgScope::All => {
            let targets = border_colors(&out, w, h);
            for y in 0..h {
                for x in 0..w {
                    let p = *out.get_pixel(x as u32, y as u32);
                    if p[3] == 0 {
                        continue;
                    }
                    if targets.iter().any(|c| within_tolerance(&p.0, c, tol)) {
                        out.get_pixel_mut(x as u32, y as u32)[3] = 0;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    // 3x3: white border (bg) + red center (subject)
    fn sprite_on_white_bg() -> RgbaImage {
        let mut img: RgbaImage = ImageBuffer::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        img.put_pixel(1, 1, Rgba([255, 0, 0, 255]));
        img
    }

    fn config_bg(scope: BgScope, tol: u8) -> Config {
        let mut c = Config::default();
        c.post_bg_remove = true;
        c.post_bg_scope = scope;
        c.post_bg_tolerance = tol;
        c.post_bg_connectivity = BgConnectivity::Conn4;
        c
    }

    #[test]
    fn outer_removes_bg_keeps_subject() {
        let out = remove_background(sprite_on_white_bg(), &config_bg(BgScope::Outer, 0));
        assert_eq!(out.get_pixel(1, 1)[3], 255, "subject survives");
        assert_eq!(out.get_pixel(0, 0)[3], 0, "corner bg removed");
    }

    #[test]
    fn outer_subject_not_touching_border_survives() {
        // subject center is opaque, all bg around it removed
        let out = remove_background(sprite_on_white_bg(), &config_bg(BgScope::Outer, 0));
        for &(x, y) in &[(0u32, 0), (2, 0), (0, 2), (2, 2), (1, 0), (0, 1)] {
            assert_eq!(out.get_pixel(x, y)[3], 0);
        }
    }

    #[test]
    fn tolerance_widens_match() {
        // bg 250,250,250; tol 10 should treat it as ~white and remove
        let mut img: RgbaImage = ImageBuffer::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                img.put_pixel(x, y, Rgba([250, 250, 250, 255]));
            }
        }
        img.put_pixel(1, 1, Rgba([0, 0, 0, 255]));
        let out = remove_background(img, &config_bg(BgScope::All, 10));
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(1, 1)[3], 255);
    }

    #[test]
    fn all_strips_interior_bg_pocket() {
        // subject ring of red around a white interior pocket touching border via color
        let mut img: RgbaImage = ImageBuffer::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255])); // red everywhere
            }
        }
        img.put_pixel(1, 1, Rgba([255, 255, 255, 255])); // white pocket (interior, same color as border... none)
        // make border white so white is a bg-target, then All strips the interior white too
        img.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        let out = remove_background(img, &config_bg(BgScope::All, 0));
        assert_eq!(out.get_pixel(1, 1)[3], 0, "interior white pocket stripped by All");
        assert_eq!(out.get_pixel(1, 0)[3], 255, "red not in border-targets survives");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test postprocess::floodfill`
Expected: 4 pass.

- [ ] **Step 3: Wire bg-remove into the postprocess entry (FIRST in order)**

Add `mod floodfill;` to the top of `src/postprocess/mod.rs` (next to `mod outline;`).

In `src/postprocess/mod.rs` `postprocess()`, add at the very start (before the alpha block):

```rust
    if config.post_bg_remove {
        img = floodfill::remove_background(img, config);
    }
```

- [ ] **Step 4: Verify anchor unchanged**

Run: `cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src/postprocess/floodfill.rs src/postprocess/mod.rs
git commit -m "feat(phase4): background flood-fill removal (Outer + All scopes)"
```

---

### Task 7: floating-island cleanup

Implement `remove_small_floating_components` (4-conn CCL, largest survives). Spec §floating-island.

**Files:**
- Modify: `src/postprocess/floodfill.rs` (append)

- [ ] **Step 1: Append impl + tests to `src/postprocess/floodfill.rs`**

Add after the `remove_background` function (and extend the `tests` module):

```rust
pub fn remove_small_floating_components(img: RgbaImage, config: &Config) -> RgbaImage {
    let max_pixels = config.post_bg_floating_max_pixels;
    if max_pixels == 0 {
        return img;
    }
    let alpha_gate: u8 = 16;
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut out = img;
    let n = w * h;

    let mut visited = vec![false; n];
    let mut component_of: Vec<u32> = vec![u32::MAX; n];
    let mut sizes: Vec<usize> = Vec::new();
    let mut largest_id = 0u32;
    let mut largest_size = 0usize;

    for start in 0..n {
        if visited[start] {
            continue;
        }
        let sx = start % w;
        let sy = start / w;
        let p = *out.get_pixel(sx as u32, sy as u32);
        if p[3] < alpha_gate {
            visited[start] = true;
            continue;
        }
        let comp_id = sizes.len() as u32;
        let mut size = 0usize;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        while let Some(idx) = queue.pop_front() {
            component_of[idx] = comp_id;
            size += 1;
            let x = idx % w;
            let y = idx / w;
            for &(nx, ny) in &NEIGHBORS_4 {
                let xx = x as i32 + nx;
                let yy = y as i32 + ny;
                if xx < 0 || yy < 0 || xx as usize >= w || yy as usize >= h {
                    continue;
                }
                let nidx = (yy as usize) * w + (xx as usize);
                if visited[nidx] {
                    continue;
                }
                let np = *out.get_pixel(xx as u32, yy as u32);
                if np[3] < alpha_gate {
                    continue;
                }
                visited[nidx] = true;
                queue.push_back(nidx);
            }
        }
        sizes.push(size);
        if size > largest_size {
            largest_size = size;
            largest_id = comp_id;
        }
    }

    for idx in 0..n {
        let comp = component_of[idx];
        if comp == u32::MAX {
            continue;
        }
        if comp != largest_id && sizes[comp as usize] <= max_pixels {
            let x = (idx % w) as u32;
            let y = (idx / w) as u32;
            out.get_pixel_mut(x, y)[3] = 0;
        }
    }
    out
}
```

Add these tests inside the existing `mod tests` in `floodfill.rs`:

```rust
    fn config_floating(max_pixels: usize) -> Config {
        let mut c = Config::default();
        c.post_bg_floating_max_pixels = max_pixels;
        c
    }

    #[test]
    fn removes_small_speckles_keeps_largest() {
        // 5x5 transparent + one 3px cluster (largest) + one 1px speckle
        let mut img: RgbaImage = ImageBuffer::new(5, 5);
        for y in 0..5 {
            for x in 0..5 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        // largest: 3-pixel L at top-left
        img.put_pixel(0, 0, Rgba([10, 10, 10, 255]));
        img.put_pixel(1, 0, Rgba([10, 10, 10, 255]));
        img.put_pixel(0, 1, Rgba([10, 10, 10, 255]));
        // speckle: 1px at bottom-right
        img.put_pixel(4, 4, Rgba([20, 20, 20, 255]));
        let out = remove_small_floating_components(img, &config_floating(2));
        assert_eq!(out.get_pixel(0, 0)[3], 255, "largest component survives");
        assert_eq!(out.get_pixel(4, 4)[3], 0, "1px speckle removed");
    }

    #[test]
    fn zero_threshold_is_noop() {
        let mut img: RgbaImage = ImageBuffer::new(2, 2);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        img.put_pixel(1, 1, Rgba([0, 0, 0, 255]));
        let out = remove_small_floating_components(img, &config_floating(0));
        assert_eq!(out.get_pixel(0, 0)[3], 255);
        assert_eq!(out.get_pixel(1, 1)[3], 255);
    }

    #[test]
    fn small_main_object_survives_as_largest() {
        // a single 1px object is the largest (and only) -> kept
        let mut img: RgbaImage = ImageBuffer::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        img.put_pixel(1, 1, Rgba([99, 99, 99, 255]));
        let out = remove_small_floating_components(img, &config_floating(5));
        assert_eq!(out.get_pixel(1, 1)[3], 255, "largest survives even if small");
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test postprocess::floodfill`
Expected: all floodfill tests pass (4 from Task 6 + 3 new = 7).

- [ ] **Step 3: Wire floating cleanup into the postprocess entry (after bg-remove)**

In `src/postprocess/mod.rs` `postprocess()`, add right after the `post_bg_remove` block:

```rust
    if config.post_bg_floating_max_pixels > 0 {
        img = floodfill::remove_small_floating_components(img, config);
    }
```

- [ ] **Step 4: Verify anchor unchanged**

Run: `cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src/postprocess/floodfill.rs src/postprocess/mod.rs
git commit -m "feat(phase4): floating-island cleanup (4-conn CCL, largest survives)"
```

---

### Task 8: Verify dispatch order (integration)

Confirm the fixed order composes correctly when multiple ops are on. This is a test-only task against the now-complete `postprocess()`.

**Files:**
- Modify: `src/postprocess/mod.rs` (append a test to the file — create a `#[cfg(test)]` block at the bottom)

- [ ] **Step 1: Append a dispatch-order test to `src/postprocess/mod.rs`**

At the bottom of `src/postprocess/mod.rs`:

```rust
#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::postprocess::{OutlineStyle, AlphaThreshold};
    use image::{ImageBuffer, Rgba};

    #[test]
    fn outline_runs_last_and_grows_canvas() {
        // alpha binarize + outline both on: outline must run after binarize
        // (so it sees clean alpha) and grow the canvas by +2.
        let mut img: RgbaImage = ImageBuffer::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                img.put_pixel(x, y, Rgba([100, 100, 100, 200]));
            }
        }
        let mut c = Config::default();
        c.post_alpha_threshold = AlphaThreshold::Fixed(128);
        c.post_outline = OutlineStyle::Sharp;
        let out = postprocess(img, &c);
        assert_eq!(out.dimensions(), (4, 4), "outline grew canvas +2");
        // alpha was binarized to 255 (>128), then outline drawn into border ring
        assert_eq!(out.get_pixel(1, 1)[3], 255);
    }

    #[test]
    fn all_off_is_identity() {
        let mut img: RgbaImage = ImageBuffer::from_pixel(2, 2, Rgba([1, 2, 3, 200]));
        let before = img.clone();
        let out = postprocess(img, &Config::default());
        assert_eq!(out.dimensions(), before.dimensions());
        assert_eq!(out.get_pixel(0, 0), before.get_pixel(0, 0));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test postprocess::dispatch`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add src/postprocess/mod.rs
git commit -m "test(phase4): postprocess dispatch order + all-off identity"
```

---

### Task 9: CLI flags

Wire the 9 flags. Spec §CLI. `--outline-color` reuses `parse_palette_hex`.

**Files:**
- Modify: `src/cli/args.rs`

- [ ] **Step 1: Add the flag cases to `parse_cli_args`**

In `src/cli/args.rs`, inside the `while i < args.len()` match (add these arms before the `arg if arg.starts_with("--")` catch-all):

```rust
            "--bg-remove" => {
                config.post_bg_remove = true;
                i += 1;
            }
            "--bg-tolerance" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--bg-tolerance requires a value".to_string(),
                    ));
                };
                match val.parse::<u8>() {
                    Ok(t) => config.post_bg_tolerance = t,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --bg-tolerance '{}' (expected 0-255)", val
                    ))),
                }
                i += 2;
            }
            "--bg-connectivity" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--bg-connectivity requires a value".to_string(),
                    ));
                };
                config.post_bg_connectivity = match val.as_str() {
                    "4" => crate::postprocess::BgConnectivity::Conn4,
                    "8" => crate::postprocess::BgConnectivity::Conn8,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --bg-connectivity '{}' (expected 4|8)", val
                    ))),
                };
                i += 2;
            }
            "--bg-scope" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--bg-scope requires a value".to_string(),
                    ));
                };
                config.post_bg_scope = match val.as_str() {
                    "outer" => crate::postprocess::BgScope::Outer,
                    "all" => crate::postprocess::BgScope::All,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --bg-scope '{}' (expected outer|all)", val
                    ))),
                };
                i += 2;
            }
            "--bg-floating-threshold" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--bg-floating-threshold requires a value".to_string(),
                    ));
                };
                match val.parse::<usize>() {
                    Ok(n) => config.post_bg_floating_max_pixels = n,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --bg-floating-threshold '{}' (expected a non-negative integer)", val
                    ))),
                }
                i += 2;
            }
            "--outline" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--outline requires a value".to_string(),
                    ));
                };
                config.post_outline = match val.as_str() {
                    "none" => crate::postprocess::OutlineStyle::None,
                    "rounded" => crate::postprocess::OutlineStyle::Rounded,
                    "sharp" => crate::postprocess::OutlineStyle::Sharp,
                    _ => return Err(PixelSnapperError::InvalidInput(format!(
                        "invalid --outline '{}' (expected none|rounded|sharp)", val
                    ))),
                };
                i += 2;
            }
            "--outline-color" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--outline-color requires a value".to_string(),
                    ));
                };
                let colors = parse_palette_hex(val)?;
                if colors.len() != 1 {
                    return Err(PixelSnapperError::InvalidInput(
                        "--outline-color expects exactly one 6-digit hex color".to_string(),
                    ));
                }
                config.post_outline_color = colors[0];
                i += 2;
            }
            "--morph" => {
                config.post_morph = true;
                i += 1;
            }
            "--alpha-threshold" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(PixelSnapperError::InvalidInput(
                        "--alpha-threshold requires a value".to_string(),
                    ));
                };
                config.post_alpha_threshold = match val.as_str() {
                    "auto" => crate::postprocess::AlphaThreshold::Auto,
                    n => match n.parse::<u8>() {
                        Ok(t) => crate::postprocess::AlphaThreshold::Fixed(t),
                        Err(_) => return Err(PixelSnapperError::InvalidInput(format!(
                            "invalid --alpha-threshold '{}' (expected 0-255 or auto)", n
                        ))),
                    },
                };
                i += 2;
            }
```

- [ ] **Step 2: Add help text for the new flags**

In `src/cli/args.rs` `print_cli_help`, add these lines into the `OPTIONS:` section string (before the `--json` line):

```
  --bg-remove                                Enable background removal
  --bg-tolerance <0-255>                     Per-channel bg tolerance [default: 64]
  --bg-connectivity <4|8>                    Flood connectivity [default: 4]
  --bg-scope <outer|all>                     Removal scope [default: outer]
  --bg-floating-threshold <n>                Floating-island cleanup size (0=off) [default: 0]
  --outline <none|rounded|sharp>             Outline style [default: none]
  --outline-color <hex>                      Outline color [default: 000000]
  --morph                                    Enable 2x2 open->close (alpha-only)
  --alpha-threshold <n|auto>                 Alpha binarize (strict >) [default: off]
```

- [ ] **Step 3: Add CLI parsing tests to `src/cli/cli_tests.rs`**

Append to `src/cli/cli_tests.rs` (inside the test module, following the existing pattern — these call `parse_cli_args` directly):

```rust
    #[test]
    fn parse_outline_color_single_hex() {
        let args = ["in.png", "out.png", "--outline-color", "ff8800"];
        let cmd = super::parse_cli_args(&args).expect("ok");
        let cfg = match cmd {
            super::CliCommand::Run(c) => c,
            _ => panic!("expected Run"),
        };
        assert_eq!(cfg.post_outline_color, [255, 136, 0]);
    }

    #[test]
    fn parse_outline_color_rejects_two_colors() {
        let args = ["in.png", "out.png", "--outline-color", "ff8800,112233"];
        assert!(super::parse_cli_args(&args).is_err());
    }

    #[test]
    fn parse_alpha_threshold_auto() {
        let args = ["in.png", "out.png", "--alpha-threshold", "auto"];
        let cfg = match super::parse_cli_args(&args).expect("ok") {
            super::CliCommand::Run(c) => c,
            _ => panic!("expected Run"),
        };
        assert!(matches!(cfg.post_alpha_threshold, crate::postprocess::AlphaThreshold::Auto));
    }

    #[test]
    fn parse_alpha_threshold_fixed() {
        let args = ["in.png", "out.png", "--alpha-threshold", "200"];
        let cfg = match super::parse_cli_args(&args).expect("ok") {
            super::CliCommand::Run(c) => c,
            _ => panic!("expected Run"),
        };
        assert!(matches!(cfg.post_alpha_threshold, crate::postprocess::AlphaThreshold::Fixed(200)));
    }

    #[test]
    fn parse_morph_and_bg_remove_bool_flags() {
        let args = ["in.png", "out.png", "--morph", "--bg-remove"];
        let cfg = match super::parse_cli_args(&args).expect("ok") {
            super::CliCommand::Run(c) => c,
            _ => panic!("expected Run"),
        };
        assert!(cfg.post_morph);
        assert!(cfg.post_bg_remove);
    }

    #[test]
    fn parse_bg_scope_and_connectivity() {
        let args = ["in.png", "out.png", "--bg-scope", "all", "--bg-connectivity", "8"];
        let cfg = match super::parse_cli_args(&args).expect("ok") {
            super::CliCommand::Run(c) => c,
            _ => panic!("expected Run"),
        };
        assert_eq!(cfg.post_bg_scope, crate::postprocess::BgScope::All);
        assert_eq!(cfg.post_bg_connectivity, crate::postprocess::BgConnectivity::Conn8);
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test cli_tests`
Expected: all pass (existing + 6 new).

- [ ] **Step 5: Commit**

```bash
git add src/cli/args.rs src/cli/cli_tests.rs
git commit -m "feat(phase4): CLI flags for postprocess ops"
```

---

### Task 10: WASM `post_config` JSON param

Add serde+serde_json (wasm-target-only), define a `PostConfig` struct, parse the JSON in `process_image`. Spec §WASM.

**Files:**
- Modify: `Cargo.toml` (add wasm-target deps)
- Modify: `src/lib.rs` (param + parse, gated `#[cfg(target_arch = "wasm32")]`)

- [ ] **Step 1: Add wasm-target serde deps to `Cargo.toml`**

Add at the end of `Cargo.toml` (this makes serde/serde_json compile only for the wasm target, keeping the native CLI lean):

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Add the `PostConfig` struct + parser to `src/lib.rs`**

Near the top of `src/lib.rs` (after the `use` block, before `process_image_common`), add a wasm-gated block:

```rust
#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct PostConfig {
    bg_remove: Option<bool>,
    bg_tolerance: Option<u8>,
    bg_connectivity: Option<String>,
    bg_scope: Option<String>,
    bg_floating_threshold: Option<usize>,
    outline: Option<String>,
    outline_color: Option<String>,
    morph: Option<bool>,
    alpha_threshold: Option<String>,
}

#[cfg(target_arch = "wasm32")]
fn apply_post_config(config: &mut Config, json: &str) -> std::result::Result<(), wasm_bindgen::JsValue> {
    let pc: PostConfig = serde_json::from_str(json)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("invalid post_config JSON: {}", e)))?;
    if let Some(v) = pc.bg_remove { config.post_bg_remove = v; }
    if let Some(v) = pc.bg_tolerance { config.post_bg_tolerance = v; }
    if let Some(v) = pc.bg_connectivity {
        config.post_bg_connectivity = match v.as_str() {
            "4" => postprocess::BgConnectivity::Conn4,
            "8" => postprocess::BgConnectivity::Conn8,
            _ => return Err(wasm_bindgen::JsValue::from_str("bg_connectivity must be 4|8")),
        };
    }
    if let Some(v) = pc.bg_scope {
        config.post_bg_scope = match v.as_str() {
            "outer" => postprocess::BgScope::Outer,
            "all" => postprocess::BgScope::All,
            _ => return Err(wasm_bindgen::JsValue::from_str("bg_scope must be outer|all")),
        };
    }
    if let Some(v) = pc.bg_floating_threshold { config.post_bg_floating_max_pixels = v; }
    if let Some(v) = pc.outline {
        config.post_outline = match v.as_str() {
            "none" => postprocess::OutlineStyle::None,
            "rounded" => postprocess::OutlineStyle::Rounded,
            "sharp" => postprocess::OutlineStyle::Sharp,
            _ => return Err(wasm_bindgen::JsValue::from_str("outline must be none|rounded|sharp")),
        };
    }
    if let Some(v) = pc.outline_color {
        let cols = palette::parse_palette_hex(&v).map_err(wasm_bindgen::JsValue::from)?;
        if cols.len() != 1 {
            return Err(wasm_bindgen::JsValue::from_str("outline_color must be a single hex color"));
        }
        config.post_outline_color = cols[0];
    }
    if let Some(v) = pc.morph { config.post_morph = v; }
    if let Some(v) = pc.alpha_threshold {
        config.post_alpha_threshold = match v.as_str() {
            "auto" => postprocess::AlphaThreshold::Auto,
            n => match n.parse::<u8>() {
                Ok(t) => postprocess::AlphaThreshold::Fixed(t),
                Err(_) => return Err(wasm_bindgen::JsValue::from_str("alpha_threshold must be 0-255 or auto")),
            },
        };
    }
    Ok(())
}
```

- [ ] **Step 3: Add the `post_config` parameter to `process_image`**

In `src/lib.rs`, change the wasm `process_image` signature to add the new param (add it last), and call `apply_post_config` right after the existing config setup. The signature becomes:

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
    colorspace: Option<String>,
    dither: Option<String>,
    preset_palette: Option<String>,
    post_config: Option<String>,
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
```

Then, just before the final `process_image_common(input_bytes, Some(config))` call, add:

```rust
    if let Some(json) = post_config {
        apply_post_config(&mut config, &json)?;
    }
```

- [ ] **Step 4: Verify both targets build + tests green**

Run: `cargo build --target wasm32-unknown-unknown`
Expected: 0 warnings.

Run: `cargo test`
Expected: all green (native unaffected — `PostConfig` is wasm-gated).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs
git commit -m "feat(phase4): WASM post_config JSON param (serde, wasm-target-only)"
```

---

### Task 11: Integration tests (anchor lock, determinism, end-to-end)

`tests/postprocess.rs` — default anchor unchanged, determinism (R1), and a real CLI run exercising bg-remove + outline. Spec §testing.

**Files:**
- Create: `tests/postprocess.rs`

- [ ] **Step 1: Create `tests/postprocess.rs`**

```rust
//! Phase 4 postprocess integration tests.
//! Cross-platform: sha2 crate + std::env::temp_dir (mirrors tests/resample.rs).

use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

fn tmp(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("pixel-snapper-p4-{}", name));
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

/// Default config (all postprocess off) -> Phase 3 Oklab anchor unchanged.
#[test]
fn default_config_anchor_unchanged() {
    let out = tmp("anchor.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png",
        out.as_str(),
        "16",
    ]));
    assert_eq!(
        sha256(&out),
        "3a589ee93b8cd2e493baa0d6fb314d279b54a1104165ad754ad4ff6d359e4420",
        "default config must match Phase 3 Oklab anchor (postprocess off)"
    );
}

/// Determinism (R1): same image + same postprocess config twice -> byte-identical.
#[test]
fn determinism_byte_identical() {
    let a = tmp("det_a.png");
    let b = tmp("det_b.png");
    let args = [
        "tests/fixtures/baseline/ai-sprite.png",
        a.as_str(),
        "16",
        "--bg-remove",
        "--alpha-threshold",
        "auto",
        "--morph",
    ];
    assert!(run_cli(&args));
    let args_b = [
        "tests/fixtures/baseline/ai-sprite.png",
        b.as_str(),
        "16",
        "--bg-remove",
        "--alpha-threshold",
        "auto",
        "--morph",
    ];
    assert!(run_cli(&args_b));
    assert_eq!(sha256(&a), sha256(&b), "same config must be byte-identical");
}

/// End-to-end: outline grows the output (postprocess actually ran).
/// Build a tiny sprite in-test, write to temp, run CLI with --outline, check
/// the output PNG decodes to a larger-than-input dimension.
#[test]
fn outline_grows_output_via_cli() {
    // build a 16x16 transparent image with an opaque red square center
    let mut img: image::RgbaImage = image::ImageBuffer::new(16, 16);
    for y in 6..10 {
        for x in 6..10 {
            img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        }
    }
    let input = tmp("outline_in.png");
    let output = tmp("outline_out.png");
    img.save(&input).expect("save input");
    assert!(run_cli(&[
        input.as_str(),
        output.as_str(),
        "16",
        "--pixel-size",
        "1",
        "--outline",
        "sharp",
    ]));
    let out = image::open(&output).expect("open output").to_rgba8();
    // input was snapped at pixel-size 1 (16x16); outline pads +2 -> at least wider
    assert!(
        out.width() >= 16 && out.height() >= 16,
        "outline ran: dims {:?}",
        out.dimensions()
    );
    // sanity: default black outline color present somewhere
    let has_black = out.pixels().any(|p| p[0] == 0 && p[1] == 0 && p[2] == 0 && p[3] == 255);
    assert!(has_black, "default black outline color should appear");
}

/// bg-remove via CLI produces a valid PNG with some transparent pixels
/// (background removed).
#[test]
fn bg_remove_via_cli_produces_transparency() {
    // ai-sprite has a background; --bg-remove should introduce/extend transparency.
    let out = tmp("bgremove.png");
    assert!(run_cli(&[
        "tests/fixtures/baseline/ai-sprite.png",
        out.as_str(),
        "16",
        "--bg-remove",
    ]));
    let img = image::open(&out).expect("open output").to_rgba8();
    let any_transparent = img.pixels().any(|p| p[3] == 0);
    assert!(any_transparent, "bg-remove should yield some transparent pixels");
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test --test postprocess`
Expected: 4 pass.

- [ ] **Step 3: Run the full suite + wasm build**

Run: `cargo test`
Expected: all green.

Run: `cargo build --target wasm32-unknown-unknown`
Expected: 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add tests/postprocess.rs
git commit -m "test(phase4): integration tests (anchor lock, determinism, CLI e2e)"
```

---

### Task 12: Docs (CLAUDE.md pipeline table + PLAN.md checkboxes)

**Files:**
- Modify: `CLAUDE.md`
- Modify: `PLAN.md`

- [ ] **Step 1: Add the postprocess stage to `CLAUDE.md` pipeline table + description**

In `CLAUDE.md`, in the modular pipeline table (the `| Stage | Module | Notes |` table), add a row after the Validate row:

```
| Postprocess | [src/postprocess/mod.rs](src/postprocess/mod.rs) | `bg_remove`/`outline`/`morph`/`alpha_threshold` — all off by default |
```

Add the sub-modules to the module list (after the Palette entry), e.g.:

```
- [src/postprocess/floodfill.rs](src/postprocess/floodfill.rs) — `remove_background` (Outer/All flood-fill) + `remove_small_floating_components`
- [src/postprocess/outline.rs](src/postprocess/outline.rs) — `apply_outline` (sharp=4-way / rounded=8-way, pad +1/side)
- [src/postprocess/morphology.rs](src/postprocess/morphology.rs) — `morph_open_close` (2×2 open→close, **alpha-only**)
- [src/postprocess/alpha.rs](src/postprocess/alpha.rs) — `binarize_alpha` (Fixed strict `>` / Otsu auto)
```

In the pipeline description (the numbered stage list), insert a new step 7 (renumber the apply_palette step):

```
7. **`postprocess`** (optional, all off by default) — fixed order: flood-fill bg removal → floating-island cleanup → morphology (alpha-only) → alpha binarize → outline. CLI flags `--bg-remove`/`--bg-tolerance`/`--bg-connectivity`/`--bg-scope`/`--bg-floating-threshold`/`--outline`/`--outline-color`/`--morph`/`--alpha-threshold`; WASM `post_config` JSON param. All ops RNG-free (deterministic). Defaults off → anchors unchanged.
```

- [ ] **Step 2: Tick Phase 4 checkboxes in `PLAN.md`**

In `PLAN.md`, under `## Phase 4 — 后处理全家桶`, change every `- [ ]` task line to `- [x]`, and append a short 实施记录 section at the end of the Phase 4 block (before `## Phase 5`):

```markdown
### 实施记录

- **分支**：`feat/phase4-postprocess`（N commit，已合并 main）
- **结果**：`postprocess/{mod,floodfill,outline,morphology,alpha}.rs` + Config 9 字段 + CLI 9 flags + WASM `post_config` JSON + `tests/postprocess.rs`
- **关键决策**（spec）：4 op 独立开关固定序；morph alpha-only（保调色板，偏离 unfake per-channel）；`All` scope 用 border 色自推导 target（一致性，偏离源码硬编码 4-way）；Otsu 全新实现（两源码库均无）；WASM 用单 JSON 参数（Option B，为 Phase 6 PipelineConfig 铺路）
- **偏离源码 3 处**（均文档化）：`All` 连通性 / morph alpha-only / Otsu 新增
- **验证**：cargo test 全绿，wasm 0 warning，Oklab 锚 `3a589ee9…e4420` + RGB 锚 `802857…9f22` 保持（默认全关零回归）
```

(Replace `N` with the actual commit count when the task runs.)

- [ ] **Step 3: Verify anchor one more time + commit**

Run: `cargo test --test postprocess default_config_anchor_unchanged`
Expected: pass.

```bash
git add CLAUDE.md PLAN.md
git commit -m "docs: phase 4 postprocess (CLAUDE.md pipeline + PLAN.md implementation record)"
```

---

## Self-Review (run after writing — already applied)

**1. Spec coverage:**
- flood-fill Outer/All → Task 6 ✓ (Selected scope explicitly Non-Goal)
- floating-island cleanup → Task 7 ✓
- outline sharp/rounded + color → Task 5 ✓
- morphology 2×2 alpha-only open→close → Task 4 ✓
- alpha Fixed strict + Otsu + degenerate fallback → Task 3 ✓
- pipeline slot + ProcessedImage dims → Task 2 ✓
- Config 9 fields → Task 1 ✓
- CLI 9 flags → Task 9 ✓
- WASM `post_config` JSON → Task 10 ✓
- determinism + anchors + fixtures → Task 11 ✓ (refined: synthetic in-test images instead of binary fixture files — cleaner, cross-platform)
- fixed dispatch order → Task 8 ✓
- docs → Task 12 ✓

**2. Placeholder scan:** none — all code complete, all defaults concrete, all test assertions explicit.

**3. Type consistency:** `post_*` field names match across Config (Task 1), postprocess dispatch (Tasks 3-8), CLI (Task 9), WASM PostConfig (Task 10 — camel_case JSON keys map to snake_case fields via explicit field-by-field copy). Enum variant names (`Conn4`/`Conn8`, `Outer`/`All`, `None`/`Rounded`/`Sharp`, `None`/`Fixed(u8)`/`Auto`) identical everywhere they're referenced.
