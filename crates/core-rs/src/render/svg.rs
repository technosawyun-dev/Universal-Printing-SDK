//! PrintDocument -> SVG, for thumbnails / non-print preview. Mirrors
//! `packages/core-ts/src/render/svg.ts`. Not pixel-accurate to any physical
//! printer -- a fixed px-per-mm scale is enough for a thumbnail.

use crate::render::barcode::generate_barcode_bits;
use crate::render::qr::qr_modules;
use crate::types::{resolve_margin_mm, resolve_page_width_mm, Align, BarcodeNode, PrintDocument, PrintNode, QrEcLevel, RowNode};
use crate::CoreError;

const PX_PER_MM: f64 = 4.0;
const LINE_HEIGHT: f64 = 18.0;
const FONT_SIZE: f64 = 13.0;

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn anchor_for(align: Option<Align>) -> (fn(f64) -> f64, &'static str) {
    match align.unwrap_or_default() {
        Align::Center => (|w| w / 2.0, "middle"),
        Align::Right => (|w| w, "end"),
        Align::Left => (|_| 0.0, "start"),
    }
}

struct Block {
    height: f64,
    svg: String,
}

fn text_block(value: &str, content_width: f64, align: Option<Align>, bold: bool, underline: bool, size_h: f64) -> Block {
    let (x_fn, anchor) = anchor_for(align);
    let font_size = FONT_SIZE * size_h;
    let height = LINE_HEIGHT * size_h;
    let mut attrs = format!(
        "x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-size=\"{font_size}\"",
        x_fn(content_width),
        height * 0.75
    );
    if bold {
        attrs.push_str(" font-weight=\"bold\"");
    }
    if underline {
        attrs.push_str(" text-decoration=\"underline\"");
    }
    Block { height, svg: format!("<text {attrs}>{}</text>", escape_xml(value)) }
}

fn row_block(node: &RowNode, content_width: f64) -> Block {
    let total_units: f64 = node.columns.iter().map(|c| c.width).sum();
    let mut cursor = 0.0;
    let mut parts = String::new();
    for col in &node.columns {
        let w = (col.width / total_units) * content_width;
        let (x_fn, anchor) = anchor_for(col.align);
        parts.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-size=\"{FONT_SIZE}\"{}>{}</text>",
            cursor + x_fn(w),
            LINE_HEIGHT * 0.75,
            if node.bold.unwrap_or(false) { " font-weight=\"bold\"" } else { "" },
            escape_xml(&col.text)
        ));
        cursor += w;
    }
    Block { height: LINE_HEIGHT, svg: parts }
}

fn qr_block(data: &str, ec: QrEcLevel, content_width: f64) -> Result<Block, CoreError> {
    let qr = qr_modules(data, ec)?;
    let px = content_width.min(LINE_HEIGHT * 6.0);
    let module_size = px / qr.size as f64;
    let mut rects = String::new();
    for row in 0..qr.size {
        for col in 0..qr.size {
            if qr.is_dark(row, col) {
                rects.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{module_size}\" height=\"{module_size}\"/>",
                    col as f64 * module_size,
                    row as f64 * module_size
                ));
            }
        }
    }
    let offset_x = (content_width - px) / 2.0;
    Ok(Block { height: px, svg: format!("<g transform=\"translate({offset_x},0)\" fill=\"#000\">{rects}</g>") })
}

fn placeholder_barcode_block(symbology: &str, data: &str, content_width: f64) -> Block {
    let height = LINE_HEIGHT * 2.0;
    Block {
        height,
        svg: format!(
            "<rect x=\"0\" y=\"0\" width=\"{content_width}\" height=\"{height}\" fill=\"none\" stroke=\"#000\"/>\
<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{FONT_SIZE}\" font-family=\"monospace\">[{}: {}]</text>",
            content_width / 2.0,
            height / 2.0 + 4.0,
            symbology.to_uppercase(),
            escape_xml(data)
        ),
    }
}

fn barcode_block(node: &BarcodeNode, content_width: f64) -> Block {
    let Some(bits) = generate_barcode_bits(node.symbology, &node.data) else {
        // data invalid for this symbology -- degrade gracefully rather than throw
        // (see docs/DOCUMENT_FORMAT.md's renderer contract).
        return placeholder_barcode_block(node.symbology.name(), &node.data, content_width);
    };
    let bar_height = node.height_px.unwrap_or(60.0).min(LINE_HEIGHT * 4.0);
    let module_width = content_width / bits.len() as f64;
    let mut rects = String::new();
    for (i, b) in bits.bytes().enumerate() {
        if b == b'1' {
            rects.push_str(&format!(
                "<rect x=\"{}\" y=\"0\" width=\"{module_width}\" height=\"{bar_height}\"/>",
                i as f64 * module_width
            ));
        }
    }
    let show_text = node.show_text != Some(false);
    let text_svg = if show_text {
        format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{FONT_SIZE}\" font-family=\"monospace\">{}</text>",
            content_width / 2.0,
            bar_height + FONT_SIZE,
            escape_xml(&node.data)
        )
    } else {
        String::new()
    };
    Block {
        height: bar_height + if show_text { FONT_SIZE + 4.0 } else { 0.0 },
        svg: format!("<g fill=\"#000\">{rects}</g>{text_svg}"),
    }
}

