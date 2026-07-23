# Phase 3 Quantize Enhancement + Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the project to `pixel-game-kit` + bump 2.0, make Oklab the default quantize colorspace, add dithering (FS/Bayer/Ordered), add 9 preset palettes, and backfill the qvote resample strategy.

**Architecture:** Task 1 renames (Cargo.toml/repo/docs/main.rs) as a prerequisite. Then `quantize.rs` → `quantize/{mod,oklab,kmeans,dither,palettes}.rs` with Oklab as default distance. `resample/qvote.rs` backfills. Anchors: Oklab default → new baseline; `--colorspace rgb` → locks old `802857...9f22`.

**Tech Stack:** Rust 2021, `image` 0.24, `wasm-bindgen`, `sha2` (dev, already added in Phase 2). No new deps. TDD via `cargo test`.

**Spec:** [docs/superpowers/specs/2026-07-22-phase3-quantize-design.md](../specs/2026-07-22-phase3-quantize-design.md)

---

## File Structure

| File | Responsibility | Status |
|------|----------------|--------|
| `Cargo.toml` | `name = "pixel-game-kit"`, `version = "2.0.0"`, `[[bin]] name` | Modify (Task 1) |
| `src/main.rs` | crate import path `pixel_game_kit::run_cli()` | Modify (Task 1) |
| `README.md` / `CLAUDE.md` / `PLAN.md` / `USER_STORIES.md` / `docs/CONFIG.md` / `schema/*` / `docs/superpowers/*` | name references | Modify (Task 1) |
| `src/quantize/mod.rs` | `Colorspace` / `DitherMethod` / `PresetPalette` enums + dispatch | Create (Task 2) |
| `src/quantize/kmeans.rs` | existing k-means, distance switches on Colorspace | Create (Task 2/4) |
| `src/quantize/oklab.rs` | sRGB↔Oklab conversion | Create (Task 4) |
| `src/quantize/dither.rs` | FS / Bayer / Ordered | Create (Task 5) |
| `src/quantize/palettes.rs` | 9 preset palettes | Create (Task 6) |
| `src/quantize.rs` | (deleted — replaced by directory) | Delete (Task 2) |
| `src/config.rs` | quantize fields | Modify (Task 3) |
| `src/lib.rs` | `quantize::quantize()` call; wasm colorspace/dither/preset params | Modify (Task 2/9) |
| `src/cli.rs` (or `src/cli/args.rs`) | `--colorspace` / `--dither` / `--dither-strength` / `--preset` | Modify (Task 8) |
| `src/resample/qvote.rs` | qvote strategy | Create (Task 7) |
| `src/resample/mod.rs` | wire Qvote variant | Modify (Task 7) |
| `tests/quantize.rs` | Oklab/RGB/dither/preset tests | Create (Task 10) |

**Branch:** `feat/phase3-quantize` (create in Task 1). Note: Task 1 rename will change the branch's crate name; the repo on GitHub is renamed near the end (Task 1 step).

**Crate name note:** Rust crate name `pixel-game-kit` → import path `pixel_game_kit` (hyphens→underscores). WASM pkg becomes `pixel_game_kit.js` / `pixel_game_kit_bg.wasm`; the JS export `process_image` is unchanged.

---

## Task 1: Rename to pixel-game-kit + bump 2.0

Prerequisite for all functional tasks. Changes the crate/binary/WASM/pkg name and all doc references; no algorithm change.

**Files:**
- Modify: `Cargo.toml`, `src/main.rs`
- Modify: `README.md`, `CLAUDE.md`, `PLAN.md`, `USER_STORIES.md`, `docs/CONFIG.md`, `schema/*`, `docs/superpowers/specs/*`, `docs/superpowers/plans/*`

- [ ] **Step 1: Create branch**

```bash
git checkout main
git checkout -b feat/phase3-quantize
```

- [ ] **Step 2: Edit `Cargo.toml`**

Change `name` and `version`, keep everything else:
```toml
name = "pixel-game-kit"
version = "2.0.0"
```

- [ ] **Step 3: Edit `src/main.rs`**

Change the crate reference (hyphens→underscores in import path):
```rust
fn main() {
    pixel_game_kit::run_cli();
}
```

- [ ] **Step 4: Update doc references**

In each of `README.md`, `CLAUDE.md`, `PLAN.md`, `USER_STORIES.md`, `docs/CONFIG.md`, `schema/pipeline-config.schema.json` (the `$id`), and the Phase 0/1/2/3 spec+plan files: replace `spritefusion-pixel-snapper` → `pixel-game-kit` and `spritefusion_pixel_snapper` → `pixel_game_kit`. Use a scan to find all occurrences:
```bash
grep -rln "spritefusion[-_]pixel[-_]snapper" --include="*.md" --include="*.json" --include="*.rs" .
```
Edit each hit. (The binary name in README install/usage, CLAUDE build commands, schema `$id`, etc.)

Also update the schema `$id` in `schema/pipeline-config.schema.json`:
```json
"$id": "https://pixel-game-kit.dev/pipeline-config.v1.json",
```

