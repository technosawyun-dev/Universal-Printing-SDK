//! PNG -> 1bpp dithered raster, for the ESC/POS `GS v 0` raster image command.
//! Mirrors `packages/core-ts/src/render/imaging.ts` -- keep both in step.
//! See that file's header comment for why gamma+Floyd-Steinberg dithering was
//! chosen over escpos-php's flat brightness threshold.

use crate::CoreError;
use base64::Engine;

pub const THERMAL_58_DOTS: u32 = 384;
pub const THERMAL_80_DOTS: u32 = 576;
const DEFAULT_GAMMA: f64 = 1.8;

pub struct RasterImage {
    pub width_dots: u32,
    pub height_dots: u32,
    pub bytes_per_row: u32,
    /// 1 bit per pixel, MSB-first, row-major; bit=1 means "print a dot" (black).
    pub bits: Vec<u8>,
}

pub fn base64_to_bytes(b64: &str) -> Result<Vec<u8>, CoreError> {
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| CoreError::Render(format!("invalid base64: {e}")))
}

fn to_grayscale(data: &[u8], width: u32, height: u32, channels: u32, bit_depth: u32) -> Vec<f64> {
    let scale = if bit_depth == 16 { 255.0 / 65535.0 } else { 1.0 };
    let sample_bytes = if bit_depth == 16 { 2 } else { 1 };
    let px_stride = channels as usize * sample_bytes;
    let sample = |pixel: usize, ch: usize| -> f64 {
        let offset = pixel * px_stride + ch * sample_bytes;
        let raw = if sample_bytes == 2 {
            u16::from_be_bytes([data[offset], data[offset + 1]]) as f64
        } else {
            data[offset] as f64
        };
        raw * scale
    };
    let count = (width * height) as usize;
    let mut gray = vec![0.0; count];
    for i in 0..count {
        gray[i] = if channels <= 2 {
            sample(i, 0)
        } else {
            0.299 * sample(i, 0) + 0.587 * sample(i, 1) + 0.114 * sample(i, 2)
        };
    }
    gray
}

/// Nearest-neighbor resize to `target_width`, preserving aspect ratio.
fn resize(gray: &[f64], src_width: u32, src_height: u32, target_width: u32) -> (Vec<f64>, u32, u32) {
    if src_width == target_width {
        return (gray.to_vec(), src_width, src_height);
    }
    let target_height = ((src_height as u64 * target_width as u64) as f64 / src_width as f64)
        .round()
        .max(1.0) as u32;
    let mut out = vec![0.0; (target_width * target_height) as usize];
    for y in 0..target_height {
        let sy = ((y as u64 * src_height as u64) / target_height as u64).min((src_height - 1) as u64) as u32;
        for x in 0..target_width {
            let sx = ((x as u64 * src_width as u64) / target_width as u64).min((src_width - 1) as u64) as u32;
            out[(y * target_width + x) as usize] = gray[(sy * src_width + sx) as usize];
        }
    }
    (out, target_width, target_height)
}

fn apply_gamma(gray: &[f64], gamma: f64) -> Vec<f64> {
    gray.iter().map(|&v| 255.0 * (v / 255.0).powf(1.0 / gamma)).collect()
}

/// In-place Floyd-Steinberg error-diffusion dither; returns 1bpp packed bits (bit=1 -> black).
fn dither_to_bits(gray: &[f64], width: u32, height: u32) -> Vec<u8> {
    let mut buf = gray.to_vec();
    let bytes_per_row = width.div_ceil(8);
    let mut bits = vec![0u8; (bytes_per_row * height) as usize];
    let (w, h) = (width as i64, height as i64);
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let old = buf[idx];
            let black = old < 128.0;
            let new_val = if black { 0.0 } else { 255.0 };
            let err = old - new_val;
            if black {
                bits[(y as u32 * bytes_per_row + (x as u32 >> 3)) as usize] |= 0x80 >> (x & 7);
            }
            if x + 1 < w {
                buf[idx + 1] += err * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    buf[idx + w as usize - 1] += err * 3.0 / 16.0;
                }
                buf[idx + w as usize] += err * 5.0 / 16.0;
                if x + 1 < w {
                    buf[idx + w as usize + 1] += err * 1.0 / 16.0;
                }
            }
        }
    }
    bits
}