fn block_for(node: &PrintNode, content_width: f64) -> Result<Option<Block>, CoreError> {
    Ok(match node {
        PrintNode::Text(n) => Some(text_block(
            &n.value,
            content_width,
            n.align,
            n.bold.unwrap_or(false),
            n.underline.unwrap_or(false),
            n.size.map(|s| s.h as f64).unwrap_or(1.0),
        )),
        PrintNode::Row(n) => Some(row_block(n, content_width)),
        PrintNode::Rule(_) => Some(Block {
            height: LINE_HEIGHT * 0.5,
            svg: format!(
                "<line x1=\"0\" y1=\"{h}\" x2=\"{content_width}\" y2=\"{h}\" stroke=\"#000\"/>",
                h = LINE_HEIGHT * 0.25
            ),
        }),
        PrintNode::Feed(n) => Some(Block { height: LINE_HEIGHT * n.lines.unwrap_or(1) as f64, svg: String::new() }),
        PrintNode::Image(n) => {
            let height = 80.0;
            Some(Block {
                height,
                svg: format!(
                    "<image href=\"data:image/png;base64,{}\" width=\"{content_width}\" height=\"{height}\" preserveAspectRatio=\"xMidYMid meet\"/>",
                    n.data
                ),
            })
        }
        PrintNode::Barcode(n) => Some(barcode_block(n, content_width)),
        PrintNode::QrCode(n) => Some(qr_block(&n.data, n.ec.unwrap_or(QrEcLevel::M), content_width)?),
        PrintNode::Cut(_) | PrintNode::Drawer(_) | PrintNode::Raw(_) => None, // hardware-only actions; nothing to preview
    })
}

pub fn to_svg(doc: &PrintDocument) -> Result<String, CoreError> {
    let width_mm = resolve_page_width_mm(&doc.paper)?;
    let margin_mm = resolve_margin_mm(&doc.paper);
    let width_px = width_mm * PX_PER_MM;
    let margin_px = margin_mm * PX_PER_MM;
    let content_width = width_px - margin_px * 2.0;

    let mut cursor_y = margin_px;
    let mut groups = String::new();
    for node in &doc.content {
        let Some(block) = block_for(node, content_width)? else { continue };
        if !block.svg.is_empty() {
            groups.push_str(&format!("<g transform=\"translate({margin_px},{cursor_y})\">{}</g>", block.svg));
        }
        cursor_y += block.height;
    }
    let height_px = cursor_y + margin_px;

    Ok(format!(
        "<svg width=\"{width_px}\" height=\"{height_px}\" viewBox=\"0 0 {width_px} {height_px}\" \
xmlns=\"http://www.w3.org/2000/svg\" font-family=\"Arial, sans-serif\">\
<rect width=\"{width_px}\" height=\"{height_px}\" fill=\"#fff\"/>{groups}</svg>"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BarcodeNode, BarcodeSymbology, CutNode, DrawerNode, Paper, PaperKind, QrCodeNode, RawNode, TextNode};

    fn doc(content: Vec<PrintNode>, kind: PaperKind) -> PrintDocument {
        PrintDocument { version: "1.0".to_string(), paper: Paper { kind, width_mm: None, margin_mm: None, chars_per_line: None }, meta: None, content }
    }

    fn text_node(value: &str) -> PrintNode {
        PrintNode::Text(TextNode { value: value.to_string(), align: None, bold: None, underline: None, invert: None, size: None, font: None })
    }

    #[test]
    fn produces_a_valid_svg_root_sized_from_paper_width() {
        let svg = to_svg(&doc(vec![text_node("hi")], PaperKind::Thermal80)).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("width=\"320\"")); // 80mm * 4 px/mm
    }

    #[test]
    fn stacks_nodes_vertically_growing_canvas_height() {
        let height_of = |svg: &str| -> u32 {
            let start = svg.find("height=\"").unwrap() + "height=\"".len();
            let end = svg[start..].find('"').unwrap() + start;
            svg[start..end].parse().unwrap()
        };
        let one = to_svg(&doc(vec![text_node("a")], PaperKind::Thermal80)).unwrap();
        let three = to_svg(&doc(vec![text_node("a"), text_node("b"), text_node("c")], PaperKind::Thermal80)).unwrap();
        assert!(height_of(&three) > height_of(&one));
    }

    #[test]
    fn renders_a_real_barcode_as_bars_plus_hri_text() {
        let svg = to_svg(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Code39, data: "ABC123".into(), height_px: None, show_text: None })], PaperKind::Thermal80)).unwrap();
        assert!(svg.contains("<rect"));
        assert!(svg.contains("ABC123"));
    }

    #[test]
    fn degrades_to_a_labeled_placeholder_box_when_data_is_invalid() {
        let svg = to_svg(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Code39, data: "lower@case".into(), height_px: None, show_text: None })], PaperKind::Thermal80)).unwrap();
        assert!(svg.contains("[CODE39: lower@case]"));
    }

    #[test]
    fn renders_qrcode_as_filled_rects() {
        let svg = to_svg(&doc(vec![PrintNode::QrCode(QrCodeNode { data: "https://example.com".into(), size: None, ec: None })], PaperKind::Thermal80)).unwrap();
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn skips_cut_drawer_raw_nodes_without_adding_height() {
        let empty = to_svg(&doc(vec![], PaperKind::Thermal80)).unwrap();
        let with_no_ops = to_svg(&doc(
            vec![PrintNode::Cut(CutNode { mode: None }), PrintNode::Drawer(DrawerNode { pin: None }), PrintNode::Raw(RawNode { escpos_base64: "AA==".into() })],
            PaperKind::Thermal80,
        ))
        .unwrap();
        assert_eq!(empty, with_no_ops);
    }
}