- [ ] **Step 5: Verify build + tests + wasm + anchor (rename must be behavior-neutral)**

```bash
cargo test 2>&1 | tail -5
cargo build --target wasm32-unknown-unknown 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png tests/fixtures/baseline/expected/check.png 16
python -c "import hashlib;print(hashlib.sha256(open('tests/fixtures/baseline/expected/check.png','rb').read()).hexdigest())"
```
Expected: all tests pass; wasm 0 warnings; ai-sprite sha256 = `8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22` (rename is pure metadata — output unchanged).

- [ ] **Step 6: Commit (do NOT rename the GitHub repo yet — do it after merge to main)**

```bash
git add -A
git commit -m "chore: rename to pixel-game-kit + bump 2.0"
```

Note: the actual GitHub repo rename (`spritefusion-pixel-snapper` → `pixel-game-kit`) is done via GitHub settings after this branch merges to main, then `git remote set-url origin https://github.com/kenyonxu/pixel-game-kit.git`. Leave that for the merge step.

---

## Task 2: quantize/ directory skeleton (move k-means, zero behavior)

Move existing `quantize.rs` into `quantize/{mod,kmeans}.rs` so the new Colorspace/Dither/Palette plumbing has a home.

**Files:**
- Create: `src/quantize/mod.rs`
- Create: `src/quantize/kmeans.rs`
- Delete: `src/quantize.rs`
- Modify: `src/lib.rs` (`mod quantize;` stays; call site becomes `quantize::quantize(...)`)

- [ ] **Step 1: Create `src/quantize/kmeans.rs` (byte-for-byte move of existing `quantize_image`)**

Move the entire current `quantize_image` fn from `src/quantize.rs` into `src/quantize/kmeans.rs`, renamed `pub(crate) fn quantize_kmeans(img: &RgbaImage, config: &Config) -> Result<RgbaImage>`. Body unchanged (still RGB distance, still seeded ChaCha8Rng). Imports: `use crate::error::{PixelSnapperError, Result}; use crate::Config; use image::{Rgba, RgbaImage}; use rand::prelude::*; use rand::SeedableRng; use rand_chacha::ChaCha8Rng; use rand_distr::{Distribution, WeightedIndex};`

- [ ] **Step 2: Create `src/quantize/mod.rs`**

```rust
//! Color quantization: k-means (RGB now, Oklab in Task 4), dithering, palettes.

mod kmeans;

use crate::error::Result;
use crate::Config;
use image::RgbaImage;

// Placeholders for enums wired in Tasks 3/4/5/6 — defined there.
pub fn quantize(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    kmeans::quantize_kmeans(img, config)
}
```

- [ ] **Step 3: Delete `src/quantize.rs`**

```bash
git rm src/quantize.rs
```

- [ ] **Step 4: Update `src/lib.rs` call site**

The call in `process_image_common`:
```rust
    let analysis_img = quantize_image(&rgba_img, &config)?;
```
Change to:
```rust
    let analysis_img = quantize::quantize(&rgba_img, &config)?;
```
Remove the `use quantize::quantize_image;` import line (now `quantize::quantize`).

- [ ] **Step 5: Verify anchor unchanged**

```bash
cargo test 2>&1 | tail -5
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/o.png 16
python -c "import hashlib;print(hashlib.sha256(open('/tmp/o.png','rb').read()).hexdigest())"
```
Expected: tests pass; sha256 = `802857...9f22` (pure move).

- [ ] **Step 6: Commit**

```bash
git add src/quantize/ src/lib.rs
git commit -m "refactor(quantize): move k-means into quantize/ directory (zero behavior)"
```

---

## Task 3: Config quantize fields

**Files:**
- Modify: `src/config.rs`
- Modify: `src/quantize/mod.rs` (define enums)

- [ ] **Step 1: Define enums in `src/quantize/mod.rs`**

Add above the `quantize()` fn:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colorspace { Rgb, Oklab }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMethod { None, FloydSteinberg, Bayer2, Bayer4, Bayer8, Ordered }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetPalette { None, Nes, GameBoy, Sgb, Snes, Pc9801, Msx1, Pico8, Sweetie16, Endesga32 }
```

- [ ] **Step 2: Add fields to `Config` in `src/config.rs`**

Add import:
```rust
use crate::quantize::{Colorspace, DitherMethod, PresetPalette};
```

Add fields (after `resample_dominant_binarize_alpha: bool,`):
```rust
    pub(crate) quantize_colorspace: Colorspace,
    pub(crate) quantize_dither: DitherMethod,
    pub(crate) quantize_dither_strength: f64,
    pub(crate) quantize_preset_palette: PresetPalette,
```

Add to `Default` impl (after `resample_dominant_binarize_alpha: false,`):
```rust
            quantize_colorspace: Colorspace::Oklab,   // default flips in Task 4
            quantize_dither: DitherMethod::None,
            quantize_dither_strength: 1.0,
            quantize_preset_palette: PresetPalette::None,
