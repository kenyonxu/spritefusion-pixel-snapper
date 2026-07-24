# Phase 4 — Post-processing Suite Design

**Date:** 2026-07-24
**Status:** Draft (awaiting review)
**Related:** [PLAN.md](../../../PLAN.md) Phase 4 — 后处理全家桶

## Background

Phase 0–3 delivered the snap pipeline (detect → cut → resample → quantize). Phase 4 adds a
**postprocess** stage so output is directly usable in a game engine: background removal, outline,
morphology cleanup, and alpha binarization. All four ops are **off by default** → zero regression
on existing anchors; the stage is purely additive and opt-in.

### Source mapping (clean-room, R3 no-GPL)

| Module | Inspiration | Path |
|--------|-------------|------|
| `floodfill.rs` | PixelRefiner `floodfill.ts` + `processor.ts` (bg removal + floating-island) | `E:\GitHub\PixelRefiner\src\core\` |
| `outline.rs` | PixelRefiner `outline.ts` | same |
| `morphology.rs` | unfake.js `morphology.rs` (2×2 open→close) | `E:\GitHub\unfake.js\crates\unfake-core\src\morphology.rs` |
| `alpha.rs` | unfake.js `downscale.rs:219` (hard `>128`) + **Otsu (fresh impl)** | `E:\GitHub\unfake.js\crates\unfake-core\src\downscale.rs` |

Algorithm logic is re-implemented in Rust; no source code is copied. Three deviations from source
are documented below (§Algorithms) — two are project-identity choices, one is a consistency fix.

## Scope — four ops, all optional

1. **Background removal** — stack-based flood-fill (per-channel tolerance, 4/8-way) + floating-island
   cleanup (4-conn CCL, largest survives).
2. **Outline** — pad canvas +1px/side, draw 1px border around opaque regions (sharp=4-way /
   rounded=8-way).
3. **Morphology** — 2×2 open→close, **alpha-only** (palette-preserving).
4. **Alpha binarize** — fixed strict threshold (`>128` default) **or** Otsu adaptive (fresh impl,
   "超越原版").

## Non-Goals

- **Flood-fill `Selected` scope** (user-picked seed) — deferred to Phase 6 when the Web UI has a
  selection mechanism. CLI/WASM have no selection input source today.
- **Multi-pixel outlines** — source supports only 1px; stay 1px.
- **Per-channel / RGB morphology** — alpha-only by design (see §Morphology deviation).
- **Configurable morph kernel size / open-only / close-only variants** — YAGNI; ship the standard
  open→close 2×2 only.
- **Morphology on RGB to clean color noise** — already handled by Phase 3 k-means quantize upstream.
- **Version bump** — additive opt-in, default config unchanged, no breaking change.

## Architecture

### Pipeline slot

Inserted in [`process_image_common`](../../../src/lib.rs) after `apply_palette`, before PNG encode:

```rust
let output_img = match config.palette.as_deref() {
    Some(palette) => apply_palette(&snapped_img, palette)?,
    None => snapped_img,
};
let output_img = postprocess::postprocess(output_img, &config)?;  // ← new
// ...encode output_img to PNG...
```

`ProcessedImage.output_width` / `output_height` **must report the final post-postprocess dimensions**
(read from `output_img.dimensions()` after the stage), because `outline` grows the canvas by +2px.
The `--json` candidate path (`detect_candidates`) does not enter postprocess — no conflict.

### Module layout (mirrors [`resample/`](../../../src/resample))

```
src/postprocess/
  mod.rs         # postprocess(img, config) -> Result<RgbaImage>; fixed-order dispatch
  floodfill.rs   # flood_fill_transparent (stack DFS) + remove_small_floating_components (4-conn CCL)
  outline.rs     # apply_outline (pad +1/side, sharp=4-way / rounded=8-way)
  morphology.rs  # morph_open_close (2×2, alpha-only, replicate border)
  alpha.rs       # binarize_alpha (Fixed strict > / Otsu auto)
