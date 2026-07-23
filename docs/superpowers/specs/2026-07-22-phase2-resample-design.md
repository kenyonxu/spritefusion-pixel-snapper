# Phase 2 Design: Resample Strategies

**Date:** 2026-07-22
**Status:** Approved (brainstormed)
**Related:** [PLAN.md](../../../PLAN.md) Phase 2 · [USER_STORIES.md](../../../USER_STORIES.md) U3.1–U3.3 · [CONFIG.md](../../CONFIG.md) `resample` schema

## Background

Phase 1 added multi-detector grid detection. After cuts are placed, each grid cell is reduced to one pixel. Phase 0/1 kept the original **majority vote** (whole-pixel mode + RGBA tiebreak). That is safe but cannot remove anti-aliasing (median does that better) and does not favor dominant colors in sparse sprites. unfake.js ships median / dominant / mode / qvote / content-adaptive; PixelRefiner ships median (its AA-removal workhorse). Phase 2 brings the three most valuable of these in alongside majority, behind a `ResampleMethod` selector. EM (content-adaptive) and qvote are deferred — EM is computationally heavy and needs a feature gate; qvote depends on the not-yet-built Phase 3 Oklab quantizer.

## Goals

- Add `median` (AA removal), `dominant` (sparse-sprite edge preservation), and `mode` (per-channel) alongside the existing `majority`.
- `resample.rs` → `resample/` directory with one file per strategy.
- Keep `majority` as default → zero behavioral regression on `ai-sprite.png` sha256.
- CLI `--resample` + WASM `resample_method` parameter (backward compatible).

## Non-Goals

- `content_adaptive` (Öztireli-Gross EM) — deferred to a later phase (heavy, feature-gated).
- `qvote` — deferred; depends on Phase 3 Oklab k-means to replace imagequant.
- Changing the default method (stays `majority`).
- Changing quantize / palette paths.

## Decisions

1. **Scope = four core strategies.** `majority` + `median` + `dominant` + `mode`. EM and qvote deferred (see Non-Goals).
2. **Default = `majority`**, unchanged. Satisfies U3.1 (default majority output identical to现状) and preserves the sha256 anchor (R1).
3. **`mode` is per-channel** (R/G/B each reduced independently). Kept for completeness because the user scoped it in, **with a documented caveat**: per-channel mode can combine into a color not present in the source (e.g. R-mode + G-mode + B-mode). `majority` (whole-pixel) stays the safe default.
4. **`dominant` alpha binarization defaults OFF.** Half-transparent edges are preserved unless explicitly enabled (`resample_dominant_binarize_alpha`). Pixel-art hardening is opt-in to avoid destroying soft sprites.

## Architecture

### Module layout

Convert `src/resample.rs` → `src/resample/` directory:

- `mod.rs` — `ResampleMethod` enum + `resample()` dispatch entry.
- `majority.rs` — existing whole-pixel majority vote (moved as-is).
- `median.rs` — per-channel median with sample window.
- `dominant.rs` — dominant color with mean fallback + optional alpha binarize.
- `mode.rs` — per-channel mode.

Each strategy file exposes `pub(crate) fn resample_<method>(img, cols, rows, config) -> Result<RgbaImage>`. `mod.rs::resample()` branches on `config.resample_method`.

### Data flow

`process_image_common` after cut placement → `resample::resample(&analysis_img, &col_cuts, &row_cuts, &config)` → branch on `config.resample_method` → sub-module → `RgbaImage`. The surrounding pipeline (quantize before, palette after) is unchanged.

