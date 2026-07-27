//! PrintDocument -> plain text. Mirrors `packages/core-ts/src/render/text.ts`.
//! Node types this renderer can't represent (image/barcode/qrcode) degrade
//! to a placeholder line rather than failing, per docs/DOCUMENT_FORMAT.md's
//! renderer contract.

use crate::render::layout::{layout_row, wrap_and_align};
use crate::types::{resolve_chars_per_line, PrintDocument, PrintNode};

pub fn to_plain_text(doc: &PrintDocument) -> String {
    let chars_per_line = resolve_chars_per_line(&doc.paper) as usize;
    let mut lines: Vec<String> = Vec::new();
    for node in &doc.content {
        match node {
            PrintNode::Text(n) => {
                lines.extend(wrap_and_align(&n.value, chars_per_line, n.align.unwrap_or_default()));
            }
            PrintNode::Row(n) => lines.push(layout_row(&n.columns, chars_per_line)),
            PrintNode::Rule(n) => {
                let ch = n.char.clone().unwrap_or_else(|| "-".to_string());
                lines.push(ch.repeat(chars_per_line));
            }
            PrintNode::Feed(n) => {
                for _ in 0..n.lines.unwrap_or(1) {
                    lines.push(String::new());
                }
            }
            PrintNode::Image(_) => {
                eprintln!("to_plain_text: 'image' node has no text representation, printing a placeholder");
                lines.push("[image]".to_string());
            }
            PrintNode::Barcode(n) => {
                eprintln!(
                    "to_plain_text: 'barcode' node ({}) has no text representation, printing a placeholder",
                    n.symbology.name()
                );
                lines.push(format!("[barcode: {} {}]", n.symbology.name(), n.data));
            }
            PrintNode::QrCode(n) => {
                eprintln!("to_plain_text: 'qrcode' node has no text representation, printing a placeholder");
                lines.push(format!("[qrcode: {}]", n.data));
            }
            PrintNode::Cut(_) | PrintNode::Drawer(_) | PrintNode::Raw(_) => {
                // hardware-only actions; no-op for plain text
            }
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Align, BarcodeNode, BarcodeSymbology, CutNode, DrawerNode, FeedNode, ImageNode, Paper, PaperKind, RawNode, RowColumn, RowNode,
        RuleNode, TextNode,
    };

    fn doc(content: Vec<PrintNode>) -> PrintDocument {
        PrintDocument {
            version: "1.0".to_string(),
            paper: Paper { kind: PaperKind::Thermal58, width_mm: None, margin_mm: None, chars_per_line: None },
            meta: None,
            content,
        }
    }

    #[test]
    fn renders_a_rule_as_a_full_width_line_of_the_repeated_char() {
        let out = to_plain_text(&doc(vec![PrintNode::Rule(RuleNode { char: None })]));
        assert_eq!(out, "-".repeat(32));
    }

    #[test]
    fn renders_a_row_using_proportional_column_layout() {
        let out = to_plain_text(&doc(vec![PrintNode::Row(RowNode {
            columns: vec![
                RowColumn { text: "Item".into(), width: 1.0, align: Some(Align::Left) },
                RowColumn { text: "1.00".into(), width: 1.0, align: Some(Align::Right) },
            ],
            bold: None,
        })]));
        assert_eq!(out.chars().count(), 32);
        assert!(out.starts_with("Item"));
        assert!(out.trim_end().ends_with("1.00"));
    }

    #[test]
    fn emits_blank_lines_for_feed() {
        let out = to_plain_text(&doc(vec![
            PrintNode::Text(TextNode { value: "a".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None }),
            PrintNode::Feed(FeedNode { lines: Some(2) }),
            PrintNode::Text(TextNode { value: "b".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None }),
        ]));
        assert_eq!(out.split('\n').collect::<Vec<_>>(), vec!["a", "", "", "b"]);
    }

    #[test]
    fn degrades_image_barcode_qrcode_to_bracketed_placeholders() {
        let out = to_plain_text(&doc(vec![
            PrintNode::Image(ImageNode { data: "...".into(), width_px: None, align: None }),
            PrintNode::Barcode(BarcodeNode { symbology: BarcodeSymbology::Code128, data: "12345".into(), height_px: None, show_text: None }),
            PrintNode::QrCode(crate::types::QrCodeNode { data: "https://example.com".into(), size: None, ec: None }),
        ]));
        assert!(out.contains("[image]"));
        assert!(out.contains("[barcode: code128 12345]"));
        assert!(out.contains("[qrcode: https://example.com]"));
    }

    #[test]
    fn no_ops_cut_drawer_raw_nodes() {
        let out = to_plain_text(&doc(vec![
            PrintNode::Cut(CutNode { mode: None }),
            PrintNode::Drawer(DrawerNode { pin: None }),
            PrintNode::Raw(RawNode { escpos_base64: "AA==".into() }),
        ]));
        assert_eq!(out, "");
    }
}
