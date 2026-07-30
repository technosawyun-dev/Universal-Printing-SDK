//! PrintDocument -> ESC/POS bytes. Mirrors `packages/core-ts/src/render/escpos.ts`
//! byte-for-byte -- see that file's header for the escpos-php cross-check notes.

use crate::render::imaging::{band_raster, base64_to_bytes, png_to_raster_default_gamma, RasterImage, THERMAL_58_DOTS, THERMAL_80_DOTS};
use crate::render::layout::layout_row;
use crate::render::text_shaping::{bitmap_to_raster, needs_shaping, shape_text_to_bitmap};
use crate::types::{
    Align, BarcodeNode, BarcodeSymbology, CutNode, DrawerNode, FeedNode, ImageNode, Paper, PaperKind, PrintDocument,
    PrintNode, QrCodeNode, QrEcLevel, RawNode, RowNode, RuleNode, TextNode,
};
use crate::{types::resolve_chars_per_line, CoreError};

/// Base line height (in thermal dots) a shaped-text bitmap targets at
/// `TextSize { w: 1, h: 1 }`, before the per-font compensation below.
const SHAPED_TEXT_BASE_HEIGHT_DOTS: f32 = 32.0;

fn shaped_text_height_dots(size_h: u8) -> f32 {
    SHAPED_TEXT_BASE_HEIGHT_DOTS * size_h.max(1) as f32 * crate::render::text_shaping::VISUAL_SIZE_COMPENSATION
}

fn max_dots_for_paper(paper: &Paper) -> u32 {
    if paper.kind == PaperKind::Thermal58 {
        THERMAL_58_DOTS
    } else {
        THERMAL_80_DOTS
    }
}

/// Emits a shaped-text bitmap via the same `GS v 0` raster path `write_image`
/// uses. Returns `false` if shaping produced nothing (e.g. whitespace-only),
/// so the caller can fall back to the plain-text path.
fn write_shaped_text(w: &mut ByteWriter, text: &str, size_h: u8, align: Align, bold: bool, max_dots: u32) -> bool {
    let Some(bitmap) = shape_text_to_bitmap(text, shaped_text_height_dots(size_h), bold) else {
        return false;
    };
    let raster: RasterImage = bitmap_to_raster(&bitmap, max_dots);
    w.push(&[ESC, 0x61, align_code(align)]);
    for (rows, bits) in band_raster(&raster, 48) {
        w.push(&[
            GS,
            0x76,
            0x30,
            0x00,
            (raster.bytes_per_row & 0xff) as u8,
            ((raster.bytes_per_row >> 8) & 0xff) as u8,
            (rows & 0xff) as u8,
            ((rows >> 8) & 0xff) as u8,
        ]);
        w.push(bits);
    }
    w.push(&[ESC, 0x61, 0]);
    true
}

const ESC: u8 = 0x1b;
const GS: u8 = 0x1d;

#[derive(Default)]
struct ByteWriter(Vec<u8>);

impl ByteWriter {
    fn push(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.extend_from_slice(bytes);
        self
    }
}

fn clamp(n: i64, min: i64, max: i64) -> u8 {
    n.clamp(min, max) as u8
}

fn align_code(align: Align) -> u8 {
    match align {
        Align::Center => 1,
        Align::Right => 2,
        Align::Left => 0,
    }
}

/// Best-effort passthrough for the common Latin-1/CP437 overlap; anything
/// above U+00FF becomes '?'. Full codepage switching is deferred for v1.
fn text_to_bytes(value: &str) -> Vec<u8> {
    value
        .chars()
        .map(|c| if (c as u32) <= 0xff { c as u8 } else { b'?' })
        .collect()
}