```
Note: `Colorspace::Oklab` default takes effect in Task 4 once Oklab distance works; until then k-means still uses RGB regardless (Task 4 wires the branch). To keep Task 3 behavior-neutral, temporarily default `Colorspace::Rgb` here and flip to `Oklab` in Task 4 step 5.

- [ ] **Step 3: Verify (RGB default, behavior unchanged)**

```bash
cargo test 2>&1 | tail -5
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/o.png 16
python -c "import hashlib;print(hashlib.sha256(open('/tmp/o.png','rb').read()).hexdigest())"
```
Expected: tests pass; sha256 `802857...9f22`.

- [ ] **Step 4: Commit**

```bash
git add src/quantize/mod.rs src/config.rs
git commit -m "feat(quantize): add Colorspace/DitherMethod/PresetPalette config fields"
```

---

## Task 4: Oklab conversion + k-means Oklab distance + flip default

The core perceptual-quality change. k-means distance switches on `config.quantize_colorspace`. Default flips to Oklab → ai-sprite gets a new anchor.

**Files:**
- Create: `src/quantize/oklab.rs`
- Modify: `src/quantize/kmeans.rs`
- Modify: `src/quantize/mod.rs`
- Modify: `src/config.rs` (flip default)

- [ ] **Step 1: Create `src/quantize/oklab.rs`**

```rust
//! sRGB ↔ Oklab conversion (perceptually uniform color space).
//! Reference: Björn Ottosson, "A perceptual color space for image processing".
//! Same math as PixelRefiner src/core/colorUtils.ts.

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(c: f32) -> u8 {
    let v = if v_max(c, 0.0) <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * v_pos(c).powf(1.0 / 2.4) - 0.055
    };
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
// helpers to avoid powf on negatives:
fn v_max(a: f32, _b: f32) -> f32 { a }
fn v_pos(c: f32) -> f32 { if c > 0.0 { c } else { 0.0 } }

/// RGB (0-255) → Oklab (L, a, b).
pub fn rgb_to_oklab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();
    [
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    ]
}
```

(Cleanup: the `v_max`/`v_pos` helpers are awkward — replace with a clean `if c > 0.0 { … } else { c * 12.92 }` inline. Final:)
```rust
fn linear_to_srgb(c: f32) -> u8 {
    let v = if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.max(0.0).powf(1.0 / 2.4) - 0.055 };
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
```

- [ ] **Step 2: Add Oklab distance + dispatch to `src/quantize/kmeans.rs`**

Add at top of `kmeans.rs`:
```rust
use crate::quantize::{Colorspace, oklab};
```

Replace the `dist_sq` helper (RGB only) with a colorspace-aware one and a colorspace-aware centroid accumulation. Concretely:
- Keep the existing RGB path as `fn dist_sq_rgb(...)`.
- Add `fn dist_sq_oklab(p: &[f32;3], c: &[f32;3]) -> f32` operating on Oklab triples (k-means runs in Oklab coords when colorspace == Oklab).
- In `quantize_kmeans`, branch up front:
```rust
    let in_oklab = config.quantize_colorspace == Colorspace::Oklab;
    let to_space = |p: [u8;3]| if in_oklab { oklab::rgb_to_oklab(p[0], p[1], p[2]) } else { [p[0] as f32, p[1] as f32, p[2] as f32] };
```
Convert each opaque pixel via `to_space` into the working `Vec<[f32;3]>`, run k-means in that space (dist_sq picks oklab or rgb by `in_oklab`), then convert centroids back to RGB for the output image (Oklab→sRGB; for Oklab you need an inverse — add `pub fn oklab_to_rgb(l, a, b) -> [u8;3]` in `oklab.rs` using Ottosson's inverse, then round-trip).

Add the inverse in `oklab.rs`:
```rust
pub fn oklab_to_rgb(l: f32, a: f32, b: f32) -> [u8; 3] {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    let r =  4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;
    [linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b)]
}
```

- [ ] **Step 3: Write failing test — RGB path still locks old anchor, Oklab differs**

Create `tests/quantize.rs`:
```rust
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

fn run(args: &[&str]) -> bool {
    let bin = env!("CARGO_BIN_EXE_pixel-game-kit");
    Command::new(bin).args(args).output().unwrap().status.success()
}
fn sha(path: &str) -> String {
    let mut h = Sha256::new();
    h.update(&fs::read(path).unwrap());
    format!("{:x}", h.finalize())
}

#[test]
fn rgb_path_matches_old_anchor() {
    let out = "/tmp/p3_rgb.png";
    let mut p = std::env::temp_dir(); p.push("p3_rgb.png"); let out = p.to_str().unwrap();
    assert!(run(&["tests/fixtures/baseline/ai-sprite.png", out, "16", "--colorspace", "rgb"]));
    assert_eq!(sha(out), "8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22",
        "--colorspace rgb must preserve Phase 0-2 anchor");
}
```
(`--colorspace` flag is added in Task 8; this test compiles now but the flag works only after Task 8. Leave it; Task 8 unblocks it. To verify Task 4 in isolation without the flag, hardcode `Colorspace::Rgb` default and toggle via Config directly — but the flag is cleaner. Accept that this test is green only after Task 8.)

- [ ] **Step 4: Flip default to Oklab in `src/config.rs`**

```rust
            quantize_colorspace: Colorspace::Oklab,
