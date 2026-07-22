# Phase 1 Design: Detector Diversity

**Date:** 2026-07-22
**Status:** Approved (brainstormed)
**Related:** [PLAN.md](../../../PLAN.md) Phase 1 · [USER_STORIES.md](../../../USER_STORIES.md) U2.1–U2.5 · [CONFIG.md](../../CONFIG.md) `detect` schema

## Background

Phase 0 split `lib.rs` into 11 modules, but grid detection still relies on a single gradient walker (`estimate_step_size` + `walk`). Compared with unfake.js: **runs (GCD)** is most accurate and fastest on clean integer-scaled pixel art; **tiled (Sobel + autocorrelation)** is robust on complex backgrounds. spritefusion's **elastic walker** is uniquely good at skew / non-integer steps. The three are complementary. Phase 1 brings runs and tiled in alongside elastic, behind an Auto selector.

## Goals

- Add `runs` and `tiled` integer-scale detectors to complement the existing `elastic` walker.
- `detect` returns a **candidate list** (candidate API), paving the way for U2.2 (Web candidate selection) without future API churn.
- `Auto` strategy runs all three and selects best by priority + confidence.
- Zero behavioral regression on existing output (sha256 anchor preserved).

## Non-Goals

- The Web candidate UI itself (Phase 6).
- Exposing raw profile / peak data for debug visualization (YAGNI).
- Changing resample / quantize (later phases).

## Decisions

1. **Scope = algorithm + candidate API.** `detect` returns `Vec<DetectionCandidate>`, not a single step. This serves U2.2 and avoids a costly return-type change later.
2. **Pipeline integration = integer-scale uniform cut, skip walker.** When runs/tiled hit an integer scale, the pipeline uses `snap_uniform_cuts` (uniform grid) and does **not** invoke the elastic walker. elastic still goes through `walk` + `stabilize_both_axes`. Each detector keeps its precision advantage.
3. **Auto = run-all + priority selection.** All three detectors run; candidates are merged; selection priority is Runs > Tiled > Elastic, tie-broken by confidence. Gives a rich candidate list for U2.2 while keeping the decision fast.

## Architecture

### Module layout

New `src/detect/` directory:

- `mod.rs` — `DetectStrategy` enum, `CutMethod` enum, `DetectionCandidate` struct, `detect()` dispatch entry, `select_best()` Auto selection.
- `runs.rs` — `detect_runs(img, config) -> Option<DetectionCandidate>`.
- `tiled.rs` — `detect_tiled(img, config) -> Option<DetectionCandidate>`.
- `elastic.rs` — `detect_elastic(profile_x, profile_y, w, h, config) -> Option<DetectionCandidate>`.

**Separation of concerns:** `detect` only detects and returns candidates; it does **not** perform the cut. Cutting stays in the pipeline. `walk` is **not** migrated — it stays in `stabilize.rs`, invoked by the pipeline when the chosen candidate is `Walker`.

### Data flow (process_image_common change)

```
compute_profiles (existing) → profile_x, profile_y
  ↓
detect::detect(img, &profiles, config, strategy) → Vec<DetectionCandidate>
  ↓ (if --pixel-size override: skip detect, synthesize Walker candidate with step=N)
select_best(candidates, strategy) → DetectionCandidate
  ↓
branch on candidate.cut_method:
  Uniform(scale) → snap_uniform_cuts (integer grid, existing in stabilize.rs)
  Walker(step)   → walk + stabilize_both_axes (existing elastic path)
  ↓
resample (unchanged)
```

## Data Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectStrategy { Auto, Runs, Tiled, Elastic }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutMethod { Uniform, Walker }