fn write_text(w: &mut ByteWriter, node: &TextNode, paper: &Paper) {
    // Underline/invert/font-B aren't applied on this path -- the shaper
    // draws whichever of the two bundled weights (regular/bold) matches
    // `node.bold`, but has no notion of underline, invert, or a distinct
    // "Font B" face. Acceptable for v1 (still correctly-shaped, legible,
    // correctly-weighted Myanmar); revisit if those turn out to matter.
    if needs_shaping(&node.value) {
        let size_h = node.size.map(|s| s.h).unwrap_or(1);
        let align = node.align.unwrap_or_default();
        let bold = node.bold.unwrap_or(false);
        if write_shaped_text(w, &node.value, size_h, align, bold, max_dots_for_paper(paper)) {
            return;
        }
        // shape_text_to_bitmap returned nothing (e.g. whitespace-only) -- fall through.
    }

    let size_w = clamp(node.size.map(|s| s.w as i64).unwrap_or(1) - 1, 0, 7);
    let size_h = clamp(node.size.map(|s| s.h as i64).unwrap_or(1) - 1, 0, 7);
    w.push(&[ESC, 0x61, align_code(node.align.unwrap_or_default())]);
    w.push(&[ESC, 0x45, node.bold.unwrap_or(false) as u8]);
    w.push(&[ESC, 0x2d, node.underline.unwrap_or(false) as u8]);
    w.push(&[GS, 0x42, node.invert.unwrap_or(false) as u8]);
    w.push(&[GS, 0x21, (size_w << 4) | size_h]);
    w.push(&[ESC, 0x4d, (node.font == Some(crate::types::Font::B)) as u8]);
    w.push(&text_to_bytes(&node.value));
    w.push(&[0x0a]);
    // Reset to defaults so this node's formatting doesn't leak into the next one.
    w.push(&[ESC, 0x61, 0, ESC, 0x45, 0, ESC, 0x2d, 0, GS, 0x42, 0, GS, 0x21, 0, ESC, 0x4d, 0]);
}

fn write_row(w: &mut ByteWriter, node: &RowNode, chars_per_line: usize, paper: &Paper) {
    if node.columns.iter().any(|c| needs_shaping(&c.text)) {
        write_row_shaped(w, node, paper);
        return;
    }
    w.push(&[ESC, 0x45, node.bold.unwrap_or(false) as u8]);
    w.push(&text_to_bytes(&layout_row(&node.columns, chars_per_line)));
    w.push(&[0x0a]);
    w.push(&[ESC, 0x45, 0]);
}

/// Row layout for the case where at least one column needs shaped-text
/// rendering. Once any column needs it, *all* columns in the row render as
/// bitmaps (even plain-ASCII ones) so proportional column positions stay in
/// real pixels throughout -- mixing a pixel-positioned bitmap column with a
/// monospace-character-grid column in the same row wouldn't line up.
fn write_row_shaped(w: &mut ByteWriter, node: &RowNode, paper: &Paper) {
    let max_dots = max_dots_for_paper(paper);
    let row_height = shaped_text_height_dots(1).ceil() as u32;
    let total_units: f64 = node.columns.iter().map(|c| c.width).sum();
    if total_units <= 0.0 {
        return;
    }
    let bold = node.bold.unwrap_or(false);

    let mut canvas = vec![0u8; (max_dots * row_height) as usize];
    let mut x_cursor = 0u32;
    for col in &node.columns {
        let col_width_dots = ((col.width / total_units) * max_dots as f64).round() as u32;
        if let Some(bitmap) = shape_text_to_bitmap(&col.text, row_height as f32, bold) {
            let bw = bitmap.width.min(col_width_dots.max(1));
            let x_offset = match col.align.unwrap_or_default() {
                Align::Right => col_width_dots.saturating_sub(bw),
                Align::Center => col_width_dots.saturating_sub(bw) / 2,
                Align::Left => 0,
            };
            let y_offset = row_height.saturating_sub(bitmap.height) / 2; // vertically center within the shared row height
            for y in 0..bitmap.height.min(row_height) {
                for x in 0..bw {
                    let src = bitmap.coverage[(y * bitmap.width + x) as usize];
                    let dst_x = x_cursor + x_offset + x;
                    let dst_y = y_offset + y;
                    if dst_x < max_dots && dst_y < row_height {
                        let idx = (dst_y * max_dots + dst_x) as usize;
                        canvas[idx] = canvas[idx].max(src);
                    }
                }
            }
        }
        x_cursor += col_width_dots;
    }

    let raster = crate::render::text_shaping::coverage_to_raster(max_dots, row_height, &canvas, max_dots);
    for (rows, bits) in band_raster(&raster, 48) {
        w.push(&[
            GS,
            0x76,
            0x30,
            0x00,
            (raster.bytes_per_row & 0xff) as u8,
            ((raster.bytes_per_row >> 8) & 0xff) as u8,
            (rows & 0xff) as u8,
            ((rows >> 8) & 0xff) as u8,
        ]);
        w.push(bits);
    }
}