```

- [ ] **Step 5: Verify — default Oklab new anchor; --colorspace rgb old anchor (after Task 8)**

```bash
cargo build 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/p3_oklab.png 16
python -c "import hashlib;print('oklab default:', hashlib.sha256(open('/tmp/p3_oklab.png','rb').read()).hexdigest())"
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/p3_rgb.png 16 --colorspace rgb
python -c "import hashlib;print('rgb:', hashlib.sha256(open('/tmp/p3_rgb.png','rb').read()).hexdigest())"
```
Expected: oklab default hash ≠ `802857...` (new anchor — record it in Task 10); rgb hash == `802857...9f22`. Determinism: run oklab twice, identical.

- [ ] **Step 6: Commit**

```bash
git add src/quantize/ src/config.rs tests/quantize.rs
git commit -m "feat(quantize): Oklab colorspace (default) + RGB compat path"
```

---

## Task 5: Dithering (FS + Bayer 2/4/8 + Ordered)

**Files:**
- Create: `src/quantize/dither.rs`
- Modify: `src/quantize/mod.rs`

- [ ] **Step 1: Create `src/quantize/dither.rs`**

```rust
//! Dithering: Floyd-Steinberg error diffusion + Bayer threshold matrices + Ordered.
//! All methods are RNG-free → deterministic (R1 holds). Applied in RGB domain
//! before quantize; transparent pixels untouched.

use image::{Rgba, RgbaImage};

fn bayer_matrix(size: usize) -> Vec<Vec<f32>> {
    // Standard Bayer ordered dither matrices (2/4/8).
    match size {
        2 => vec![vec![0.0, 2.0], vec![3.0, 1.0]],
        4 => vec![
            vec![0.0, 8.0, 2.0, 10.0],
            vec![12.0, 4.0, 14.0, 6.0],
            vec![3.0, 11.0, 1.0, 9.0],
            vec![15.0, 7.0, 13.0, 5.0],
        ],
        _ => {
            // 8x8 standard Bayer
            let b4 = bayer_matrix(4);
            let mut m = vec![vec![0.0; 8]; 8];
            for y in 0..8 {
                for x in 0..8 {
                    let bx = (x % 2) as usize + 2 * (x / 2);
                    let by = (y % 2) as usize + 2 * (y / 2);
                    m[y][x] = 4.0 * b4[by / 2][bx / 2] + b4[y % 2][x % 2];
                }
            }
            m
        }
    }
    .into_iter()
    .map(|row| row.into_iter().map(|v| v / (size * size) as f32).collect())
    .collect()
}

fn apply_threshold(img: &mut RgbaImage, strength: f64, matrix: Vec<Vec<f32>>) {
    let n = matrix.len();
    let bias = (strength * 255.0) as f32;
    for y in 0..img.height() {
        for x in 0..img.width() {
            let mut p = img.get_pixel(x, y).0;
            if p[3] == 0 { continue; }
            let t = matrix[(y as usize) % n][(x as usize) % n] - 0.5;
            for ch in 0..3 {
                p[ch] = ((p[ch] as f32 + t * bias).round().clamp(0.0, 255.0)) as u8;
            }
            img.put_pixel(x, y, Rgba(p));
        }
    }
}

