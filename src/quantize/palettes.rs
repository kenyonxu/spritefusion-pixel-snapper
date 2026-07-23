//! Preset retro palettes for the `quantize_preset_palette` Config field.
//!
//! When a preset other than `None` is selected, every pixel of the k-means
//! output is snapped to the nearest palette color (squared-Euclidean) before
//! the rest of the pipeline runs.
//!
//! # Sources
//!
//! - **NES** (55 unique colors): PixelRefiner `src/shared/config.ts` `nes`
//!   entry (`RETRO_PALETTES`). PixelRefiner's 64-entry list contains 10
//!   duplicate `#000000` slots representing invalid NES color-generator
//!   positions; we deduplicate to the 55 unique colors. Dedupe is safe because
//!   `apply_palette` is nearest-neighbor — duplicates never changed output.
//! - **GameBoy** (4): PixelRefiner `gb_legacy` (matches the canonical DMG
//!   palette `[15,56,15] / [48,98,48] / [139,172,15] / [155,188,15]`).
//! - **PC-9801** (16): PixelRefiner `pc98`.
//! - **MSX1** (15): PixelRefiner `msx`.
//! - **PICO-8** (16): canonical PICO-8 palette (matches PixelRefiner `pico8`).
//! - **Sweetie16** (16): Lospec `sweetie-16`.
//! - **Endesga32** (32): Lospec `endesga-32` by ENDESGA, created for NYKRA.
//! - **SGB**: no-op. The Super Game Boy has 4 BIOS palettes and no canonical
//!   default; selecting this preset performs no palette snap.
//! - **SNES**: no-op. The SNES has a 15-bit color space (32768 colors) and no
//!   fixed system palette; selecting this preset performs no palette snap.

use crate::quantize::PresetPalette;

/// Returns the static RGB palette for `p`, or `None` for `None` / no-op
/// presets (currently `Sgb` and `Snes`).
pub fn palette(p: PresetPalette) -> Option<&'static [[u8; 3]]> {
    use crate::quantize::PresetPalette::*;
    match p {
        // `Sgb` and `Snes` are no-op variants: keep them in the enum for
        // stability, but return None so `quantize()` skips the snap. See the
        // module-level docs for rationale. (Glob-imported `PresetPalette::*`
        // shadows `Option::None`, so the return value is qualified.)
        None | Sgb | Snes => Option::None,
        Pico8 => Some(&PICO8),
        Sweetie16 => Some(&SWEETIE16),
        GameBoy => Some(&GAMEBOY),
        Nes => Some(&NES),
        Pc9801 => Some(&PC9801),
        Msx1 => Some(&MSX1),
        Endesga32 => Some(&ENDESGA32),
    }
}

static PICO8: [[u8; 3]; 16] = [
    [0, 0, 0],
    [29, 43, 83],
    [126, 37, 83],
    [0, 135, 81],
    [171, 82, 54],
    [95, 87, 79],
    [194, 195, 199],
    [255, 241, 232],
    [255, 0, 77],
    [255, 163, 0],
    [255, 236, 39],
    [0, 228, 54],
    [41, 173, 255],
    [131, 118, 156],
    [255, 119, 168],
    [255, 204, 170],
];

static SWEETIE16: [[u8; 3]; 16] = [
    [26, 28, 44],
    [93, 39, 93],
    [177, 62, 83],
    [239, 125, 87],
    [255, 205, 117],
    [167, 240, 112],
    [56, 183, 100],
    [37, 113, 121],
    [41, 54, 111],
    [59, 93, 201],
    [65, 166, 246],
    [115, 239, 247],
    [244, 244, 244],
    [148, 176, 194],
    [86, 108, 134],
    [51, 60, 87],
];

static GAMEBOY: [[u8; 3]; 4] = [
    [15, 56, 15],
    [48, 98, 48],
    [139, 172, 15],
    [155, 188, 15],
];