fn write_rule(w: &mut ByteWriter, node: &RuleNode, chars_per_line: usize) {
    let ch = node.char.clone().unwrap_or_else(|| "-".to_string());
    w.push(&text_to_bytes(&ch.repeat(chars_per_line)));
    w.push(&[0x0a]);
}

fn write_feed(w: &mut ByteWriter, node: &FeedNode) {
    let lines = node.lines.unwrap_or(1);
    if lines <= 1 {
        w.push(&[0x0a]);
    } else {
        w.push(&[ESC, 0x64, clamp(lines as i64, 1, 255)]);
    }
}

fn write_image(w: &mut ByteWriter, node: &ImageNode, paper: &Paper) -> Result<(), CoreError> {
    let max_dots = if paper.kind == PaperKind::Thermal58 { THERMAL_58_DOTS } else { THERMAL_80_DOTS };
    let target_width = node.width_px.map(|v| (v as u32).min(max_dots)).unwrap_or(max_dots);
    let raster = png_to_raster_default_gamma(&node.data, target_width)?;
    w.push(&[ESC, 0x61, align_code(node.align.unwrap_or(Align::Center))]);
    for (rows, bits) in band_raster(&raster, 48) {
        w.push(&[
            GS,
            0x76,
            0x30,
            0x00,
            (raster.bytes_per_row & 0xff) as u8,
            ((raster.bytes_per_row >> 8) & 0xff) as u8,
            (rows & 0xff) as u8,
            ((rows >> 8) & 0xff) as u8,
        ]);
        w.push(bits);
    }
    w.push(&[ESC, 0x61, 0]);
    Ok(())
}

fn barcode_type(symbology: BarcodeSymbology) -> u8 {
    match symbology {
        BarcodeSymbology::Upca => 65,
        BarcodeSymbology::Ean13 => 67,
        BarcodeSymbology::Code39 => 69,
        BarcodeSymbology::Code128 => 73,
    }
}

fn write_barcode(w: &mut ByteWriter, node: &BarcodeNode) {
    let height_px = clamp(node.height_px.unwrap_or(60.0) as i64, 1, 255);
    w.push(&[GS, 0x68, height_px]); // GS h n -- barcode height
    w.push(&[GS, 0x77, 3]); // GS w n -- module width
    w.push(&[GS, 0x48, if node.show_text == Some(false) { 0 } else { 2 }]); // GS H n -- HRI position
    let data = if node.symbology == BarcodeSymbology::Code128 {
        format!("{{B{}", node.data)
    } else {
        node.data.clone()
    };
    let bytes = text_to_bytes(&data);
    w.push(&[GS, 0x6b, barcode_type(node.symbology), bytes.len() as u8]);
    w.push(&bytes);
}

fn qr_ec_byte(ec: QrEcLevel) -> u8 {
    match ec {
        QrEcLevel::L => 48,
        QrEcLevel::M => 49,
        QrEcLevel::Q => 50,
        QrEcLevel::H => 51,
    }
}