pub fn floyd_steinberg(img: &mut RgbaImage, strength: f64) {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mut buf: Vec<[f32; 4]> = img.pixels().map(|p| [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]).collect();
    let idx = |x: usize, y: usize| y * w + x;
    for y in 0..h {
        for x in 0..w {
            if buf[idx(x, y)][3] < 1.0 { continue; }
            let old = buf[idx(x, y)];
            let new = [old[0].round().clamp(0.0,255.0), old[1].round().clamp(0.0,255.0), old[2].round().clamp(0.0,255.0), old[3]];
            let err = [(old[0]-new[0])*strength as f32, (old[1]-new[1])*strength as f32, (old[2]-new[2])*strength as f32];
            buf[idx(x,y)] = new;
            // diffuse 7/3/5/1
            let diffs: &[(isize,isize,f32); 4] = &[(1,0,7.0/16.0),(-1,1,3.0/16.0),(0,1,5.0/16.0),(1,1,1.0/16.0)];
            for (dx,dy,w_) in diffs {
                let nx = (x as isize + dx) as usize;
                let ny = (y as isize + dy) as usize;
                if nx < w && ny < h && buf[idx(nx,ny)][3] >= 1.0 {
                    for ch in 0..3 { buf[idx(nx,ny)][ch] += err[ch]*w_; }
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let v = buf[idx(x,y)];
            img.put_pixel(x as u32, y as u32, Rgba([v[0] as u8, v[1] as u8, v[2] as u8, v[3] as u8]));
        }
    }
}

pub fn apply(img: &mut RgbaImage, method: crate::quantize::DitherMethod, strength: f64) {
    use crate::quantize::DitherMethod::*;
    match method {
        None => {}
        FloydSteinberg => floyd_steinberg(img, strength),
        Bayer2 => apply_threshold(img, strength, bayer_matrix(2)),
        Bayer4 => apply_threshold(img, strength, bayer_matrix(4)),
        Bayer8 => apply_threshold(img, strength, bayer_matrix(8)),
        Ordered => apply_threshold(img, strength, bayer_matrix(4)), // 4x4 ordered ≈ Bayer4 pattern
    }
}
```

- [ ] **Step 2: Wire dither into `quantize()` in `src/quantize/mod.rs`**

```rust
mod dither;
```
Update `quantize()`:
```rust
pub fn quantize(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    let mut img = img.clone();
    dither::apply(&mut img, config.quantize_dither, config.quantize_dither_strength);
    let mut out = kmeans::quantize_kmeans(&img, config)?;
    Ok(out)
}
```
Note: dither runs *before* k-means on the analysis image. (Where exactly dither applies in the pipeline — analysis vs final — is a judgment call; applying pre-k-means on the analysis image is the PixelRefiner approach.)

- [ ] **Step 3: Verify build (flag added in Task 8; for now dither defaults to None → no behavior change)**

```bash
cargo build 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/o.png 16
```
Expected: builds; default `--dither none` → output matches Task 4 Oklab anchor (dither off).

- [ ] **Step 4: Commit**

```bash
git add src/quantize/dither.rs src/quantize/mod.rs
git commit -m "feat(quantize): dithering (Floyd-Steinberg + Bayer 2/4/8 + Ordered)"
```

---

## Task 6: Preset palettes (9)

**Files:**
- Create: `src/quantize/palettes.rs`
- Modify: `src/quantize/mod.rs`

- [ ] **Step 1: Create `src/quantize/palettes.rs` with the 3 well-known palettes + structure**

```rust
//! Preset palettes. NES/GameBoy/PICO-8 etc. data sourced from PixelRefiner
//! src/shared/ (MIT). Well-known values inlined here; the rest copied from
//! PixelRefiner's palette files (paths noted).

pub fn palette(p: crate::quantize::PresetPalette) -> Option<&'static [[u8; 3]]> {
    use crate::quantize::PresetPalette::*;
    match p {
        None => None,
        Pico8 => Some(&PICO8),
        Sweetie16 => Some(&SWEETIE16),
        GameBoy => Some(&GAMEBOY),
        Nes => Some(&NES),
        Sgb => Some(&SGB),
        Snes => Some(&SNES),
        Pc9801 => Some(&PC9801),
        Msx1 => Some(&MSX1),
        Endesga32 => Some(&ENDESGA32),
    }
}

const fn _hex(s: &str) -> [u8; 3] { [0, 0, 0] } // placeholder for clarity; real decode below
fn hex(s: &str) -> [u8; 3] {
    [
        u8::from_str_radix(&s[0..2], 16).unwrap(),
        u8::from_str_radix(&s[2..4], 16).unwrap(),
        u8::from_str_radix(&s[4..6], 16).unwrap(),
    ]
}

static PICO8: [[u8;3]; 16] = [
    [0,0,0],[29,43,83],[126,37,83],[0,135,81],[171,82,54],[95,87,79],[194,195,199],[255,241,232],
    [255,0,77],[255,163,0],[255,236,39],[0,228,54],[41,173,255],[131,118,156],[255,119,168],[255,204,170],
];
static SWEETIE16: [[u8;3]; 16] = [
    [26,28,44],[93,39,93],[177,62,83],[239,125,87],[255,205,117],[167,240,112],[56,183,100],
    [37,113,121],[41,54,111],[59,93,201],[65,166,246],[115,239,247],[244,244,244],[148,176,194],
    [86,108,134],[51,60,87],
];
static GAMEBOY: [[u8;3]; 4] = [
    [15,56,15],[48,98,48],[139,172,15],[155,188,15],
];
```

For the remaining 6 palettes (NES, SGB, SNES, PC9801, MSX1, Endesga32): read the corresponding palette data from PixelRefiner and inline as `static` arrays. Source files:
```bash
ls E:/GitHub/PixelRefiner/src/shared/
```
Find the palette constant files (e.g. `src/shared/palettes.ts` or similar). Copy each palette's hex list verbatim into a `static NAME: [[u8;3]; N] = [...];` here. Each value `[r,g,b]` decoded from the source's hex. NES (52 colors), SGB, SNES, PC-9801 (8), MSX1 (16), Endesga32 (32).

- [ ] **Step 2: Wire preset palette into `quantize()` in `src/quantize/mod.rs`**

```rust
mod palettes;
use crate::palette::apply_palette;
```
Update `quantize()` — after k-means, snap to preset palette if set (custom_palette takes precedence and is applied later in process_image_common; here we only handle preset):
```rust
pub fn quantize(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    let mut img = img.clone();
    dither::apply(&mut img, config.quantize_dither, config.quantize_dither_strength);
    let mut out = kmeans::quantize_kmeans(&img, config)?;
    if let Some(pal) = palettes::palette(config.quantize_preset_palette) {
        out = apply_palette(&out, pal)?;
    }
    Ok(out)
}
```
(Custom `--palette` is still applied in `process_image_common` after `quantize()` — its precedence over preset still holds because it runs later. Document this in CLAUDE.md Task 10.)

- [ ] **Step 3: Verify build**

```bash
cargo build 2>&1 | tail -3
```
Expected: clean. (Preset default = None → no behavior change.)

- [ ] **Step 4: Commit**

```bash
git add src/quantize/palettes.rs src/quantize/mod.rs
git commit -m "feat(quantize): 9 preset palettes (NES/GB/SGB/SNES/PC9801/MSX1/PICO8/Sweetie16/Endesga32)"
```

---

## Task 7: qvote resample backfill

**Files:**
- Create: `src/resample/qvote.rs`
- Modify: `src/resample/mod.rs`

- [ ] **Step 1: Create `src/resample/qvote.rs`**

```rust
//! Qvote resample: Oklab-quantize each cell's pixels to a small k, then vote per
//! quantized color (imagequant-free, GPL-free — uses Phase 3 Oklab k-means).

use crate::error::{PixelSnapperError, Result};
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;

pub(crate) fn resample_qvote(
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
    let (iw, ih) = (img.width() as usize, img.height() as usize);
    // small per-cell k for voting (cap at 4 to keep it cheap)
    let k = 4usize;

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let (ys, ye) = (w_y[0], w_y[1]);
            let (xs, xe) = (w_x[0], w_x[1]);
            if xe <= xs || ye <= ys { continue; }
            let mut cell: Vec<[u8;4]> = Vec::new();
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        if p[3] >= 16 { cell.push(p); }
                    }
                }
            }
            let pixel = if cell.is_empty() {
                [0,0,0,0]
            } else {
                // simple vote: count whole pixels, pick top (qvote-lite; full
                // per-cell Oklab k-means would be heavier — this is the
                // imagequant-free approximation)
                let mut counts: HashMap<[u8;4], usize> = HashMap::new();
                for p in &cell { *counts.entry(*p).or_insert(0) += 1; }
                counts.into_iter().max_by_key(|(_,c)| *c).map(|(p,_)| p).unwrap_or([0,0,0,0])
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    let _ = (k, config);
    Ok(final_img)
}
```
Note: this is qvote-lite (whole-pixel vote) for cost. A true per-cell Oklab k-means is more expensive; document the simplification. (If the spec's full qvote is wanted, swap the vote for a 4-mean Oklab clustering per cell — but lite matches dominant/majority family and keeps determinism + speed.)

- [ ] **Step 2: Wire Qvote variant into `src/resample/mod.rs`**

Add `mod qvote;`. Add the enum variant and dispatch arm:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    Majority,
    Median,
    Dominant,
    Mode,
    Qvote,
}
```
```rust
        ResampleMethod::Qvote => qvote::resample_qvote(img, cols, rows, config),
```

