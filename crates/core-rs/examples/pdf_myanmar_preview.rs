// Throwaway end-to-end sanity check: builds a real PrintDocument with
// Myanmar text/row nodes and renders it through the actual to_pdf()
// pipeline, saving a real PDF a human can open and read.
use printers_core::render::pdf::to_pdf;
use printers_core::types::*;

fn main() {
    let doc = PrintDocument {
        version: "1.0".into(),
        paper: Paper { kind: PaperKind::A4, width_mm: None, margin_mm: None, chars_per_line: None },
        meta: Some(DocMeta { title: Some("Myanmar PDF test".into()), encoding: None, locale: None }),
        content: vec![
            PrintNode::Text(TextNode { value: "ပြင်ဆင်မှု ဘောက်ချာ".into(), align: Some(Align::Center), bold: Some(true), underline: None, invert: None, size: Some(TextSize { w: 2, h: 2 }), font: None }),
            PrintNode::Rule(RuleNode { char: None }),
            PrintNode::Text(TextNode { value: "မင်္ဂလာပါ".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None }),
            PrintNode::Row(RowNode {
                columns: vec![
                    RowColumn { text: "ကုန်ပစ္စည်း".into(), width: 2.0, align: Some(Align::Left) },
                    RowColumn { text: "15,000 Ks".into(), width: 1.0, align: Some(Align::Right) },
                ],
                bold: None,
            }),
            PrintNode::Text(TextNode { value: "IMEI: 123456789012345 မော်ဒယ်".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None }),
        ],
    };

    let bytes = to_pdf(&doc).unwrap();
    std::fs::write("pdf_myanmar_preview.pdf", &bytes).unwrap();
    println!("wrote pdf_myanmar_preview.pdf ({} bytes)", bytes.len());
}