fn write_qrcode(w: &mut ByteWriter, node: &QrCodeNode) {
    let size = clamp(node.size.unwrap_or(6) as i64, 1, 16);
    let ec_byte = qr_ec_byte(node.ec.unwrap_or(QrEcLevel::M));
    let data = node.data.as_bytes();
    w.push(&[GS, 0x28, 0x6b, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00]); // select model 2
    w.push(&[GS, 0x28, 0x6b, 0x03, 0x00, 0x31, 0x43, size]); // module size
    w.push(&[GS, 0x28, 0x6b, 0x03, 0x00, 0x31, 0x45, ec_byte]); // error correction level
    let store_len = data.len() + 3;
    w.push(&[GS, 0x28, 0x6b, (store_len & 0xff) as u8, ((store_len >> 8) & 0xff) as u8, 0x31, 0x50, 0x30]); // store data
    w.push(data);
    w.push(&[GS, 0x28, 0x6b, 0x03, 0x00, 0x31, 0x51, 0x30]); // print
}

fn write_cut(w: &mut ByteWriter, node: &CutNode) {
    w.push(&[GS, 0x56, if node.mode == Some(crate::types::CutMode::Full) { 0 } else { 1 }]);
}

fn write_drawer(w: &mut ByteWriter, node: &DrawerNode) {
    w.push(&[ESC, 0x70, if node.pin == Some(5) { 1 } else { 0 }, 25, 250]);
}

fn write_raw(w: &mut ByteWriter, node: &RawNode) -> Result<(), CoreError> {
    w.push(&base64_to_bytes(&node.escpos_base64)?);
    Ok(())
}