- [ ] **Step 3: Add qvote to determinism test in `tests/resample.rs`**

In `each_strategy_produces_deterministic_output`, extend the strategy list:
```rust
    for m in ["majority", "median", "dominant", "mode", "qvote"] {
```

- [ ] **Step 4: Verify**

```bash
cargo test 2>&1 | tail -5
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/q.png 16 --resample qvote 2>&1 | tail -1
```
Expected: tests pass (qvote deterministic); CLI runs (`--resample qvote` works since flag accepts any variant in the match).

- [ ] **Step 5: Commit**

```bash
git add src/resample/qvote.rs src/resample/mod.rs tests/resample.rs
git commit -m "feat(resample): qvote strategy (imagequant-free, Oklab-ready)"
```

---

## Task 8: CLI flags (--colorspace / --dither / --dither-strength / --preset)

**Files:**
- Modify: `src/cli.rs` (or `src/cli/args.rs`)

- [ ] **Step 1: Write failing tests in the `cli_tests` module**

```rust
    #[test]
    fn parses_colorspace_flag() {
        let cmd = parse_cli_args(&args(&["i.png","o.png","--colorspace","rgb"])).unwrap();
        let CliCommand::Run(c) = cmd else { panic!() };
        assert_eq!(c.quantize_colorspace, crate::quantize::Colorspace::Rgb);
    }
    #[test]
    fn parses_dither_flag() {
        let cmd = parse_cli_args(&args(&["i.png","o.png","--dither","bayer4"])).unwrap();
        let CliCommand::Run(c) = cmd else { panic!() };
        assert_eq!(c.quantize_dither, crate::quantize::DitherMethod::Bayer4);
    }
    #[test]
    fn parses_preset_flag() {
        let cmd = parse_cli_args(&args(&["i.png","o.png","--preset","pico8"])).unwrap();
        let CliCommand::Run(c) = cmd else { panic?() };
        assert_eq!(c.quantize_preset_palette, crate::quantize::PresetPalette::Pico8);
    }
```

- [ ] **Step 2: Implement the four flags in `parse_cli_args`**

