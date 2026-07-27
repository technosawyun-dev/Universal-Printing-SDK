//! PrintDocument -> PDF bytes. Only exists in core-rs (not core-ts) -- used
//! by crates/print-agent's silent A4/A3/Letter path (docs/PROTOCOL.md's
//! `/print-silent`). Rendering PDF in a browser JS context is unnecessary
//! since the agent handles that transport itself (docs/ARCHITECTURE.md).

use crate::render::barcode::generate_barcode_bits;
use crate::render::qr::qr_modules;
use crate::types::{resolve_margin_mm, resolve_page_height_mm, resolve_page_width_mm, Align, PrintDocument, PrintNode, QrEcLevel};
use crate::CoreError;
use printpdf::*;

const MM_TO_PT: f32 = 2.834_645_7;
const LINE_HEIGHT_PT: f32 = 14.0;
const FONT_SIZE_PT: f32 = 11.0;
const BLACK: Color = Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None });

fn text_x(align: Option<Align>, content_width_pt: f32, text_width_pt: f32) -> f32 {
    match align.unwrap_or_default() {
        Align::Center => (content_width_pt - text_width_pt) / 2.0,
        Align::Right => content_width_pt - text_width_pt,
        Align::Left => 0.0,
    }
}

/// Rough Helvetica-ish average glyph width -- we don't load real font metrics,
/// this is good enough to approximate alignment on a receipt/invoice.
fn approx_text_width_pt(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.5
}

fn show_text(ops: &mut Vec<Op>, text: &str, x_pt: f32, y_pt: f32, size_pt: f32, bold: bool) {
    ops.push(Op::StartTextSection);
    ops.push(Op::SetTextCursor { pos: Point { x: Pt(x_pt), y: Pt(y_pt) } });
    ops.push(Op::SetFont {
        font: PdfFontHandle::Builtin(if bold { BuiltinFont::HelveticaBold } else { BuiltinFont::Helvetica }),
        size: Pt(size_pt),
    });
    ops.push(Op::SetLineHeight { lh: Pt(size_pt) });
    ops.push(Op::SetFillColor { col: BLACK });
    ops.push(Op::ShowText { items: vec![TextItem::Text(text.to_string())] });
    ops.push(Op::EndTextSection);
}

fn filled_rect(ops: &mut Vec<Op>, x0: f32, y0: f32, x1: f32, y1: f32) {
    ops.push(Op::SetFillColor { col: BLACK });
    ops.push(Op::DrawPolygon {
        polygon: Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    LinePoint { p: Point { x: Pt(x0), y: Pt(y0) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x1), y: Pt(y0) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x1), y: Pt(y1) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x0), y: Pt(y1) }, bezier: false },
                ],
            }],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        },
    });
}

fn outlined_rect(ops: &mut Vec<Op>, x0: f32, y0: f32, x1: f32, y1: f32) {
    ops.push(Op::SetOutlineColor { col: BLACK });
    ops.push(Op::SetOutlineThickness { pt: Pt(1.0) });
    ops.push(Op::DrawPolygon {
        polygon: Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    LinePoint { p: Point { x: Pt(x0), y: Pt(y0) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x1), y: Pt(y0) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x1), y: Pt(y1) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x0), y: Pt(y1) }, bezier: false },
                ],
            }],
            mode: PaintMode::Stroke,
            winding_order: WindingOrder::NonZero,
        },
    });
}

