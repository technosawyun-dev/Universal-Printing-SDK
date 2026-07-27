//! PrintDocument -> self-contained HTML. Mirrors `packages/core-ts/src/render/html.ts`.
//! `row` columns use real proportional (flexbox) widths, not a char grid.

use crate::render::barcode::generate_barcode_bits;
use crate::render::qr::qr_modules;
use crate::types::{is_thermal, resolve_margin_mm, resolve_page_height_mm, resolve_page_width_mm, Align, PrintDocument, PrintNode, QrEcLevel};
use crate::CoreError;

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn align_css(align: Option<Align>) -> &'static str {
    match align.unwrap_or_default() {
        Align::Center => "center",
        Align::Right => "right",
        Align::Left => "left",
    }
}

fn render_qr_svg(data: &str, ec: QrEcLevel, module_px: u32) -> Result<String, CoreError> {
    let qr = qr_modules(data, ec)?;
    let mut rects = String::new();
    for row in 0..qr.size {
        for col in 0..qr.size {
            if qr.is_dark(row, col) {
                rects.push_str(&format!("<rect x=\"{col}\" y=\"{row}\" width=\"1\" height=\"1\"/>"));
            }
        }
    }
    let px = qr.size as u32 * module_px;
    Ok(format!(
        "<svg width=\"{px}\" height=\"{px}\" viewBox=\"0 0 {size} {size}\" shape-rendering=\"crispEdges\">\
<rect width=\"{size}\" height=\"{size}\" fill=\"#fff\"/><g fill=\"#000\">{rects}</g></svg>",
        size = qr.size
    ))
}

fn render_barcode_svg(bits: &str, height_px: u32, module_width: u32) -> String {
    let mut rects = String::new();
    for (i, b) in bits.bytes().enumerate() {
        if b == b'1' {
            rects.push_str(&format!("<rect x=\"{}\" y=\"0\" width=\"{module_width}\" height=\"{height_px}\"/>", i as u32 * module_width));
        }
    }
    let width = bits.len() as u32 * module_width;
    format!("<svg width=\"{width}\" height=\"{height_px}\" viewBox=\"0 0 {width} {height_px}\" shape-rendering=\"crispEdges\"><g fill=\"#000\">{rects}</g></svg>")
}

fn placeholder_barcode(symbology: &str, data: &str) -> String {
    format!(
        "<div style=\"border:1px solid #000;padding:4px 8px;display:inline-block;\
text-align:center;font-family:monospace\">[{}: {}]</div>",
        symbology.to_uppercase(),
        escape_html(data)
    )
}

fn render_node(node: &PrintNode) -> Result<String, CoreError> {
    Ok(match node {
        PrintNode::Text(n) => {
            let mut style = vec![format!("text-align:{}", align_css(n.align))];
            if n.bold.unwrap_or(false) {
                style.push("font-weight:bold".into());
            }
            if n.underline.unwrap_or(false) {
                style.push("text-decoration:underline".into());
            }
            if n.invert.unwrap_or(false) {
                style.push("background:#000;color:#fff".into());
            }
            style.push(format!("font-size:{}%", n.size.map(|s| s.h as u32).unwrap_or(1) * 100));
            if n.font == Some(crate::types::Font::B) {
                style.push("font-family:monospace".into());
            }
            format!("<div style=\"{}\">{}</div>", style.join(";"), escape_html(&n.value))
        }
        PrintNode::Row(n) => {
            let cols: String = n
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "<div style=\"flex:{} 0 0;text-align:{};overflow:hidden;white-space:nowrap\">{}</div>",
                        c.width,
                        align_css(c.align),
                        escape_html(&c.text)
                    )
                })
                .collect();
            format!(
                "<div style=\"display:flex;gap:4px;{}\">{}</div>",
                if n.bold.unwrap_or(false) { "font-weight:bold" } else { "" },
                cols
            )
        }
        PrintNode::Rule(_) => "<hr style=\"border:none;border-top:1px solid #000;margin:4px 0\"/>".to_string(),
        PrintNode::Feed(n) => format!("<div style=\"height:{}em\"></div>", n.lines.unwrap_or(1)),
        PrintNode::Image(n) => {
            let width_attr = n.width_px.map(|w| format!(" width=\"{w}\"")).unwrap_or_default();
            format!(
                "<div style=\"text-align:{}\"><img alt=\"\" src=\"data:image/png;base64,{}\"{}/></div>",
                align_css(n.align),
                n.data,
                width_attr
            )
        }
        PrintNode::Barcode(n) => match generate_barcode_bits(n.symbology, &n.data) {
            None => placeholder_barcode(n.symbology.name(), &n.data), // data invalid for this symbology (see docs/DOCUMENT_FORMAT.md renderer contract)
            Some(bits) => {
                let hri = if n.show_text == Some(false) {
                    String::new()
                } else {
                    format!("<div style=\"font-family:monospace;font-size:11px\">{}</div>", escape_html(&n.data))
                };
                format!(
                    "<div style=\"text-align:center\">{}{hri}</div>",
                    render_barcode_svg(&bits, n.height_px.map(|h| h as u32).unwrap_or(60), 2)
                )
            }
        },
        PrintNode::QrCode(n) => format!(
            "<div style=\"text-align:center\">{}</div>",
            render_qr_svg(&n.data, n.ec.unwrap_or(QrEcLevel::M), 4)?
        ),
        PrintNode::Cut(_) | PrintNode::Drawer(_) | PrintNode::Raw(_) => String::new(), // hardware-only, no-op for HTML/PDF preview
    })
}

