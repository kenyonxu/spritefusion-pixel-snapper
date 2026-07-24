# Phase 3 Cleanup + qvote Upgrade Design

**Date:** 2026-07-23
**Status:** Draft (awaiting review)
**Related:** [PLAN.md](../../../PLAN.md) Phase 3 实施记录（遗留项）

## Background

Phase 3 landed with three documented leftover items (see PLAN.md Phase 3 实施记录 — 遗留). This spec cleans them up.

## Scope — three items

### 1. bayer8 standard matrix (bug fix)

**Problem:** `src/quantize/dither.rs::bayer_matrix(8)` recurses on the *normalized* 4×4 matrix (already divided by 16), producing a non-standard 8×8 pattern. `--dither bayer8` output is therefore wrong (not the canonical Bayer ordered-dither pattern).

**Fix:** hardcode the canonical Bayer 8×8 integer matrix (0–63, the standard ordered-dither table) and normalize by /64. No recursion.

```rust
fn bayer8_raw() -> [[u32; 8]; 8] {
    [
        [ 0,48,12,60, 3,51,15,63],
        [32,16,44,28,35,19,47,31],
        [ 8,56, 4,52,11,59, 7,55],
        [40,24,36,20,43,27,39,23],
        [ 2,50,14,62, 1,49,13,61],
        [34,18,46,30,33,17,45,29],
        [10,58, 6,54, 9,57, 5,53],
        [42,26,38,22,41,25,37,21],
    ]
}
```
`bayer_matrix(8)` returns this normalized (`v as f32 / 64.0`). `bayer_matrix(2)` and `(4)` stay as-is (they were already correct).

**Anchor impact:** default `--dither none` → no change to Oklab anchor `3a589ee9…e4420`. Only `--dither bayer8` output changes (now correct).

### 2. native unused-import warnings (cleanup)

**Problem:** `cargo build` (native, not wasm) emits ~3 warnings: `src/cli/mod.rs` re-exports (`parse_cli_args`, `CliCommand`, `BatchConfig`, etc.) and `src/lib.rs` `parse_palette_hex` — flagged unused.

**Fix:** audit each. Two cases:
- **Truly unused** (no consumer, not even `cli_tests` via `use super::*`): delete the re-export / import line.
- **Used only by `cli_tests` via glob**: keep but the warning indicates the glob isn't pulling it — either restructure so `cli_tests` imports directly, or add targeted `#[allow(unused_imports)]` with a comment explaining the test-only consumer.

Goal: `cargo build` (native) **0 warnings**, matching the wasm gate (`cargo build --target wasm32` is already 0).

**Anchor impact:** none (pure import hygiene).

### 3. qvote true implementation (per-cell Oklab k-means)

**Problem:** `src/resample/qvote.rs` is currently qvote-*lite* (whole-pixel vote ≈ `majority`) — no independent value. The spec called for per-cell Oklab k-means + vote on the dominant cluster.

**Fix:** rewrite `resample_qvote`:
1. For each cell, collect opaque pixels (alpha ≥ 16).
2. Convert each to Oklab (`crate::quantize::oklab::rgb_to_oklab`).
3. Run k-means with `k = min(4, n)` (seeded — derive per-cell seed from `config.seed` + cell index for determinism, or reuse the global seeded rng advanced per cell).
4. Find the cluster with the most members (vote).
5. Output that cluster's centroid, converted back to RGB (`oklab_to_rgb`) — round to nearest u8.

This gives each cell a "dominant cluster representative" color, distinct from `majority` (raw whole-pixel mode) and `dominant` (top-color-threshold-or-mean).

**Determinism (R1):** k-means must be seeded (ChaCha8Rng from `config.seed`, advanced deterministically per cell — e.g. `ChaCha8Rng::seed_from_u64(config.seed.wrapping_add((row_idx as u64) << 32 | col_idx as u64))`). No `Math.random`-equivalent.

**Performance:** O(cells × pixels_per_cell × k × iterations). Cells are small (post-resample grid); acceptable. If profiling shows a hotspot, cap k=2 or iterations=3.

**Anchor impact:** default `--resample majority` → no change. Only `--resample qvote` output changes (now distinct from majority).

## Non-Goals

- Local directory rename (`spritefusion-pixel-snapper` → `pixel-game-kit` on disk) — user does this manually (`mv`), no code change.
- SGB/SNES palette no-ops — no canonical palette exists; stay as documented None.
- qvote full Oklab k-means is the scope ceiling — no content-adaptive EM (still Phase 2-deferred).

## Acceptance

- `cargo build` (native) **0 warnings**; `cargo build --target wasm32` **0 warnings**.
- `--dither bayer8` produces the canonical Bayer 8×8 pattern (visual check or locked fixture).
- `--resample qvote` output differs from `--resample majority` on ai-sprite (qvote is no longer a clone).
- Default anchors unchanged: Oklab `3a589ee9…e4420`, RGB `802857…9f22` (no default-config regression).
- `cargo test` all green (30+).

## Risks

| Risk | Mitigation |
|------|------------|
| bayer8 fix changes bayer8 fixture hash | expected; re-lock the bayer8 test hash |
| native warning audit removes a re-export cli_tests needs | run `cargo test` after each deletion |
| qvote per-cell k-means slow on large images | cap k=2 / iters=3 if profiling flags it |
| qvote determinism subtle bug (per-cell seed) | determinism test: run twice, assert byte-identical |
