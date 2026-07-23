# Phase 3 Design: Quantize Enhancement + Rename to pixel-game-kit

**Date:** 2026-07-22
**Status:** Approved (brainstormed)
**Related:** [PLAN.md](../../../PLAN.md) Phase 3 · [USER_STORIES.md](../../../USER_STORIES.md) U4.1–U4.5 · [CONFIG.md](../../CONFIG.md) `quantize` schema

## Background

Phase 2 added multi-strategy resample. The quantize stage still uses the original RGB k-means, no dithering, no preset palettes — leaving perceptual quality (Oklab), retro texture (dithering), and platform-matching (palettes) on the table. Phase 3 brings these in. Also: the project has grown far beyond "pixel snapper" (grid alignment) into a full AI-pixel-art-to-game-asset pipeline (multi-detector, multi-resample, Oklab, dithering, palettes, post-processing, vectorize, web product). The name no longer fits, and the `spritefusion-` prefix (upstream company) misleads. Phase 3 renames to `pixel-game-kit` and bumps to 2.0.

## Goals

- **Oklab k-means** as the default colorspace (perceptually uniform — smoother gradients than RGB). RGB retained as `--colorspace rgb`.
- **Dithering**: Floyd-Steinberg + Bayer 2/4/8 + Ordered.
- **Preset palettes**: NES / GameBoy / SGB / SNES / PC-9801 / MSX1 / PICO-8 / Sweetie16 / Endesga32.
- **qvote resample backfill** (deferred from Phase 2; now possible since Oklab k-means lands here).
- **Rename to `pixel-game-kit`** + **bump 2.0**.

## Non-Goals

- `content_adaptive` (Öztireli EM) resample — still deferred (heavy, feature-gate, separate).
- Web UI (Phase 6).
- Post-processing (Phase 4).

## Decisions

1. **Oklab is the default colorspace.** No external users exist (only the owner), so the breaking cost is zero. Oklab gives perceptually better quantization (smoother gradients). RGB stays as `--colorspace rgb` for fallback/comparison. The sha256 anchor updates to the Oklab baseline; the RGB path is locked by a separate regression test.
2. **Dithering = full set** (FS + Bayer 2/4/8 + Ordered). Implementation is cheap (error diffusion vs threshold matrices); complete enum now.
3. **Palettes = full set** (9 presets, data borrowed from PixelRefiner `src/shared/`, MIT). Pure data, low cost.
4. **qvote backfilled here.** Phase 2 deferred qvote because it needs Oklab k-means to replace imagequant. Phase 3 ships Oklab → qvote becomes possible → add `resample/qvote.rs` (Oklab-quantize-then-vote).
5. **Rename `spritefusion-pixel-snapper` → `pixel-game-kit`** + **bump 1.x → 2.0**. Name now fits the full pipeline; drops the misleading upstream prefix; 2.0 marks the breaking default change. Done as Phase 3 plan Task 1 (prerequisite), before any functional Task.
6. **Palette precedence unchanged**: `custom_palette` > `preset_palette` > k-means.

## Architecture

### Rename (Task 1, prerequisite)

- `Cargo.toml`: `name = "pixel-game-kit"`, `[[bin]] name = "pixel-game-kit"`.
- Repo rename on GitHub: `spritefusion-pixel-snapper` → `pixel-game-kit` (origin remote URL updates).
- WASM pkg output: `pixel_game_kit.js` / `pixel_game_kit_bg.wasm` (wasm-pack derives from crate name). The JS export `process_image` is unchanged.
- Doc references updated: `README.md`, `CLAUDE.md`, `PLAN.md`, `USER_STORIES.md`, `docs/CONFIG.md`, `schema/*`, `docs/superpowers/*`.
- `upstream` remote kept (still pulls Hugo-Dz updates).
- main.rs shim calls `pixel_game_kit::run_cli()` (crate name change → import path change).

### Module layout

`src/quantize.rs` → `src/quantize/` directory:
- `mod.rs` — `Colorspace` / `DitherMethod` / `PresetPalette` enums + `quantize()` dispatch.
- `oklab.rs` — sRGB↔Oklab conversion (← PixelRefiner `colorUtils.ts`).
- `kmeans.rs` — existing k-means moved in; distance switches on `Colorspace` (Oklab default).
- `dither.rs` — FS / Bayer / Ordered (← PixelRefiner `quantizer.ts`).
- `palettes.rs` — 9 preset palette data tables + `preset_palette(name) -> &'static [[u8;3]]`.

`src/resample/qvote.rs` (new) — qvote strategy using Oklab k-means.

### Data flow

`process_image_common` after resample → `quantize::quantize(&img, &config)` → branch on colorspace (Oklab default) → optional dither → optional palette snap. The palette snap reuses existing `palette::apply_palette` (preset or custom).

