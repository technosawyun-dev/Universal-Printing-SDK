//! Renders complex-script text (e.g. Myanmar) that the byte-oriented ESC/POS
//! text path and the PDF renderer's builtin-font text path can't handle, into
//! a grayscale coverage bitmap via real text shaping + outline rasterization.
//! Both existing text paths fall back to this instead of emitting garbage:
//! - `render/escpos.rs`'s `text_to_bytes` replaces any codepoint above
//!   U+00FF with '?' -- thermal printer firmware has no Myanmar codepage.
//! - `render/pdf.rs` only draws with the 14 builtin Latin PDF fonts, which
//!   have no Myanmar glyphs at all.
//!
//! Uses `rustybuzz` (a pure-Rust HarfBuzz) for shaping -- required because
//! Myanmar reorders/recombines vowel signs and medials around a base
//! consonant; naively drawing one glyph per codepoint in string order (what
//! both fallback paths did before this module existed) produces the wrong
//! reading order, not just missing glyphs. `ab_glyph` then rasterizes the
//! shaped glyph outlines into a coverage bitmap, which both callers feed
//! through their existing image-embedding pipeline (the same one already
//! used for `image` nodes) rather than needing a second rendering backend.
//!
//! Both weights are bundled at compile time (`include_bytes!`) so this works
//! identically on every target (Windows/Linux/macOS/Android/iOS) without a
//! runtime font dependency or relying on whatever happens to be installed.
//! Noto Sans Myanmar Regular + Bold, SIL Open Font License 1.1 -- see
//! `assets/NotoSansMyanmar-OFL.txt`.
//!
//! Known limitation: this always shapes the *entire* input string against
//! the single bundled Myanmar font, rather than splitting mixed-script runs
//! (e.g. "IMEI: 123456" mixed with Myanmar words) across per-script fonts.
//! Noto Sans Myanmar also covers Basic Latin/digits, so mixed text renders,
//! just in one consistent typeface rather than switching to a Latin font for
//! the Latin portion. Good enough for v1; revisit if that's visually wrong.

use ab_glyph::{Font as AbFont, GlyphId, Point};

static MYANMAR_FONT_REGULAR_BYTES: &[u8] = include_bytes!("../../assets/NotoSansMyanmar-Regular.ttf");
static MYANMAR_FONT_BOLD_BYTES: &[u8] = include_bytes!("../../assets/NotoSansMyanmar-Bold.ttf");

fn myanmar_font_bytes(bold: bool) -> &'static [u8] {
    if bold {
        MYANMAR_FONT_BOLD_BYTES
    } else {
        MYANMAR_FONT_REGULAR_BYTES
    }
}

/// True if `text` contains any character the simple Latin-1 paths (ESC/POS's
/// `text_to_bytes`, the PDF renderer's builtin-font path) can't represent --
/// deliberately the same U+00FF threshold `text_to_bytes` already uses, so
/// both renderers agree on which strings need this path.
pub fn needs_shaping(text: &str) -> bool {
    text.chars().any(|c| (c as u32) > 0xff)
}

/// Noto Sans Myanmar's em-square reserves extra vertical room for Myanmar's
/// stacked vowel signs/medials, so a capital Latin letter's visible ink is
/// only ~33% of the declared em-size (vs. ~70% for a Latin-only font like
/// Arial). Callers that want shaped text to visually match a target pixel
/// height should multiply their target by this before passing it as
/// `em_size_px` to [`shape_text_to_bitmap`]. Verified visually against a
/// known-good browser rendering of the same font/text.
pub const VISUAL_SIZE_COMPENSATION: f32 = 2.2;

/// An 8-bit grayscale coverage bitmap: 0 = background, 255 = fully-covered
/// glyph ink. Row-major, one byte per pixel (no padding).
pub struct TextBitmap {
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<u8>,
}