```

### Dispatch model — fixed order, independent toggles (Approach A)

The four ops are **independent on/off toggles** (not a mutually-exclusive strategy enum — bg-remove
+ outline is a common combination). When multiple are enabled they run in a fixed order:

```
flood-fill  →  floating-island cleanup  →  morphology  →  alpha-binarize  →  outline
```

Rationale:
- **outline last** — it expands the canvas; no pixel-level op may follow it.
- **alpha before outline** — outline keys off `alpha > 0`; binarizing first gives it clean edges.
- **morph before alpha** — clean speckles/holes first, then threshold.
- **flood-fill first** — remove background before any cleanup.

`postprocess/mod.rs` runs each op gated by its config flag in this order; each op is a pure
`fn(img: RgbaImage, config: &Config) -> Result<RgbaImage>`.

## Config — flat `post_*` fields (default all-off → zero regression)

Added to [`Config`](../../../src/config.rs), consistent with the existing `resample_*` / `quantize_*` /
`detect_*` flat-prefix convention:

| Field | Type | Default | Source |
|---|---|---|---|
| `post_bg_remove` | `bool` | `false` | new |
| `post_bg_tolerance` | `u8` | `64` | PixelRefiner `backgroundTolerance` |
| `post_bg_connectivity` | `enum {Conn4, Conn8}` | `Conn4` | PixelRefiner floodfill `connectivity` |
| `post_bg_scope` | `enum {Outer, All}` | `Outer` | PixelRefiner (Selected deferred) |
| `post_bg_floating_max_pixels` | `usize` | `0` (off) | PixelRefiner `floatingMaxPixels` |
| `post_outline` | `enum {None, Rounded, Sharp}` | `None` | PixelRefiner outline `style` |
| `post_outline_color` | `[u8; 3]` | `[0, 0, 0]` | PixelRefiner outline `color` |
| `post_morph` | `bool` | `false` | unfake `morph_open_close` |
| `post_alpha_threshold` | `enum {None, Fixed(u8), Auto}` | `None` | unfake `>128` + Otsu (new) |

All defaults off → default-config output is byte-identical to Phase 3 → **Oklab anchor
`3a589ee9…e4420` and RGB anchor `802857…9f22` both preserved**.

## Algorithms (clean-room semantics + deviations)

### flood-fill — `floodfill.rs` ← PixelRefiner

- **Stack-based iterative DFS** (non-recursive, R4-friendly). Per-pixel match gate (ALL must hold):
  1. `visited[idx] == false`
  2. RGB per-channel `abs(a[c] - seed[c]) <= tolerance` (per-channel, **not** euclidean)
  3. `alpha != 0` (skip already-transparent)
- On match: set `alpha = 0` (RGB left unchanged).
- Seed color captured from seed pixel **RGB only** (alpha ignored for the target color).
- Seed must itself have `alpha != 0` or the fill is a no-op.
- Neighbor offsets: `Conn4 = [(-1,0),(1,0),(0,-1),(0,1)]`; `Conn8 = Conn4 + diagonals`.
- **`Outer` scope**: seed from **every border pixel** (walk all 4 edges); skip border seeds with
  `alpha == 0`; share one `visited` map across all border seeds (so merges compose).
- **`All` scope**: scan every pixel; for each unvisited pixel, seed a flood-fill (shared `visited`),
  then a second pass forces `alpha = 0` on any remaining pixel whose RGB is within tolerance of any
  flood-fill seed color encountered. Aggressive — can remove interior same-color regions;
  documented warning.
- ⚠️ **Deviation from source**: PixelRefiner hard-codes `All` to 4-way even if the caller configures
  8. We make `All` **respect the configured connectivity** for consistency. Documented.
- `tolerance` default `64`, range 0–255.

### floating-island cleanup — inside `floodfill.rs` ← PixelRefiner `processor.ts:798`

- Runs only when `post_bg_floating_max_pixels > 0`.
- **4-connected** CCL (stack-based; ±1, ±w neighbors, no diagonals).
- `isOpaque(p) := alpha >= 16` (note `>=`, not `>`).
- Per component: count `size`; collect coords but stop storing once `len > max_pixels` (bounds
  memory; `size` keeps counting).
- Track the largest component across the whole image.
- **Removal rule**: erase every component with `size <= max_pixels`, **except the one with the
  largest size** (anti-foot-gun: even a small main object survives).
- Erase sets `alpha = 0`.

### outline — `outline.rs` ← PixelRefiner `outline.ts`

- Pad canvas by **+1px on every side** (output dims grow by +2 each axis). Copy original to offset
  `(1, 1)`.
- For each destination pixel where `alpha == 0` (transparent), inspect neighbors in the source:
  - `Sharp` → 4 neighbors
  - `Rounded` → 8 neighbors
- If **any** neighbor has `alpha > 0`, fill the dst pixel with `post_outline_color` at `alpha = 255`;
  first-match short-circuits the inner loop.
- Only draws into transparent pixels (never overwrites opaque); single-pass 1px thickness.
- `alpha > 0` (strict) — any nonzero alpha counts as opaque for outline growing.
- Default color black `[0,0,0]`; configurable via `--outline-color`.

### morphology — `morphology.rs` ← unfake `morphology.rs`

- **2×2 kernel**, anchor = top-left (offsets `dx,dy ∈ {0,1}` — the source pixel is the top-left of
  the 2×2 block).
- **Border: replicate** — `xx = (x+dx).min(w-1)`, `yy = (y+dy).min(h-1)` (clamp right/bottom edges;
  not mirror, not zero-extension).
- **Order: open then close.** `open = dilate(erode(x))`; `close = erode(dilate(x))`;
  final = `close(open(input))`.
- `erode` = local `min` (init acc=255); `dilate` = local `max` (init acc=0).
- ⚠️ **Deviation from source (user-approved)**: unfake runs **per-channel including alpha**. We run
  **alpha-only** — RGB is untouched. Rationale: by the time we reach postprocess, RGB is already
  clean palette colors (Phase 3 quantize ran upstream); per-channel erode/dilate would synthesize
  colors outside the palette, violating the palette-fidelity property that `majority` resample
  exists to protect. The real pixel-art use cases (fill 2×2 transparent holes, remove 1px
  transparent speckles) are alpha operations. README/NOTICE will note unfake as inspiration.
- Note: unfake's own header concedes it is "not OpenCV bit-exact"; we replicate unfake's kernel
  semantics, not OpenCV's.

### alpha binarize — `alpha.rs` ← unfake `downscale.rs:219` + Otsu (fresh)

- `Fixed(t)`: `alpha > t` → 255 else 0 (**strict `>`**; `t=128` maps 128→0). Matches unfake's
  `median_alpha > 128`.
- `Auto` (Otsu): build a 256-bin histogram of the alpha channel, find the threshold maximizing
  between-class variance (classic Otsu), binarize. **No source for this** — both inspiration
  codebases lack Otsu; this is the fresh "超越原版" implementation PLAN calls for.
- **Otsu degenerate fallback**: if the computed threshold is `0` or `255` (single-peak histogram,
  e.g. fully-opaque or fully-transparent image), fall back to `Fixed(128)` to avoid degenerate
  binarization.
- Default `None` (off).

## CLI / WASM surface (Option B)

### CLI — independent flags (added to [`args.rs`](../../../src/cli/args.rs); `--help` synced)

```
--bg-remove                        Enable background removal
--bg-tolerance <0-255>             Per-channel bg tolerance [default: 64]
--bg-connectivity <4|8>            Flood connectivity [default: 4]
--bg-scope <outer|all>             Removal scope [default: outer]
--bg-floating-threshold <n>        Floating-island cleanup size (0=off) [default: 0]
--outline <rounded|sharp>          Outline style [default: off]
--outline-color <hex>              Outline color [default: 000000]
--morph                            Enable 2×2 open→close (alpha-only)
--alpha-threshold <n|auto>         Alpha binarize (strict >) [default: off]
```

Each op is independently gated by its own flag — `--bg-floating-threshold > 0` enables floating
cleanup **regardless of `--bg-remove`** (e.g. clearing dust speckles on an already-transparent
sprite needs no flood-fill first). The `bg-*` flags other than floating are inert unless
`--bg-remove` is on. Dispatch in `postprocess/mod.rs` runs, in fixed order, whichever ops their
flags enable: flood-fill (if `post_bg_remove`) → floating cleanup (if `post_bg_floating_max_pixels
> 0`) → morphology (if `post_morph`) → alpha binarize (if `post_alpha_threshold != None`) →
outline (if `post_outline != None`).

### WASM — single `post_config` JSON param

`process_image` gains one new param:

```rust
post_config: Option<String>,   // JSON, e.g. {"bg_remove":true,"outline":"sharp","alpha_threshold":"auto"}
```

Parsed into the `post_*` config fields; parse failure → `JsValue` error. Rationale (Option B): the
WASM signature already has 9 positional params; adding ~7 more makes omission-from-the-middle
impossible without passing `null` for every skipped position. A single JSON param keeps the WASM
surface flat and pre-builds the Phase 6 `PipelineConfig` JSON bridge. CLI stays idiomatic
(per-flag discoverability); WASM and CLI serve different consumers.

## Determinism, testing, anchors

- **R1 determinism**: all four ops are **RNG-free** (flood/outline/morph/alpha/Otsu are deterministic
  algorithms). Lower determinism risk than any prior phase.
- **Zero regression**: defaults all off → existing anchors preserved:
  - Oklab default anchor `3a589ee9…e4420`
  - RGB compat anchor `802857…9f22` (`--colorspace rgb`)
- **New fixtures** in `tests/fixtures/baseline/`:
  - `transparent-bg.png` — sprite on solid bg (flood-fill `Outer`/`All`)
  - `noisy.png` — opaque sprite with 1px transparent speckles + 2×2 holes (morph + floating cleanup)
  - `outline-test.png` — small opaque shape on transparent canvas (outline dims +2, 4/8-way)
  - reuse existing `aa-edges.png` for alpha/Otsu behavior
- **New `tests/postprocess.rs`** (mirrors `tests/resample.rs`: `sha2` + `std::env::temp_dir` for
  cross-platform):
  - flood-fill removes bg, preserves subject, floating cleanup deletes small components but keeps
    largest
  - outline grows dims by +2, correct 4/8-way neighbors, default color black + custom color
  - morph: fills 2×2 transparent holes, removes 1px speckles, **does not synthesize new RGB**
    (palette-fidelity assertion)
  - alpha: `Fixed(128)` strict `>`; Otsu produces a sane threshold on a bimodal alpha image;
    degenerate fallback to 128 on single-peak
  - determinism: same image+config twice → byte-identical
  - default-config anchor lock (Oklab + RGB unchanged)
  - CLI flag parsing appended to `cli_tests`

## Acceptance

- Background removal preserves the subject and clears isolated noise (floating cleanup).
- Outline is single-pixel, correct 4-way (sharp) / 8-way (rounded), grows canvas +2.
- Morphology fills 2×2 transparent holes and removes 1px speckles **without synthesizing
  palette-foreign colors**.
- Otsu binarize beats hard `128` on a semi-transparent-edge image; falls back to `128` on
  single-peak.
- `cargo build` (native) and `cargo build --target wasm32` both **0 warnings**.
- `cargo test` all green (existing + new `tests/postprocess.rs`).
- Default-config anchors unchanged (Oklab `3a589ee9…e4420`, RGB `802857…9f22`).
- CLI `--help` lists the new flags; WASM `post_config` JSON parses and round-trips.

## Risks

| Risk | Mitigation |
|------|------------|
| outline changes output dims (+2), breaking downstream dim assumptions | `ProcessedImage` reports final post-postprocess dims; `--json` candidate path bypasses postprocess |
| Otsu degenerates on single-peak alpha histogram | fallback to `Fixed(128)` when threshold ∈ {0, 255} |
| flood `All` scope removes interior same-color regions | default `Outer`; `All` documented as aggressive, requires explicit `--bg-scope all` |
| alpha-only morph diverges from unfake behavior | documented as project-identity choice (palette-fidelity); README/NOTICE credits unfake |
| `All` scope respecting configured connectivity ≠ PixelRefiner | documented deviation (consistency fix) |
| WASM `post_config` JSON schema drift before Phase 6 | keep the JSON keys 1:1 with `post_*` Config field names so Phase 6 `PipelineConfig` serde absorbs them directly |

## Open questions

None — all four design forks resolved during brainstorming:
1. flood scope = Outer + All (Selected deferred to Phase 6)
2. CLI/WASM surface = Option B (CLI independent flags + WASM single JSON param)
3. morph channel = alpha-only (palette-fidelity)
4. outline color = default black + `--outline-color`
