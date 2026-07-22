//! Palette parsing, nearest-color mapping, and palette application.

use crate::error::{PixelSnapperError, Result};
use image::{Rgba, RgbaImage};
use std::collections::HashMap;

pub const MAX_PALETTE_COLORS: usize = 256;

pub fn parse_palette_hex(value: &str) -> Result<Vec<[u8; 3]>> {
    if value.trim().is_empty() {
        return Err(PixelSnapperError::InvalidInput(
            "Palette must contain at least one color".to_string(),
        ));
    }

    let mut seen = std::collections::HashSet::new();
    let mut palette = Vec::new();
    for part in value.split(',') {
        let hex = part.trim().trim_start_matches('#');
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(PixelSnapperError::InvalidInput(format!(
                "invalid palette color '{}', expected a 6-digit hex code",
                part.trim()
            )));
        }
        let color = [
            u8::from_str_radix(&hex[0..2], 16).unwrap(),
            u8::from_str_radix(&hex[2..4], 16).unwrap(),
            u8::from_str_radix(&hex[4..6], 16).unwrap(),
        ];
        if seen.insert(color) {
            palette.push(color);
        }
    }

    if palette.len() > MAX_PALETTE_COLORS {
        return Err(PixelSnapperError::InvalidInput(format!(
            "Palette must contain at most {} distinct colors",
            MAX_PALETTE_COLORS
        )));
    }
    Ok(palette)
}

pub fn nearest_palette_color(rgb: [u8; 3], palette: &[[u8; 3]]) -> [u8; 3] {
    let mut best_color = palette[0];
    let mut best_distance = u32::MAX;
    for color in palette {
        let dr = rgb[0] as i32 - color[0] as i32;
        let dg = rgb[1] as i32 - color[1] as i32;
        let db = rgb[2] as i32 - color[2] as i32;
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best_distance = distance;
            best_color = *color;
        }
    }
    best_color
}

pub fn apply_palette(img: &RgbaImage, palette: &[[u8; 3]]) -> Result<RgbaImage> {
    if palette.is_empty() {
        return Err(PixelSnapperError::InvalidInput(
            "Palette must contain at least one RGB color".to_string(),
        ));
    }

    let mut cache: HashMap<[u8; 3], [u8; 3]> = HashMap::new();
    let mut recolored_img = RgbaImage::new(img.width(), img.height());

    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] == 0 {
            recolored_img.put_pixel(x, y, *pixel);
            continue;
        }

        let key = [pixel[0], pixel[1], pixel[2]];
        let color = *cache
            .entry(key)
            .or_insert_with(|| nearest_palette_color(key, palette));
        recolored_img.put_pixel(x, y, Rgba([color[0], color[1], color[2], pixel[3]]));
    }

    Ok(recolored_img)
}