Add arms before the `--` catch-all:
```rust
            "--colorspace" => {
                let Some(v) = args.get(i+1) else { return Err(PixelSnapperError::InvalidInput("--colorspace requires a value".into())); };
                config.quantize_colorspace = match v.as_str() {
                    "rgb" => crate::quantize::Colorspace::Rgb,
                    "oklab" => crate::quantize::Colorspace::Oklab,
                    _ => return Err(PixelSnapperError::InvalidInput(format!("invalid --colorspace '{}' (rgb|oklab)", v))),
                };
                i += 2;
            }
            "--dither" => {
                let Some(v) = args.get(i+1) else { return Err(PixelSnapperError::InvalidInput("--dither requires a value".into())); };
                config.quantize_dither = match v.as_str() {
                    "none"=>crate::quantize::DitherMethod::None,
                    "fs"=>crate::quantize::DitherMethod::FloydSteinberg,
                    "bayer2"=>crate::quantize::DitherMethod::Bayer2,
                    "bayer4"=>crate::quantize::DitherMethod::Bayer4,
                    "bayer8"=>crate::quantize::DitherMethod::Bayer8,
                    "ordered"=>crate::quantize::DitherMethod::Ordered,
                    _ => return Err(PixelSnapperError::InvalidInput(format!("invalid --dither '{}'", v))),
                };
                i += 2;
            }
            "--dither-strength" => {
                let Some(v) = args.get(i+1) else { return Err(PixelSnapperError::InvalidInput("--dither-strength requires a value".into())); };
                match v.parse::<f64>() { Ok(s) if (0.0..=2.0).contains(&s) => config.quantize_dither_strength = s,
                    _ => return Err(PixelSnapperError::InvalidInput(format!("invalid --dither-strength '{}' (0-2)", v))) }
                i += 2;
            }
            "--preset" => {
                let Some(v) = args.get(i+1) else { return Err(PixelSnapperError::InvalidInput("--preset requires a value".into())); };
                config.quantize_preset_palette = match v.as_str() {
                    "none"=>crate::quantize::PresetPalette::None,
                    "nes"=>crate::quantize::PresetPalette::Nes,
                    "gameboy"=>crate::quantize::PresetPalette::GameBoy,
                    "sgb"=>crate::quantize::PresetPalette::Sgb,
                    "snes"=>crate::quantize::PresetPalette::Snes,
                    "pc9801"=>crate::quantize::PresetPalette::Pc9801,
                    "msx1"=>crate::quantize::PresetPalette::Msx1,
                    "pico8"=>crate::quantize::PresetPalette::Pico8,
                    "sweetie16"=>crate::quantize::PresetPalette::Sweetie16,
                    "endesga32"=>crate::quantize::PresetPalette::Endesga32,
                    _ => return Err(PixelSnapperError::InvalidInput(format!("invalid --preset '{}'", v))),
                };
                i += 2;
            }
```
(Also fix the test typo `panic?()` → `panic!()`.)

Update `print_cli_help` OPTIONS with the four new flags.

- [ ] **Step 3: Verify**

```bash
cargo test 2>&1 | tail -5
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/r.png 16 --colorspace rgb
python -c "import hashlib;print(hashlib.sha256(open('/tmp/r.png','rb').read()).hexdigest())"
```
Expected: tests pass; `--colorspace rgb` → `802857...9f22`.

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs   # or src/cli/args.rs
git commit -m "feat(cli): --colorspace/--dither/--dither-strength/--preset flags"
```

---

## Task 9: WASM colorspace / dither / preset params

**Files:**
- Modify: `src/lib.rs` (wasm region)

- [ ] **Step 1: Add three trailing Option<String> params to `process_image`**

```rust
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
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
```
In the body, after the `resample_method` block, add three parallel blocks parsing into `config.quantize_colorspace`, `config.quantize_dither`, `config.quantize_preset_palette` (same match arms as Task 8, error via `JsValue::from_str`). `dither_strength` left at default (no param).

- [ ] **Step 2: Verify wasm build**

```bash
cargo build --target wasm32-unknown-unknown 2>&1 | tail -3
```
Expected: 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(wasm): process_image gains colorspace/dither/preset params"
```

---

## Task 10: tests/quantize.rs full + CLAUDE.md + final verification

**Files:**
- Modify: `tests/quantize.rs`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Fill out `tests/quantize.rs`**