pub fn to_html(doc: &PrintDocument) -> Result<String, CoreError> {
    let width_mm = resolve_page_width_mm(&doc.paper)?;
    let margin_mm = resolve_margin_mm(&doc.paper);
    let height_mm = resolve_page_height_mm(&doc.paper);
    let thermal = is_thermal(&doc.paper);
    let title = doc.meta.as_ref().and_then(|m| m.title.clone()).unwrap_or_else(|| "Print".to_string());

    let mut body = String::new();
    for node in &doc.content {
        body.push_str(&render_node(node)?);
        body.push('\n');
    }

    let page_size = match height_mm {
        Some(h) => format!("{width_mm}mm {h}mm"),
        None => format!("{width_mm}mm auto"),
    };
    Ok(format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>\n  \
@page {{ size: {page_size}; margin: {margin_mm}mm; }}\n  body {{\n    margin: 0 auto;\n    width: {width_mm}mm;\n    \
padding: {margin_mm}mm;\n    box-sizing: border-box;\n    font-family: {font_family};\n    font-size: {font_size};\n  }}\n\
</style>\n</head>\n<body>\n{body}</body>\n</html>\n",
        title = escape_html(&title),
        font_family = if thermal { "'Courier New', monospace" } else { "Arial, sans-serif" },
        font_size = if thermal { "12px" } else { "14px" },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BarcodeNode, BarcodeSymbology, CutNode, DocMeta, DrawerNode, Paper, PaperKind, QrCodeNode, RawNode, RowColumn, RowNode, TextNode};

    fn doc(content: Vec<PrintNode>, kind: PaperKind) -> PrintDocument {
        PrintDocument {
            version: "1.0".to_string(),
            paper: Paper { kind, width_mm: None, margin_mm: None, chars_per_line: None },
            meta: Some(DocMeta { title: Some("Receipt #1".to_string()), encoding: None, locale: None }),
            content,
        }
    }

    #[test]
    fn produces_self_contained_html_with_the_doc_title() {
        let html = to_html(&doc(vec![PrintNode::Text(TextNode { value: "Hello".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None })], PaperKind::A4)).unwrap();
        assert!(html.contains("<title>Receipt #1</title>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn escapes_html_significant_characters_in_text_content() {
        let html = to_html(&doc(vec![PrintNode::Text(TextNode { value: "<script>&\"'".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None })], PaperKind::A4)).unwrap();
        assert!(!html.contains("<script>&\"'"));
        assert!(html.contains("&lt;script&gt;&amp;&quot;&#39;"));
    }

    #[test]
    fn lays_out_row_columns_with_proportional_flex_widths() {
        let html = to_html(&doc(
            vec![PrintNode::Row(RowNode {
                columns: vec![
                    RowColumn { text: "Item".into(), width: 3.0, align: Some(Align::Left) },
                    RowColumn { text: "1.00".into(), width: 1.0, align: Some(Align::Right) },
                ],
                bold: None,
            })],
            PaperKind::A4,
        ))
        .unwrap();
        assert!(html.contains("flex:3 0 0"));
        assert!(html.contains("flex:1 0 0"));
    }

    #[test]
    fn renders_a_real_barcode_as_bars_plus_hri_text_by_default() {
        let html = to_html(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Ean13, data: "0123456789012".into(), height_px: None, show_text: None })], PaperKind::A4)).unwrap();
        assert!(html.contains("<svg"));
        assert!(html.contains("<rect"));
        assert!(html.contains("0123456789012"));
    }

    #[test]
    fn omits_hri_text_when_show_text_is_false() {
        let html = to_html(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Ean13, data: "0123456789012".into(), height_px: None, show_text: Some(false) })], PaperKind::A4)).unwrap();
        assert!(!html.contains("0123456789012"));
    }

    #[test]
    fn degrades_to_a_labeled_placeholder_box_when_data_is_invalid() {
        let html = to_html(&doc(vec![PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Ean13, data: "not-digits".into(), height_px: None, show_text: None })], PaperKind::A4)).unwrap();
        assert!(html.contains("[EAN13: not-digits]"));
    }

    #[test]
    fn renders_qrcode_as_inline_svg() {
        let html = to_html(&doc(vec![PrintNode::QrCode(QrCodeNode { data: "https://example.com".into(), size: None, ec: None })], PaperKind::A4)).unwrap();
        assert!(html.contains("<svg"));
        assert!(html.contains("<rect"));
    }

    #[test]
    fn no_ops_cut_drawer_raw_nodes() {
        let html = to_html(&doc(vec![PrintNode::Cut(CutNode { mode: None }), PrintNode::Drawer(DrawerNode { pin: None }), PrintNode::Raw(RawNode { escpos_base64: "AA==".into() })], PaperKind::A4)).unwrap();
        assert!(html.contains("<body>"));
    }

    #[test]
    fn sets_page_size_from_paper_geometry() {
        let html = to_html(&doc(vec![], PaperKind::A4)).unwrap();
        assert!(html.contains("size: 210mm 297mm"));
    }
}