// NES — PixelRefiner `nes`, deduped from 64 entries (10× #000000 → 1) to 55
// unique colors. Order preserved from source.
static NES: [[u8; 3]; 55] = [
    [124, 124, 124],
    [0, 0, 252],
    [0, 0, 188],
    [68, 40, 188],
    [148, 0, 132],
    [168, 0, 32],
    [168, 16, 0],
    [136, 20, 0],
    [80, 48, 0],
    [0, 120, 0],
    [0, 104, 0],
    [0, 88, 0],
    [0, 64, 88],
    [0, 0, 0],
    [188, 188, 188],
    [0, 120, 248],
    [0, 88, 248],
    [104, 68, 252],
    [216, 0, 204],
    [228, 0, 88],
    [248, 56, 0],
    [228, 92, 16],
    [172, 124, 0],
    [0, 184, 0],
    [0, 168, 0],
    [0, 168, 68],
    [0, 136, 136],
    [248, 248, 248],
    [60, 188, 252],
    [104, 136, 252],
    [152, 120, 252],
    [248, 120, 252],
    [248, 88, 152],
    [248, 120, 88],
    [252, 160, 68],
    [248, 184, 0],
    [184, 248, 24],
    [88, 216, 84],
    [88, 248, 152],
    [0, 232, 216],
    [120, 120, 120],
    [252, 252, 252],
    [164, 228, 252],
    [184, 184, 248],
    [216, 184, 248],
    [248, 184, 248],
    [248, 164, 192],
    [240, 208, 176],
    [252, 224, 168],
    [248, 216, 120],
    [216, 248, 120],
    [184, 248, 184],
    [184, 248, 216],
    [0, 252, 252],
    [248, 216, 248],
];

// PC-9801 — PixelRefiner `pc98`.
static PC9801: [[u8; 3]; 16] = [
    [0, 0, 0],
    [0, 0, 248],
    [248, 0, 0],
    [248, 0, 248],
    [0, 248, 0],
    [0, 248, 248],
    [248, 248, 0],
    [248, 248, 248],
    [136, 136, 136],
    [0, 0, 136],
    [136, 0, 0],
    [136, 0, 136],
    [0, 136, 0],
    [0, 136, 136],
    [136, 136, 0],
    [192, 192, 192],
];

// MSX1 — PixelRefiner `msx` (15 colors, TMS9918A fixed palette).
static MSX1: [[u8; 3]; 15] = [
    [0, 0, 0],
    [62, 184, 73],
    [116, 208, 125],
    [89, 85, 224],
    [128, 118, 241],
    [185, 94, 81],
    [101, 219, 239],
    [219, 101, 89],
    [255, 137, 125],
    [204, 195, 94],
    [222, 208, 135],
    [58, 162, 65],
    [183, 102, 181],
    [204, 204, 204],
    [255, 255, 255],
];

// Endesga32 — Lospec `endesga-32` by ENDESGA, originally created for NYKRA.
static ENDESGA32: [[u8; 3]; 32] = [
    [190, 74, 47],
    [215, 118, 67],
    [234, 212, 170],
    [228, 166, 114],
    [184, 111, 80],
    [115, 62, 57],
    [62, 39, 49],
    [162, 38, 51],
    [228, 59, 68],
    [247, 118, 34],
    [254, 174, 52],
    [254, 231, 97],
    [99, 199, 77],
    [62, 137, 72],
    [38, 92, 66],
    [25, 60, 62],
    [18, 78, 137],
    [0, 153, 219],
    [44, 232, 245],
    [255, 255, 255],
    [192, 203, 220],
    [139, 155, 180],
    [90, 105, 136],
    [58, 68, 102],
    [38, 43, 68],
    [24, 20, 37],
    [255, 0, 68],
    [104, 56, 108],
    [181, 80, 136],
    [246, 117, 122],
    [232, 183, 150],
    [194, 133, 105],
];