#[derive(Debug, Clone)]
pub struct DetectionCandidate {
    pub detector: DetectStrategy,  // Runs | Tiled | Elastic (Auto never appears in a candidate)
    pub scale: Option<usize>,      // integer scale (Runs/Tiled); None for Elastic
    pub step: f64,                 // continuous step (Elastic); Runs/Tiled = scale as f64
    pub confidence: f64,           // 0.0..=1.0
    pub cut_method: CutMethod,     // Uniform (Runs/Tiled) | Walker (Elastic)
}
```

## Detectors

### runs (← unfake `runs.rs` + PixelRefiner posterize)
1. **posterize(img, 64)** — quantize each channel to 4 buckets, suppressing single-pixel noise (mitigates the GCD off-by-one fragility flagged in PLAN).
2. Collect horizontal and vertical same-color run lengths.
3. GCD of run lengths → integer `scale`.
4. If run count < `runs_min_runs` (default 10) → return `None`.
5. On hit → `DetectionCandidate { detector: Runs, scale: Some(s), step: s as f64, confidence, cut_method: Uniform }`.
   - confidence: derived from run-length consistency (low variance across runs → high confidence).

### tiled (← unfake `edge.rs`)
1. Split image into 3×3 tiles with 25% overlap.
2. Drop tiles with stddev < `tiled_stddev_threshold` (default 5).
3. Sobel operator per tile → edge-strength profile.
4. `peak_lag` autocorrelation (max_lag = min(n/8, 128), 0.6·gmax threshold) → per-tile scale.
5. Mode vote across tiles.
6. On hit → `DetectionCandidate { detector: Tiled, scale, step: scale as f64, confidence, cut_method: Uniform }`.
   - confidence: vote agreement rate.

### elastic (wrap of existing path)
1. `compute_profiles` (profile.rs).
2. `estimate_step_size` (profile.rs) → `step_x`, `step_y` (Options).
3. `resolve_step_sizes` (profile.rs) → unified `step` (with skew correction).
4. If either axis fails to detect → `None` (pipeline then falls back to uniform fallback grid).
5. On hit → `DetectionCandidate { detector: Elastic, scale: None, step, confidence, cut_method: Walker }`.
   - confidence: derived from peak strength vs threshold.

## Auto Selection

`select_best(candidates, strategy)`:
- **Manual strategy** (Runs/Tiled/Elastic): filter to that detector's candidates, return top-N (N default 3).
- **Auto**: sort candidates by `(priority, -confidence)` where priority `Runs=0 < Tiled=1 < Elastic=2` (lower wins), tie broken by higher confidence.
- The selected candidate = first after sort.
- The full candidate list is still returned (for `--json` / WASM).

`detect()` dispatch: Auto runs all three detectors and merges candidates; manual runs only the specified one.

## Interface

### CLI
- `--detect auto|runs|tiled|elastic` (default `auto`).
- `--json`: output includes `candidates: [{detector, scale, step, confidence, cut_method, selected}]`.
- `--pixel-size N`: unchanged, highest priority (skips detect, synthesizes Walker candidate with step=N).

### WASM
- `process_image(...)`: add `detect_strategy: Option<String>` parameter (backward compatible — `None`/absent = auto).
- New export `detect_candidates(input_bytes, config?) -> Result<Vec<DetectionCandidate>>` (serialized to JsValue, serves U2.2 Web candidate UI).

### Config (aligns with existing schema in CONFIG.md)
`detect.strategy` / `detect.runs_min_runs` / `detect.tiled_stddev_threshold` / `detect.tiled_peak_ratio` / `detect.skew_tolerance`.
- No RNG needed for confidence (deterministic numeric).

## Fixtures & Tests

New fixtures under `tests/fixtures/baseline/` (self-made or borrowed):
- `clean.png` — clean integer-scaled pixel art (runs should hit).
- `complex-bg.png` — multicolor complex background (tiled should hit).
- `skewed.png` — non-integer / skewed grid (elastic should hit).

Integration tests (`tests/detect.rs`):
- `runs_detects_clean` — clean.png → selected detector == Runs.
- `tiled_detects_complex` — complex-bg.png → Tiled.
- `elastic_detects_skew` — skewed.png → Elastic.
- `auto_picks_correct` — all three fixtures select correctly under Auto.
- `manual_strategy_respected` — `--detect runs` on skewed.png still returns Runs candidate (even if not optimal).
- `pixel_size_override_skips_detect` — `--pixel-size` bypasses detect.
- sha256 regression — `ai-sprite.png` (existing) + three new fixtures, default-config output locked.

## Incidental

`cli.rs` (553 lines) → split into `cli/args.rs` (parse_cli_args + CliCommand + run_cli + cli_tests) + `cli/batch.rs` (BatchConfig/Event + process_batch_with_reporter + helpers). Each file < 400 lines. Phase 0 leftover, done here to keep the codebase clean while touching the area.

## Acceptance

- Three detectors hit their respective fixtures.
- Auto selects correctly on all three fixtures.
- Existing `ai-sprite.png` sha256 unchanged (default config = zero behavioral regression).
- `cargo test` green; `cargo build --target wasm32` 0 warnings.
- CLI `--detect` / `--json` work; WASM `process_image` stays backward compatible (`detect_strategy` optional).

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| GCD noise sensitivity | posterize(64) before runs detection |
| tiled max_lag=128 misses very large scales | document limit; fall back to elastic for huge images |
| Auto run-all cost | all three detectors are O(pixels); acceptable; `--pixel-size` skips detect entirely |
| Candidate API churn risk | designed as candidate list now; U2.2 needs no change |
| cli split regression | sha256 anchor + cli_tests kept green |

## Mapping

- **PLAN Phase 1**: covers all tasks (detect/ directory, runs/tiled/elastic, Auto, CLI/WASM, fixtures, CLAUDE.md update).
- **USER_STORIES**: U2.1 (auto detect 🔴), U2.2 (candidate API enables Web 🔴), U2.4 (detect feedback 🔴), U2.5 (force detector 🟡).
- **CONFIG.md**: detect schema already defined; this spec aligns field names.
- **Phase 0 leftover**: cli.rs split (incidental).