pub fn to_escpos(doc: &PrintDocument) -> Result<Vec<u8>, CoreError> {
    let mut w = ByteWriter::default();
    let chars_per_line = resolve_chars_per_line(&doc.paper) as usize;
    w.push(&[ESC, 0x40]); // initialize
    for node in &doc.content {
        match node {
            PrintNode::Text(n) => write_text(&mut w, n, &doc.paper),
            PrintNode::Row(n) => write_row(&mut w, n, chars_per_line, &doc.paper),
            PrintNode::Rule(n) => write_rule(&mut w, n, chars_per_line),
            PrintNode::Feed(n) => write_feed(&mut w, n),
            PrintNode::Image(n) => write_image(&mut w, n, &doc.paper)?,
            PrintNode::Barcode(n) => write_barcode(&mut w, n),
            PrintNode::QrCode(n) => write_qrcode(&mut w, n),
            PrintNode::Cut(n) => write_cut(&mut w, n),
            PrintNode::Drawer(n) => write_drawer(&mut w, n),
            PrintNode::Raw(n) => write_raw(&mut w, n)?,
        }
    }
    Ok(w.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Paper, PaperKind};

    fn doc(content: Vec<PrintNode>, kind: PaperKind) -> PrintDocument {
        PrintDocument { version: "1.0".to_string(), paper: Paper { kind, width_mm: None, margin_mm: None, chars_per_line: None }, meta: None, content }
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    #[test]
    fn starts_every_document_with_the_initialize_command() {
        let bytes = to_escpos(&doc(vec![], PaperKind::Thermal80)).unwrap();
        assert_eq!(&bytes[0..2], &[0x1b, 0x40]);
    }

    #[test]
    fn encodes_a_plain_text_line_ending_in_lf() {
        let bytes = to_escpos(&doc(vec![PrintNode::Text(TextNode { value: "hi".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None })], PaperKind::Thermal80)).unwrap();
        let hi = find(&bytes, b"hi").unwrap();
        assert_eq!(bytes[hi + 2], 0x0a);
    }

    #[test]
    fn emits_gs_v_for_cut_with_0_full_and_1_partial() {
        let full = to_escpos(&doc(vec![PrintNode::Cut(CutNode { mode: Some(crate::types::CutMode::Full) })], PaperKind::Thermal80)).unwrap();
        let partial = to_escpos(&doc(vec![PrintNode::Cut(CutNode { mode: None })], PaperKind::Thermal80)).unwrap();
        assert_eq!(&full[full.len() - 3..], &[0x1d, 0x56, 0x00]);
        assert_eq!(&partial[partial.len() - 3..], &[0x1d, 0x56, 0x01]);
    }

    #[test]
    fn emits_esc_p_for_the_drawer_pulse_with_pin_encoding() {
        let pin2 = to_escpos(&doc(vec![PrintNode::Drawer(DrawerNode { pin: None })], PaperKind::Thermal80)).unwrap();
        let pin5 = to_escpos(&doc(vec![PrintNode::Drawer(DrawerNode { pin: Some(5) })], PaperKind::Thermal80)).unwrap();
        assert_eq!(&pin2[pin2.len() - 5..], &[0x1b, 0x70, 0, 25, 250]);
        assert_eq!(&pin5[pin5.len() - 5..], &[0x1b, 0x70, 1, 25, 250]);
    }

    #[test]
    fn emits_esc_d_n_for_multi_line_feed_and_bare_lf_for_single() {
        let single = to_escpos(&doc(vec![PrintNode::Feed(FeedNode { lines: None })], PaperKind::Thermal80)).unwrap();
        let multi = to_escpos(&doc(vec![PrintNode::Feed(FeedNode { lines: Some(5) })], PaperKind::Thermal80)).unwrap();
        assert_eq!(&single[single.len() - 1..], &[0x0a]);
        assert_eq!(&multi[multi.len() - 3..], &[0x1b, 0x64, 5]);
    }

    #[test]
    fn builds_a_code128_barcode_with_subset_prefix_and_function_b_framing() {
        let bytes = to_escpos(&doc(
            vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Code128, data: "12345".into(), height_px: None, show_text: None })],
            PaperKind::Thermal80,
        ))
        .unwrap();
        let gsk = (0..bytes.len() - 3).find(|&i| bytes[i] == GS && bytes[i + 1] == 0x6b && bytes[i + 2] == 73).unwrap();
        assert_eq!(bytes[gsk + 3], 7); // len("{B12345")
        assert_eq!(&bytes[gsk + 4..gsk + 4 + 7], b"{B12345");
    }

    #[test]
    fn builds_qrcode_command_with_correct_ec_level_bytes() {
        for (ec, expected) in [
            (QrEcLevel::L, 48),
            (QrEcLevel::M, 49),
            (QrEcLevel::Q, 50),
            (QrEcLevel::H, 51),
        ] {
            let bytes = to_escpos(&doc(vec![PrintNode::QrCode(QrCodeNode { data: "hi".into(), size: None, ec: Some(ec) })], PaperKind::Thermal80)).unwrap();
            let idx = (0..bytes.len() - 7)
                .find(|&i| bytes[i] == GS && bytes[i + 1] == 0x28 && bytes[i + 2] == 0x6b && bytes[i + 5] == 0x31 && bytes[i + 6] == 0x45)
                .unwrap();
            assert_eq!(bytes[idx + 7], expected);
        }
    }

    #[test]
    fn lays_out_a_row_into_the_papers_char_width_before_encoding() {
        let bytes = to_escpos(&doc(
            vec![PrintNode::Row(RowNode {
                columns: vec![
                    crate::types::RowColumn { text: "A".into(), width: 1.0, align: Some(Align::Left) },
                    crate::types::RowColumn { text: "B".into(), width: 1.0, align: Some(Align::Right) },
                ],
                bold: None,
            })],
            PaperKind::Thermal58,
        ))
        .unwrap();
        let a_idx = find(&bytes, b"A").unwrap();
        let lf_idx = (a_idx..bytes.len()).find(|&i| bytes[i] == 0x0a).unwrap();
        assert_eq!(lf_idx - a_idx, 32); // thermal-58 -> 32 chars per line
    }
}
