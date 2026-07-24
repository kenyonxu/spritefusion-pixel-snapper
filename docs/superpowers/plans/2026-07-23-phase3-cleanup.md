# Phase 3 Cleanup + qvote Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`).

**Goal:** Fix the bayer8 non-standard matrix, clear native unused-import warnings, and upgrade qvote from lite (≈majority) to true per-cell Oklab k-means.

**Architecture:** Three independent fixes + a verification task. Each is small and behavior-isolated (default config anchors stay invariant; only the specific flag's output changes).

**Tech Stack:** Rust 2021, `image` 0.24, `wasm-bindgen`. No new deps.

**Spec:** [docs/superpowers/specs/2026-07-23-phase3-cleanup-design.md](../specs/2026-07-23-phase3-cleanup-design.md)

**Anchors (must stay invariant under default config):** Oklab default `3a589ee93b8cd2e493baa0d6fb314d279b54a1104165ad754ad4ff6d359e4420`; RGB `--colorspace rgb` `8028577762af407b84ce6edb38bf60491973e246c2326dad9f6c7fe8434c9f22`.

---

## Task 1: bayer8 standard matrix

**Files:** Modify `src/quantize/dither.rs`

- [ ] **Step 1:** In `bayer_matrix(size)`, replace the `_ => { … recursion … }` arm with a hardcoded canonical 8×8:

```rust
        _ => vec![
            vec![ 0,48,12,60, 3,51,15,63],
            vec![32,16,44,28,35,19,47,31],
            vec![ 8,56, 4,52,11,59, 7,55],
            vec![40,24,36,20,43,27,39,23],
            vec![ 2,50,14,62, 1,49,13,61],
            vec![34,18,46,30,33,17,45,29],
            vec![10,58, 6,54, 9,57, 5,53],
            vec![42,26,38,22,41,25,37,21],
        ],
```

(The trailing `.into_iter().map(|row| row.into_iter().map(|v| v / (size*size) as f32).collect()).collect()` already normalizes — for size=8 that's /64, correct.)

- [ ] **Step 2:** Verify build + default anchor unchanged:
```
cargo build 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/c1.png 16
python -c "import hashlib;print(hashlib.sha256(open('/tmp/c1.png','rb').read()).hexdigest())"
```
Expected: builds; default anchor = `3a589ee9…e4420` (dither default none, unaffected).

- [ ] **Step 3:** Smoke-test bayer8 now produces standard pattern:
```
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/c1b8.png 16 --dither bayer8
```
Expected: runs without error (pattern correctness is visual; a locked-hash test is optional).

- [ ] **Step 4:** Commit: `git add src/quantize/dither.rs && git commit -m "fix(dither): standard Bayer 8x8 matrix (was non-standard recursion)"`

---

## Task 2: native unused-import warnings

**Files:** Modify `src/cli/mod.rs`, `src/lib.rs` (audit each warning)

- [ ] **Step 1:** Run `cargo build 2>&1 | grep -A3 "warning.*unused"` to list every unused-import warning with file:line.
- [ ] **Step 2:** For each warning:
  - If the item is genuinely unused (no `cli_tests` consumer via `use super::*`, no other consumer): delete the import / re-export line.
  - If `cli_tests` consumes it via glob but rustc still flags the re-export: check whether `cli_tests` is `mod cli_tests;` (sibling) vs inside `args.rs`. If the re-export exists only for test glob convenience, either (a) move the test to import directly `use crate::cli::args::{parse_cli_args, CliCommand};`, or (b) add `#[allow(unused_imports)]` on the re-export with a `// used by cli_tests via glob` comment.
  - `src/lib.rs:25` `parse_palette_hex`: check if `process_image` (wasm) or any native fn uses it. If not, delete from the `use palette::{…}` line (keep `apply_palette`).
- [ ] **Step 3:** Verify `cargo build` (native) = **0 warnings**:
```
cargo build 2>&1 | grep -c warning
```
Expected: 0. Also `cargo test` still green (30+).
- [ ] **Step 4:** Commit: `git add -A src/ && git commit -m "chore: clear native unused-import warnings"`

---

## Task 3: qvote true implementation (per-cell Oklab k-means)

**Files:** Modify `src/resample/qvote.rs`

- [ ] **Step 1:** Rewrite `resample_qvote` body (keep signature). Replace the lite whole-pixel vote with per-cell Oklab k-means:

```rust
use crate::error::{PixelSnapperError, Result};
use crate::quantize::oklab;
use crate::Config;
use image::{ImageBuffer, Rgba, RgbaImage};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

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

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let (ys, ye) = (w_y[0], w_y[1]);
            let (xs, xe) = (w_x[0], w_x[1]);
            if xe <= xs || ye <= ys {
                continue;
            }
            // collect opaque pixels → Oklab
            let mut pts: Vec<[f32; 3]> = Vec::new();
            let mut alphas: Vec<u8> = Vec::new();
            for y in ys..ye {
                for x in xs..xe {
                    if x < iw && y < ih {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        if p[3] >= 16 {
                            pts.push(oklab::rgb_to_oklab(p[0], p[1], p[2]));
                            alphas.push(p[3]);
                        }
                    }
                }
            }
            let pixel = if pts.is_empty() {
                [0, 0, 0, 0]
            } else {
                let k = 4.min(pts.len());
                // deterministic per-cell seed
                let cell_seed = config
                    .seed
                    .wrapping_add((y_i as u64).wrapping_mul(0x9E3779B97F4A7C15))
                    .wrapping_add(x_i as u64);
                let mut rng = ChaCha8Rng::seed_from_u64(cell_seed);
                // init: k random distinct points
                use rand::seq::SliceRandom;
                let mut idxs: Vec<usize> = (0..pts.len()).collect();
                idxs.shuffle(&mut rng);
                let mut centroids: Vec<[f32; 3]> = idxs.iter().take(k).map(|&i| pts[i]).collect();
                // 5 iterations of k-means
                for _ in 0..5 {
                    let mut sums = vec![[0f32; 3]; k];
                    let mut counts = vec![0usize; k];
                    for p in &pts {
                        let mut best = 0;
                        let mut best_d = f32::MAX;
                        for (i, c) in centroids.iter().enumerate() {
                            let d = (p[0]-c[0]).powi(2)+(p[1]-c[1]).powi(2)+(p[2]-c[2]).powi(2);
                            if d < best_d { best_d = d; best = i; }
                        }
                        for ch in 0..3 { sums[best][ch] += p[ch]; }
                        counts[best] += 1;
                    }
                    for i in 0..k {
                        if counts[i] > 0 {
                            for ch in 0..3 { centroids[i][ch] = sums[i][ch] / counts[i] as f32; }
                        }
                    }
                }
                // vote: largest cluster
                let mut assign_counts = vec![0usize; k];
                for p in &pts {
                    let mut best = 0;
                    let mut best_d = f32::MAX;
                    for (i, c) in centroids.iter().enumerate() {
                        let d = (p[0]-c[0]).powi(2)+(p[1]-c[1]).powi(2)+(p[2]-c[2]).powi(2);
                        if d < best_d { best_d = d; best = i; }
                    }
                    assign_counts[best] += 1;
                }
                let winner = assign_counts.iter().enumerate().max_by_key(|(_, c)| **c).map(|(i, _)| i).unwrap_or(0);
                let rgb = oklab::oklab_to_rgb(centroids[winner][0], centroids[winner][1], centroids[winner][2]);
                // alpha: median of the winning cluster (or just max alpha)
                [rgb[0], rgb[1], rgb[2], alphas.iter().copied().max().unwrap_or(255)]
            };
            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(pixel));
        }
    }
    Ok(final_img)
}
```

(If `rand::seq::SliceRandom` isn't in scope, add `use rand::seq::SliceRandom;` at top. `rand` crate is already a dep.)

- [ ] **Step 2:** Verify build + determinism + distinctness:
```
cargo build 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/q1.png 16 --resample qvote
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/q2.png 16 --resample qvote
python -c "import hashlib;a=hashlib.sha256(open('/tmp/q1.png','rb').read()).hexdigest();b=hashlib.sha256(open('/tmp/q2.png','rb').read()).hexdigest();print('deterministic:',a==b);print(a[:16])"
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/maj.png 16 --resample majority
python -c "import hashlib;print('qvote != majority:', hashlib.sha256(open('/tmp/q1.png','rb').read()).hexdigest() != hashlib.sha256(open('/tmp/maj.png','rb').read()).hexdigest())"
```
Expected: builds; deterministic (q1==q2); qvote != majority (qvote now distinct).

- [ ] **Step 3:** Default anchor check (majority default unchanged):
```
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/d.png 16
python -c "import hashlib;print(hashlib.sha256(open('/tmp/d.png','rb').read()).hexdigest()=='3a589ee93b8cd2e493baa0d6fb314d279b54a1104165ad754ad4ff6d359e4420')"
```
Expected: True.

- [ ] **Step 4:** Commit: `git add src/resample/qvote.rs && git commit -m "feat(resample): qvote true per-cell Oklab k-means (was lite)"`

---

## Task 4: final verification + CLAUDE.md

**Files:** Modify `CLAUDE.md` (update qvote note)

- [ ] **Step 1:** Full verify:
```
cargo test 2>&1 | tail -5
cargo build 2>&1 | grep -c warning   # native, expect 0
cargo build --target wasm32-unknown-unknown 2>&1 | tail -3
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/f.png 16
python -c "import hashlib;print(hashlib.sha256(open('/tmp/f.png','rb').read()).hexdigest())"  # 3a589ee9
cargo run --release -q -- tests/fixtures/baseline/ai-sprite.png /tmp/fr.png 16 --colorspace rgb
python -c "import hashlib;print(hashlib.sha256(open('/tmp/fr.png','rb').read()).hexdigest())"  # 802857
```
Expected: tests green; native 0 warnings; wasm 0 warnings; oklab `3a589ee9…`; rgb `802857…`.
- [ ] **Step 2:** Update CLAUDE.md resample note — qvote is no longer "lite": change any "qvote-lite / ≈majority" text to "per-cell Oklab k-means". Update PLAN.md Phase 3 遗留 to mark bayer8/warning/qvote resolved (or note this cleanup branch).
- [ ] **Step 3:** Commit: `git add CLAUDE.md PLAN.md && git commit -m "docs: phase 3 cleanup complete (bayer8/warning/qvote resolved)"`

---

## Self-Review

**Spec coverage:** bayer8 (Task 1), warnings (Task 2), qvote (Task 3), verify+docs (Task 4). ✓
**Anchor safety:** default (dither none + resample majority) never touched in any task → Oklab `3a589ee9` and RGB `802857` stay invariant. ✓
**Determinism:** qvote per-cell seed derived from `config.seed` + cell coords (Task 3 step 1) — no global RNG. ✓
**Risk:** Task 2 warning audit is the only one that could break tests (deleting a re-export cli_tests needs) — Step 3 runs `cargo test` after each deletion. ✓