/// Shapes `text` and rasterizes it at `em_size_px` (the em-square size, in
/// pixels -- the same convention every other renderer in this codebase uses
/// for font size, e.g. `render/pdf.rs`'s `FONT_SIZE_PT`). Returns `None` for
/// empty/whitespace-only input or if the font data is somehow unparseable
/// (bundled font, so that should never happen outside of a corrupted build).
///
/// Note for callers: Noto Sans Myanmar's em-square is deliberately oversized
/// relative to its glyphs, to leave vertical room for Myanmar's stacked
/// vowel signs/medials above and below the baseline -- a capital Latin 'H'
/// measures only ~33% of `em_size_px` tall here, versus ~70% for a
/// Latin-only font like Arial at the same declared size. This is correct,
/// standard font-rendering behavior (every renderer sizes by em-square, not
/// cap-height) and not something this function compensates for -- pick a
/// larger `em_size_px` than you would for a Latin font if the output looks
/// small, rather than expecting this function to renormalize it for you.
pub fn shape_text_to_bitmap(text: &str, em_size_px: f32, bold: bool) -> Option<TextBitmap> {
    if text.trim().is_empty() {
        return None;
    }

    let font_bytes = myanmar_font_bytes(bold);
    let hb_face = rustybuzz::Face::from_slice(font_bytes, 0)?;
    let ab_font = ab_glyph::FontRef::try_from_slice(font_bytes).ok()?;

    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let glyph_buffer = rustybuzz::shape(&hb_face, &[], buffer);

    let units_per_em = hb_face.units_per_em() as f32;
    let scale = em_size_px / units_per_em;

    // Pass 1: walk the shaped glyph run left-to-right, accumulating the pen
    // position from HarfBuzz's advances, and outline each glyph at its final
    // pixel position. HarfBuzz's y-axis points up (font/PDF convention);
    // ab_glyph positions glyphs in a y-down pixel-buffer convention, so
    // y_offset is negated when converting between the two.
    let mut pen_x = 0.0f32;
    let mut pen_y = 0.0f32;
    let mut outlines = Vec::new();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);

    for (info, pos) in glyph_buffer.glyph_infos().iter().zip(glyph_buffer.glyph_positions().iter()) {
        let glyph_x = pen_x + pos.x_offset as f32 * scale;
        let glyph_y = pen_y - pos.y_offset as f32 * scale;
        let glyph = GlyphId(info.glyph_id as u16).with_scale_and_position(em_size_px, point_xy(glyph_x, glyph_y));
        if let Some(outlined) = ab_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            min_x = min_x.min(bounds.min.x);
            min_y = min_y.min(bounds.min.y);
            max_x = max_x.max(bounds.max.x);
            max_y = max_y.max(bounds.max.y);
            outlines.push(outlined);
        }
        pen_x += pos.x_advance as f32 * scale;
        pen_y += pos.y_advance as f32 * scale;
    }

    if outlines.is_empty() {
        return None;
    }

    let width = (max_x - min_x).ceil().max(1.0) as u32;
    let height = (max_y - min_y).ceil().max(1.0) as u32;
    // Cap runaway allocation (e.g. a pathological font/size combination) --
    // matches the spirit of imaging.rs's other defensive bounds.
    if width > 20_000 || height > 20_000 {
        return None;
    }
    let mut coverage = vec![0u8; (width * height) as usize];

    for outlined in &outlines {
        let bounds = outlined.px_bounds();
        let offset_x = (bounds.min.x - min_x).round() as i32;
        let offset_y = (bounds.min.y - min_y).round() as i32;
        outlined.draw(|x, y, c| {
            let (px, py) = (offset_x + x as i32, offset_y + y as i32);
            if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                let idx = (py as u32 * width + px as u32) as usize;
                let value = (c.clamp(0.0, 1.0) * 255.0) as u8;
                coverage[idx] = coverage[idx].max(value);
            }
        });
    }

    Some(TextBitmap { width, height, coverage })
}

fn point_xy(x: f32, y: f32) -> Point {
    ab_glyph::point(x, y)
}