pub fn to_pdf(doc: &PrintDocument) -> Result<Vec<u8>, CoreError> {
    let width_mm = resolve_page_width_mm(&doc.paper)? as f32;
    let height_mm = resolve_page_height_mm(&doc.paper).unwrap_or(297.0) as f32;
    let margin_mm = resolve_margin_mm(&doc.paper) as f32;
    let width_pt = width_mm * MM_TO_PT;
    let height_pt = height_mm * MM_TO_PT;
    let margin_pt = margin_mm * MM_TO_PT;
    let content_width_pt = width_pt - margin_pt * 2.0;

    let title = doc.meta.as_ref().and_then(|m| m.title.clone()).unwrap_or_else(|| "Print".to_string());
    let mut pdf_doc = PdfDocument::new(&title);
    let mut ops: Vec<Op> = Vec::new();
    let mut cursor_pt = height_pt - margin_pt; // PDF y-axis is bottom-up; start at the top margin.

    for node in &doc.content {
        match node {
            PrintNode::Text(n) => {
                let size = FONT_SIZE_PT * n.size.map(|s| s.h as f32).unwrap_or(1.0);
                let x = text_x(n.align, content_width_pt, approx_text_width_pt(&n.value, size));
                show_text(&mut ops, &n.value, margin_pt + x, cursor_pt - size, size, n.bold.unwrap_or(false));
                cursor_pt -= size * 1.4;
            }
            PrintNode::Row(n) => {
                let total_units: f32 = n.columns.iter().map(|c| c.width as f32).sum();
                let mut x_cursor = 0.0;
                let bold = n.bold.unwrap_or(false);
                for col in &n.columns {
                    let w = (col.width as f32 / total_units) * content_width_pt;
                    let text_w = approx_text_width_pt(&col.text, FONT_SIZE_PT);
                    let x = x_cursor + text_x(col.align, w, text_w);
                    show_text(&mut ops, &col.text, margin_pt + x, cursor_pt - FONT_SIZE_PT, FONT_SIZE_PT, bold);
                    x_cursor += w;
                }
                cursor_pt -= FONT_SIZE_PT * 1.4;
            }
            PrintNode::Rule(_) => {
                let y = cursor_pt - 4.0;
                ops.push(Op::SetOutlineColor { col: BLACK });
                ops.push(Op::SetOutlineThickness { pt: Pt(0.75) });
                ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![
                            LinePoint { p: Point { x: Pt(margin_pt), y: Pt(y) }, bezier: false },
                            LinePoint { p: Point { x: Pt(margin_pt + content_width_pt), y: Pt(y) }, bezier: false },
                        ],
                        is_closed: false,
                    },
                });
                cursor_pt -= LINE_HEIGHT_PT * 0.5;
            }
            PrintNode::Feed(n) => cursor_pt -= LINE_HEIGHT_PT * n.lines.unwrap_or(1) as f32,
            PrintNode::Image(n) => {
                let png_bytes = crate::render::imaging::base64_to_bytes(&n.data)?;
                let mut warnings = Vec::new();
                let image = RawImage::decode_from_bytes(&png_bytes, &mut warnings)
                    .map_err(|e| CoreError::Render(format!("invalid image: {e}")))?;
                let image_id = pdf_doc.add_image(&image);
                let image_width_pt = (n.width_px.map(|w| w as f32).unwrap_or(image.width as f32)).min(content_width_pt);
                let image_height_pt = image_width_pt * (image.height as f32 / image.width as f32);
                // Synthesize a dpi so printpdf's auto pixel-size scaling lands exactly on
                // our computed width/height, keeping scale_x/scale_y at the default (1.0).
                let dpi = image.width as f32 * 72.0 / image_width_pt;
                let x = margin_pt + text_x(n.align, content_width_pt, image_width_pt);
                let y = cursor_pt - image_height_pt;
                ops.push(Op::UseXobject {
                    id: image_id,
                    transform: XObjectTransform { translate_x: Some(Pt(x)), translate_y: Some(Pt(y)), dpi: Some(dpi), ..Default::default() },
                });
                cursor_pt -= image_height_pt;
            }
            PrintNode::Barcode(n) => match generate_barcode_bits(n.symbology, &n.data) {
                None => {
                    // data invalid for this symbology -- degrade gracefully rather than
                    // throw (see docs/DOCUMENT_FORMAT.md's renderer contract).
                    let box_height = LINE_HEIGHT_PT * 2.0;
                    outlined_rect(&mut ops, margin_pt, cursor_pt - box_height, margin_pt + content_width_pt, cursor_pt);
                    let label = format!("[{}: {}]", n.symbology.name().to_uppercase(), n.data);
                    let x = text_x(None, content_width_pt, approx_text_width_pt(&label, FONT_SIZE_PT));
                    show_text(&mut ops, &label, margin_pt + x, cursor_pt - box_height / 2.0, FONT_SIZE_PT, false);
                    cursor_pt -= box_height;
                }
                Some(bits) => {
                    let bar_height = n.height_px.map(|h| h as f32).unwrap_or(60.0).min(LINE_HEIGHT_PT * 4.0);
                    let module_width = content_width_pt / bits.len() as f32;
                    let top_y = cursor_pt;
                    for (i, b) in bits.bytes().enumerate() {
                        if b == b'1' {
                            let x0 = margin_pt + i as f32 * module_width;
                            filled_rect(&mut ops, x0, top_y - bar_height, x0 + module_width, top_y);
                        }
                    }
                    cursor_pt -= bar_height;
                    if n.show_text != Some(false) {
                        let x = text_x(None, content_width_pt, approx_text_width_pt(&n.data, FONT_SIZE_PT));
                        show_text(&mut ops, &n.data, margin_pt + x, cursor_pt - FONT_SIZE_PT, FONT_SIZE_PT, false);
                        cursor_pt -= FONT_SIZE_PT * 1.4;
                    }
                }
            },
            PrintNode::QrCode(n) => {
                let qr = qr_modules(&n.data, n.ec.unwrap_or(QrEcLevel::M))?;
                let px = content_width_pt.min(LINE_HEIGHT_PT * 6.0);
                let module = px / qr.size as f32;
                let top_y = cursor_pt;
                for row in 0..qr.size {
                    for col in 0..qr.size {
                        if qr.is_dark(row, col) {
                            let x0 = margin_pt + col as f32 * module;
                            let y0 = top_y - (row as f32 + 1.0) * module;
                            filled_rect(&mut ops, x0, y0, x0 + module, y0 + module);
                        }
                    }
                }
                cursor_pt -= px;
            }
            PrintNode::Cut(_) | PrintNode::Drawer(_) | PrintNode::Raw(_) => {} // hardware-only actions; no-op for PDF
        }
    }

    let page = PdfPage::new(Mm(width_mm), Mm(height_mm), ops);
    let bytes = pdf_doc.with_pages(vec![page]).save(&PdfSaveOptions::default(), &mut Vec::new());
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BarcodeNode, BarcodeSymbology, CutNode, DrawerNode, Paper, PaperKind, QrCodeNode, RawNode, RowColumn, RowNode, RuleNode, TextNode};

    fn doc(content: Vec<PrintNode>) -> PrintDocument {
        PrintDocument { version: "1.0".to_string(), paper: Paper { kind: PaperKind::A4, width_mm: None, margin_mm: None, chars_per_line: None }, meta: None, content }
    }

    #[test]
    fn produces_a_well_formed_pdf_for_a_representative_document() {
        let bytes = to_pdf(&doc(vec![
            PrintNode::Text(TextNode { value: "Invoice #1".into(), align: Some(Align::Center), bold: Some(true), underline: None, invert: None, size: None, font: None }),
            PrintNode::Row(RowNode { columns: vec![RowColumn { text: "Item".into(), width: 1.0, align: Some(Align::Left) }, RowColumn { text: "1.00".into(), width: 1.0, align: Some(Align::Right) }], bold: None }),
            PrintNode::Rule(RuleNode { char: None }),
            PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Code128, data: "12345".into(), height_px: None, show_text: None }),
            PrintNode::QrCode(QrCodeNode { data: "https://example.com".into(), size: None, ec: None }),
            PrintNode::Cut(CutNode { mode: None }),
            PrintNode::Drawer(DrawerNode { pin: None }),
            PrintNode::Raw(RawNode { escpos_base64: "AA==".into() }),
        ]))
        .unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 200);
    }

    #[test]
    fn renders_a_real_barcode_and_an_invalid_one_degrades_to_placeholder() {
        let valid = to_pdf(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Ean13, data: "400638133393".into(), height_px: None, show_text: None })])).unwrap();
        assert!(valid.starts_with(b"%PDF"));

        let invalid = to_pdf(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Ean13, data: "not-digits".into(), height_px: None, show_text: None })])).unwrap();
        assert!(invalid.starts_with(b"%PDF"));
    }

    #[test]
    fn embeds_an_image_node() {
        let mut png_bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, 2, 2);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0, 255, 255, 0]).unwrap();
        }
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        let bytes = to_pdf(&doc(vec![crate::types::PrintNode::Image(crate::types::ImageNode { data: b64, width_px: Some(40.0), align: None })])).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }
}