## Data Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    Majority,
    Median,
    Dominant,
    Mode,
}
```

Config additions (snake_case, aligns with `resample` schema in CONFIG.md):

```rust
pub(crate) resample_method: ResampleMethod,               // default Majority
pub(crate) resample_sample_window: usize,                 // median neighborhood, default 3 (range 1-9)
pub(crate) resample_dominant_threshold: f64,              // default 0.15
pub(crate) resample_dominant_binarize_alpha: bool,        // default false
```

## Strategies

### majority (existing, moved to `majority.rs`)
For each cell: `HashMap<[u8;4], usize>` count, pick max count, tie-break by `RGBA` ordering (deterministic). Behavior identical to today — this is the move-only step that preserves the sha256 anchor.

### median (← PixelRefiner `processor.ts` downsample)
For each cell, for each channel (R/G/B and A), collect the values of opaque pixels (alpha ≥ 16) within a `sample_window × sample_window` neighborhood centered on the cell, sort, take the middle value. If no opaque pixels in the window, fall back to all pixels in the cell. This suppresses anti-aliased edges (the dominant use case, U3.2). `sample_window = 1` degenerates to center-pixel sampling (alias preserved — useful for compare views).

### dominant (← unfake `downscale.rs`)
For each cell: count pixels by whole-pixel value. If the top color's count / cell-pixel-count ≥ `resample_dominant_threshold` (default 0.15), emit the top color; otherwise emit the per-channel mean of opaque pixels. If `resample_dominant_binarize_alpha` is on, output alpha is forced to 0 or 255 (thresholded). Preserves crisp edges on sparse palettes; the mean fallback prevents speckle in noisy cells.

### mode (← unfake, per-channel)
For each cell, for each channel independently: count values, take the mode (highest-count value), tie-break by lowest value (deterministic). Combine the three channel-modes (and alpha mode) into the output pixel. **Caveat (Decision 3):** the combined color may not exist in the source. Documented in `--help` and CLAUDE.md.

## Interface

### CLI
- `--resample <majority|median|dominant|mode>` (default `majority`).
- `--sample-window <n>` (1-9, median only; ignored by other methods with a note).
- Existing `[COLORS]`, `--pixel-size`, `--palette`, `--detect` unchanged.

### WASM
- `process_image(...)`: add `resample_method: Option<String>` parameter (backward compatible — `None`/absent = `majority`).
- No new standalone export needed in Phase 2 (candidates-style API was a Phase 1 concern; resample method is a simple config field).

### Config (aligns with CONFIG.md `resample` schema)
Field names above. The schema's `resample.method` enum gains these four variants (content_adaptive stays in schema as a documented future variant, not wired in Phase 2).

## Tests

Integration tests in `tests/resample.rs` (new):
- `majority_is_default_and_matches_anchor` — `ai-sprite.png` with default config → sha256 `802857…9f22` (zero regression).
- `median_smooths_aa_edges` — an AA-edges fixture → median output locked to a sha256; manually verified sharper than majority on the same input.
- `dominant_preserves_sparse_sprite` — a 4-color sprite fixture → dominant output locked.
- `mode_emits_per_channel` — a fixture where per-channel mode produces a color majority would not → locked + comment explaining.
- `manual_method_respected` — `--resample median` actually routes to median (assert via `--json` or output diff vs majority).

Fixtures: reuse `tests/fixtures/baseline/ai-sprite.png` (majority anchor) + `clean.png` (dominant/mode) + add `aa-edges.png` (median). All checked in; outputs under `expected/` gitignored (regenerated + sha256-asserted in-test).

## Acceptance

- Default `majority` → `ai-sprite.png` sha256 unchanged (`802857…9f22`).
- `median` visibly reduces AA on the AA-edges fixture.
- `dominant` preserves edges on a sparse sprite.
- `mode` produces per-channel-mode output (documented caveat respected).
- `cargo test` green; `cargo build --target wasm32` 0 warnings; `--resample` / WASM `resample_method` work.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| `median` window cost on large cells | window clamped to cell bounds; O(cell × window²) acceptable |
| `mode` emits non-source colors, surprising users | documented in `--help` + CLAUDE.md; `majority` stays default |
| `dominant` mean fallback introduces blur | only triggers when no color clears threshold (noisy cells); threshold tunable |
| resample split breaks anchor | `majority.rs` is a byte-for-byte move; first test locks sha256 before any new strategy lands |
| `resample_method` param breaks WASM JS callers | added as trailing `Option<String>`, `None` = majority = current behavior |

## Mapping

- **PLAN Phase 2**: covers majority/median/dominant/mode + dispatch + Config + CLI/WASM + tests. Defers EM (documented) and qvote (depends on Phase 3).
- **USER_STORIES**: U3.1 (default majority 🔴), U3.2 (median AA removal 🔴), U3.3 (multi-strategy choice 🟡). U3.4 (content-adaptive 🟢) explicitly deferred.
- **CONFIG.md**: `resample` schema aligned; `content_adaptive` variant left as documented future.
- **Phase 0/1 precedent**: same TDD + sha256-anchor discipline; `resample/` directory follows the `detect/` pattern.