/// Converts a shaped [`TextBitmap`] into the same 1bpp packed [`RasterImage`]
/// shape `render/imaging.rs` already produces for `image` nodes, so both the
/// ESC/POS and PDF renderers can feed shaped text through their existing,
/// already-tested raster-emission code instead of a second bespoke path.
/// Plain thresholding (not Floyd-Steinberg dithering) is deliberate -- text
/// benefits from crisp edges at receipt/small-print resolution, where
/// dithering a stroke only a couple of dots wide makes it look noisy rather
/// than smoother. Also caps the output to `max_width_dots`, cropping any
/// overflow rather than wrapping (see module docs: no line-wrapping yet).
pub fn bitmap_to_raster(bitmap: &TextBitmap, max_width_dots: u32) -> crate::render::imaging::RasterImage {
    coverage_to_raster(bitmap.width, bitmap.height, &bitmap.coverage, max_width_dots)
}

/// Same conversion as [`bitmap_to_raster`], but for a raw coverage buffer
/// (e.g. one several shaped column bitmaps have already been composited
/// into) rather than requiring a single [`TextBitmap`].
pub fn coverage_to_raster(width: u32, height: u32, coverage: &[u8], max_width_dots: u32) -> crate::render::imaging::RasterImage {
    use crate::render::imaging::RasterImage;
    const THRESHOLD: u8 = 128;

    let out_width = width.min(max_width_dots.max(1));
    let bytes_per_row = out_width.div_ceil(8);
    let mut bits = vec![0u8; (bytes_per_row * height) as usize];
    for y in 0..height {
        for x in 0..out_width {
            if coverage[(y * width + x) as usize] >= THRESHOLD {
                bits[(y * bytes_per_row + (x >> 3)) as usize] |= 0x80 >> (x & 7);
            }
        }
    }
    RasterImage { width_dots: out_width, height_dots: height, bytes_per_row, bits }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_shaping_is_false_for_plain_ascii() {
        assert!(!needs_shaping("Invoice #123 - Total: 9.99"));
    }

    #[test]
    fn needs_shaping_is_true_for_myanmar_text() {
        // "Hello" in Myanmar script (mingalarba)
        assert!(needs_shaping("မင်္ဂလာပါ"));
    }

    #[test]
    fn shapes_myanmar_text_into_a_non_empty_bitmap() {
        let bitmap = shape_text_to_bitmap("မင်္ဂလာပါ", 32.0, false).expect("should shape");
        assert!(bitmap.width > 0);
        assert!(bitmap.height > 0);
        assert!(bitmap.coverage.iter().any(|&b| b > 0), "expected at least one covered (ink) pixel");
    }

    #[test]
    fn shapes_mixed_myanmar_and_latin_text() {
        let bitmap = shape_text_to_bitmap("IMEI: မင်္ဂလာပါ 123", 32.0, false).expect("should shape");
        assert!(bitmap.width > 0);
        assert!(bitmap.coverage.iter().any(|&b| b > 0));
    }

    #[test]
    fn returns_none_for_empty_or_whitespace_input() {
        assert!(shape_text_to_bitmap("", 32.0, false).is_none());
        assert!(shape_text_to_bitmap("   ", 32.0, false).is_none());
    }

    #[test]
    fn larger_font_size_produces_a_larger_bitmap() {
        let small = shape_text_to_bitmap("မင်္ဂလာပါ", 16.0, false).unwrap();
        let large = shape_text_to_bitmap("မင်္ဂလာပါ", 64.0, false).unwrap();
        assert!(large.width > small.width);
        assert!(large.height > small.height);
    }

    #[test]
    fn bold_uses_a_visibly_heavier_font_than_regular() {
        let regular = shape_text_to_bitmap("မင်္ဂလာပါ", 48.0, false).unwrap();
        let bold = shape_text_to_bitmap("မင်္ဂလာပါ", 48.0, true).unwrap();
        let ink_coverage = |b: &TextBitmap| b.coverage.iter().filter(|&&c| c >= 128).count();
        assert!(ink_coverage(&bold) > ink_coverage(&regular), "bold glyphs should cover more pixels than regular at the same size");
    }
}
