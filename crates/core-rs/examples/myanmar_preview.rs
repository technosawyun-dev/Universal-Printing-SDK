// Throwaway visual sanity-check: shapes a mix of Myanmar and Latin text and
// writes it to a PNG so a human (who reads Myanmar, unlike this codebase's
// author) can confirm it actually reads correctly, not just "renders without
// crashing." Not part of the library, not referenced by any renderer.
use printers_core::render::text_shaping::shape_text_to_bitmap;

fn main() {
    let samples = [
        "မင်္ဂလာပါ",
        "ပြင်ဆင်မှု ဘောက်ချာ",
        "IMEI: 123456789012345 မော်ဒယ်",
        "စုစုပေါင်း ငွေပမာဏ: 15,000 Ks",
    ];

    let padding = 10u32;
    let line_height = 110u32;
    let width = 1400u32;
    let height = line_height * samples.len() as u32 + padding * 2;

    let mut canvas = vec![255u8; (width * height) as usize];

    for (i, text) in samples.iter().enumerate() {
        let bitmap = shape_text_to_bitmap(text, 80.0, false).expect("should shape");
        let y0 = padding + i as u32 * line_height;
        for y in 0..bitmap.height.min(line_height) {
            for x in 0..bitmap.width.min(width - padding) {
                let src = bitmap.coverage[(y * bitmap.width + x) as usize];
                let dst_x = padding + x;
                let dst_y = y0 + y;
                if dst_x < width && dst_y < height {
                    let idx = (dst_y * width + dst_x) as usize;
                    canvas[idx] = 255 - src;
                }
            }
        }
    }

    let path = "myanmar_preview.png";
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&canvas).unwrap();
    println!("wrote {path}");
}
