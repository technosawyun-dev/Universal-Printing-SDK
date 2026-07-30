// Throwaway end-to-end sanity check: builds a real PrintDocument with
// Myanmar text nodes and a row, renders it through the actual to_escpos()
// pipeline (not the shaping function in isolation), parses the GS v 0 raster
// commands back out of the resulting bytes, and reassembles them into a PNG
// so a human can visually confirm the real renderer's output.
use printers_core::render::escpos::to_escpos;
use printers_core::types::*;

fn main() {
    let doc = PrintDocument {
        version: "1.0".into(),
        paper: Paper { kind: PaperKind::Thermal80, width_mm: None, margin_mm: None, chars_per_line: None },
        meta: None,
        content: vec![
            PrintNode::Text(TextNode { value: "မင်္ဂလာပါ".into(), align: Some(Align::Center), bold: None, underline: None, invert: None, size: None, font: None }),
            PrintNode::Text(TextNode { value: "ပြင်ဆင်မှု ဘောက်ချာ".into(), align: None, bold: None, underline: None, invert: None, size: None, font: None }),
            PrintNode::Row(RowNode {
                columns: vec![
                    RowColumn { text: "ကုန်ပစ္စည်း".into(), width: 2.0, align: Some(Align::Left) },
                    RowColumn { text: "15,000 Ks".into(), width: 1.0, align: Some(Align::Right) },
                ],
                bold: None,
            }),
        ],
    };

    let bytes = to_escpos(&doc).unwrap();
    println!("total bytes: {}", bytes.len());

    // Parse out every GS v 0 raster block: 1D 76 30 00 xL xH yL yH <data>
    let mut blocks = Vec::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        if bytes[i] == 0x1d && bytes[i + 1] == 0x76 && bytes[i + 2] == 0x30 && bytes[i + 3] == 0x00 {
            let bytes_per_row = bytes[i + 4] as u32 | ((bytes[i + 5] as u32) << 8);
            let rows = bytes[i + 6] as u32 | ((bytes[i + 7] as u32) << 8);
            let data_len = (bytes_per_row * rows) as usize;
            let data = &bytes[i + 8..i + 8 + data_len];
            blocks.push((bytes_per_row, rows, data.to_vec()));
            i += 8 + data_len;
        } else {
            i += 1;
        }
    }
    println!("found {} raster block(s)", blocks.len());

    let width_dots = blocks.iter().map(|(bpr, _, _)| bpr * 8).max().unwrap_or(384);
    let total_height: u32 = blocks.iter().map(|(_, rows, _)| rows + 4).sum();
    let mut canvas = vec![255u8; (width_dots * total_height) as usize];

    let mut y0 = 0u32;
    for (bytes_per_row, rows, data) in &blocks {
        for y in 0..*rows {
            for x in 0..(bytes_per_row * 8).min(width_dots) {
                let byte = data[(y * bytes_per_row + (x >> 3)) as usize];
                let bit = (byte >> (7 - (x & 7))) & 1;
                if bit == 1 {
                    let idx = ((y0 + y) * width_dots + x) as usize;
                    canvas[idx] = 0;
                }
            }
        }
        y0 += rows + 4;
    }

    let file = std::fs::File::create("escpos_myanmar_preview.png").unwrap();
    let mut encoder = png::Encoder::new(file, width_dots, total_height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&canvas).unwrap();
    println!("wrote escpos_myanmar_preview.png ({width_dots}x{total_height})");
}