## Data Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colorspace { Rgb, Oklab }  // default Oklab

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMethod { None, FloydSteinberg, Bayer2, Bayer4, Bayer8, Ordered }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetPalette { None, Nes, GameBoy, Sgb, Snes, Pc9801, Msx1, Pico8, Sweetie16, Endesga32 }
```

Config additions (snake_case, aligns with `quantize` schema):
```rust
pub(crate) quantize_colorspace: Colorspace,            // default Oklab
pub(crate) quantize_dither: DitherMethod,              // default None
pub(crate) quantize_dither_strength: f64,              // default 1.0
pub(crate) quantize_preset_palette: PresetPalette,     // default None
```
(`custom_palette` already exists from Phase 0.)

## Algorithm details

### Oklab (← PixelRefiner colorUtils.ts)
sRGB → linear → LMS → cbrt → Oklab. k-means distance = squared-Euclidean in Oklab (replaces RGB squared-Euclidean). Seeded `ChaCha8Rng` preserved (R1 determinism). The RGB path is the same k-means with RGB distance.

### Dithering (← PixelRefiner quantizer.ts)
- **Floyd-Steinberg**: classic 7/3/5/1 error diffusion, error × strength, RGB-domain diffusion, quantize in colorspace. Skip alpha=0.
- **Bayer 2/4/8**: precomputed threshold matrices normalized to [0,1]; bias = (threshold−0.5)·strength·255 added to RGB before quantize.
- **Ordered**: 4×4 ordered matrix, same bias mechanism.

### Preset palettes (← PixelRefiner src/shared/)
9 static `&'static [[u8;3]]` tables. Applied via existing `palette::apply_palette` (nearest-color squared-Euclidean).

### qvote (← unfake, imagequant-free)
For each resample cell: Oklab-quantize the cell's pixels to a small k, then vote per quantized color. Uses Phase 3 `kmeans.rs` (Oklab) instead of imagequant → GPL-free.

## Interface

### CLI
- `--colorspace <rgb|oklab>` (default `oklab`)
- `--dither <none|fs|bayer2|bayer4|bayer8|ordered>` (default `none`)
- `--dither-strength <0-2>` (default 1.0)
- `--preset <none|nes|gameboy|sgb|snes|pc9801|msx1|pico8|sweetie16|endesga32>` (default `none`)
- Existing `[COLORS]`, `--palette`, `--detect`, `--resample` unchanged.
- Binary renamed: `pixel-game-kit` (was `spritefusion-pixel-snapper`).

### WASM
- `process_image(...)`: add `colorspace` / `dither` / `preset_palette` trailing `Option<String>` params (backward compatible).
- pkg output renamed `pixel_game_kit.js`; `process_image` export unchanged.

### Config (aligns with CONFIG.md `quantize` schema)
Field names above. The schema's `quantize.colorspace` default flips to `oklab`; `dither` and `preset_palette` enums gain these variants.

## Tests

- `tests/quantize.rs` (new):
  - `oklab_default_new_anchor` — ai-sprite default (Oklab) → new sha256, recorded.
  - `rgb_path_matches_old_anchor` — `--colorspace rgb` → `802857...9f22` (RGB compatibility locked).
  - each dither method → fixture + sha256.
  - each preset palette → fixture + assert "every output color is in the preset".
  - determinism: same input+config twice → byte-identical.
- `tests/resample.rs`: add `qvote` to the strategy determinism loop.

## Acceptance

- Oklab default → ai-sprite new anchor (deterministic, recorded).
- `--colorspace rgb` → `802857...9f22` (RGB path preserved).
- Each dither method runs + distinct output.
- Each preset → output colors all within preset.
- qvote runs as a resample strategy.
- Rename complete: `pixel-game-kit` binary, pkg, docs; 2.0 in Cargo.toml.
- `cargo test` green; `cargo build --target wasm32` 0 warnings.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Oklab k-means slower than RGB | colorspace conversion is O(pixels), k-means same iterations; acceptable |
| Rename breaks local muscle memory / scripts | owner-only user; 2.0 documents the change; old binary name gone intentionally |
| Preset palette color values disputed (NES/SNES history) | borrow PixelRefiner's curated data; document source |
| qvote quality without imagequant | non-bit-exact vs unfake but GPL-free; Oklab k-means is a good proxy |
| Dithering breaks determinism? | FS/Bayer/Ordered are RNG-free deterministic processes (R1 holds) |
| 2.0 anchor change confuses regression | record new Oklab anchor explicitly; keep RGB anchor test as compatibility gate |

## Mapping

- **PLAN Phase 3**: covers Oklab/dither/preset-palette + qvote backfill + rename + 2.0.
- **USER_STORIES**: U4.2 (Oklab 🔴), U4.3 (preset palettes 🟡), U4.5 (dithering 🟡), U4.4 (custom palette, existing). U4.1 (k_colors, existing).
- **CONFIG.md**: `quantize` schema updated (Oklab default, dither/preset enums).
- **Phase 2 leftover**: qvote backfilled (was blocked on Oklab).
- **Project rename**: `spritefusion-pixel-snapper` → `pixel-game-kit`, 2.0.