/// Decodes a base64 PNG and dithers it to a 1bpp raster no wider than `target_width_dots`.
pub fn png_to_raster(base64_png: &str, target_width_dots: u32, gamma: f64) -> Result<RasterImage, CoreError> {
    let bytes = base64_to_bytes(base64_png)?;
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes.as_slice()));
    let mut reader = decoder
        .read_info()
        .map_err(|e| CoreError::Render(format!("invalid PNG: {e}")))?;
    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| CoreError::Render("PNG image is too large to decode".to_string()))?;
    let mut buf = vec![0u8; buffer_size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| CoreError::Render(format!("invalid PNG: {e}")))?;
    let channels = info.color_type.samples() as u32;
    let bit_depth = match info.bit_depth {
        png::BitDepth::Sixteen => 16,
        _ => 8,
    };
    let gray = to_grayscale(&buf[..info.buffer_size()], info.width, info.height, channels, bit_depth);

    let clamped_width = target_width_dots.min(info.width);
    let (resized, w, h) = resize(&gray, info.width, info.height, clamped_width);
    let gamma_corrected = apply_gamma(&resized, gamma);
    let bits = dither_to_bits(&gamma_corrected, w, h);
    Ok(RasterImage {
        width_dots: w,
        height_dots: h,
        bytes_per_row: w.div_ceil(8),
        bits,
    })
}

pub fn png_to_raster_default_gamma(base64_png: &str, target_width_dots: u32) -> Result<RasterImage, CoreError> {
    png_to_raster(base64_png, target_width_dots, DEFAULT_GAMMA)
}

/// Splits a raster into row-bands so `GS v 0` chunks stay small enough for the printer's receive buffer.
pub fn band_raster(image: &RasterImage, band_rows: u32) -> Vec<(u32, &[u8])> {
    let mut bands = Vec::new();
    let mut y = 0;
    while y < image.height_dots {
        let rows = band_rows.min(image.height_dots - y);
        let start = (y * image.bytes_per_row) as usize;
        let end = start + (rows * image.bytes_per_row) as usize;
        bands.push((rows, &image.bits[start..end]));
        y += rows;
    }
    bands
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn encode_gray_png(width: u32, height: u32, pixels: &[u8]) -> String {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
        }
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn dithers_pure_black_white_pixels_to_exact_1bpp_bits_with_no_drift() {
        // Gamma-correcting pure 0/255 values is a no-op, so alternating b/w
        // pixels accumulate zero diffusion error -- we can assert an exact byte.
        let png = encode_gray_png(4, 1, &[0, 255, 0, 255]);
        let raster = png_to_raster_default_gamma(&png, 4).unwrap();
        assert_eq!(raster.width_dots, 4);
        assert_eq!(raster.height_dots, 1);
        assert_eq!(raster.bytes_per_row, 1);
        assert_eq!(raster.bits[0], 0xa0); // 1010_0000: bit=1 for the two black pixels
    }

    #[test]
    fn keeps_thermal_dot_width_constants_aligned_with_docs() {
        assert_eq!(THERMAL_58_DOTS, 384);
        assert_eq!(THERMAL_80_DOTS, 576);
    }

    #[test]
    fn downscales_an_oversized_image_to_the_target_dot_width() {
        let pixels = vec![255u8; 800 * 100];
        let png = encode_gray_png(800, 100, &pixels);
        let raster = png_to_raster_default_gamma(&png, 384).unwrap();
        assert_eq!(raster.width_dots, 384);
        assert_eq!(raster.height_dots, 48); // 100 * 384/800, rounded
    }

    #[test]
    fn splits_a_tall_raster_into_row_bands_that_reconstruct_the_original_bits() {
        let pixels = vec![0u8; 8 * 100]; // all black
        let png = encode_gray_png(8, 100, &pixels);
        let raster = png_to_raster_default_gamma(&png, 8).unwrap();
        let bands = band_raster(&raster, 48);
        assert_eq!(bands.len(), 3); // 48 + 48 + 4 rows
        assert_eq!(bands.iter().map(|(rows, _)| rows).sum::<u32>(), 100);

        let mut reconstructed = Vec::new();
        for (_, bits) in &bands {
            reconstructed.extend_from_slice(bits);
        }
        assert_eq!(reconstructed, raster.bits);
    }
}