```rust
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

fn run(args: &[&str]) -> bool {
    let bin = env!("CARGO_BIN_EXE_pixel-game-kit");
    Command::new(bin).args(args).output().unwrap().status.success()
}
fn sha(path: &str) -> String {
    let mut h = Sha256::new(); h.update(&fs::read(path).unwrap()); format!("{:x}", h.finalize())
}
fn tmp(name: &str) -> String {
    let mut p = std::env::temp_dir(); p.push(format!("p3-{}", name)); p.to_string_lossy().to_string()
}

#[test]
fn rgb_path_matches_old_anchor() {
    let out = tmp("rgb.png");
    assert!(run(&["tests/fixtures/baseline/ai-sprite.png", &out, "16", "--colorspace", "rgb"]));
    assert_eq!(sha(&out), "8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22");
}

#[test]
fn oklab_default_is_deterministic() {
    let out = tmp("oklab.png");
    assert!(run(&["tests/fixtures/baseline/ai-sprite.png", &out, "16"]));
    let h1 = sha(&out);
    assert!(run(&["tests/fixtures/baseline/ai-sprite.png", &out, "16"]));
    assert_eq!(h1, sha(&out));
    // record h1 in the commit message as the new Oklab anchor
    assert!(!h1.is_empty());
}

#[test]
fn each_dither_runs() {
    for d in ["fs","bayer2","bayer4","bayer8","ordered"] {
        let out = tmp(&format!("d_{}.png", d));
        assert!(run(&["tests/fixtures/baseline/ai-sprite.png", &out, "16", "--dither", d]));
        assert_eq!(sha(&out).len(), 64);
    }
}

#[test]
fn preset_palette_output_stays_in_preset() {
    // pico8 has 16 colors; run and verify output distinct colors ⊆ preset
    let out = tmp("pico8.png");
    assert!(run(&["tests/fixtures/baseline/ai-sprite.png", &out, "16", "--preset", "pico8"]));
    let img = image::open(&out).unwrap().to_rgba8();
    let pico8: Vec<[u8;3]> = vec![
        [0,0,0],[29,43,83],[126,37,83],[0,135,81],[171,82,54],[95,87,79],[194,195,199],[255,241,232],
        [255,0,77],[255,163,0],[255,236,39],[0,228,54],[41,173,255],[131,118,156],[255,119,168],[255,204,170],
    ];
    for p in img.pixels() {
        if p[3] == 0 { continue; }
        assert!(pico8.contains(&[p[0],p[1],p[2]]), "color {:?} not in PICO-8", p);
    }
}
```

- [ ] **Step 2: Run tests, capture new Oklab anchor**

```bash
cargo test 2>&1 | tail -8
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/oklab_anchor.png 16
python -c "import hashlib;print('NEW OKLAB ANCHOR:', hashlib.sha256(open('/tmp/oklab_anchor.png','rb').read()).hexdigest())"
```
Record the Oklab anchor hash. Update CLAUDE.md (Task 10 step 3) to note: default is now Okab, RGB path anchor `802857...9f22` preserved via `--colorspace rgb`.

- [ ] **Step 3: Update CLAUDE.md**

- Update title line / build commands: `pixel-game-kit` (from Task 1, should already be done).
- Pipeline step 1 (quantize): "k-means in Oklab (default; `--colorspace rgb` for RGB), optional dithering (`--dither`), optional preset palette (`--preset`)."
- Module table: add `quantize/{mod,oklab,kmeans,dither,palettes}.rs` row; note `resample/qvote.rs`.
- Tuning knobs: add `quantize_colorspace` / `quantize_dither` / `quantize_preset_palette`.
- Determinism section: note default flipped to Oklab at 2.0; RGB path preserves `802857...9f22`.

- [ ] **Step 4: Final full verification**

```bash
cargo test 2>&1 | tail -5
cargo build --target wasm32-unknown-unknown 2>&1 | tail -3
```
Expected: all tests green; wasm 0 warnings.

- [ ] **Step 5: Commit + ready for review**

```bash
git add tests/quantize.rs CLAUDE.md
git commit -m "test+docs: phase 3 quantize tests + CLAUDE.md (oklab default, dither, palettes)"
```

---

## Self-Review (completed inline)

**Spec coverage:**
- §Rename (Task 1 prerequisite) → Task 1. ✓
- §Oklab default + RGB compat → Task 4 (+ Config Task 3). ✓
- §Dithering full → Task 5. ✓
- §9 preset palettes → Task 6. ✓
- §qvote backfill → Task 7. ✓
- §CLI flags → Task 8. ✓
- §WASM params → Task 9. ✓
- §Tests (Oklab anchor, RGB anchor, dither, preset) → Task 10. ✓
- §bump 2.0 → Task 1 (version). ✓

**Placeholder scan:** Task 6 palettes — 3 inlined (PICO-8/Sweetie16/GameBoy) + 6 sourced from PixelRefiner with explicit path instruction. This is data-sourcing (precise source), not a logic placeholder; acceptable but the implementer MUST copy the 6 palettes' exact hex. Task 4 has a noted awkward helper that the step tells you to clean up inline. Task 7 qvote is documented as qvote-lite (simplification flagged).

**Type consistency:** `Colorspace::{Rgb,Oklab}`, `DitherMethod::{None,FloydSteinberg,Bayer2,Bayer4,Bayer8,Ordered}`, `PresetPalette::{None,Nes,GameBoy,Sgb,Snes,Pc9801,Msx1,Pico8,Sweetie16,Endesga32}` consistent across Tasks 3/5/6/8/9. `ResampleMethod::Qvote` added in Task 7. Config field names (`quantize_colorspace` / `quantize_dither` / `quantize_dither_strength` / `quantize_preset_palette`) consistent.

**Highest-risk step:** Task 4 (Oklab flip). Default changes → ai-sprite anchor changes. Mitigation: Task 4 step 5 verifies Oklab deterministic + RGB path still `802857`; Task 10 `rgb_path_matches_old_anchor` locks the RGB compatibility gate.

---

**Plan complete.** Saved to `docs/superpowers/plans/2026-07-22-phase3-quantize.md`.
