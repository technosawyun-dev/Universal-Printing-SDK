//! Real 1D barcode bar-pattern generation, for HTML/SVG/PDF *preview* only --
//! mirrors `packages/core-ts/src/render/barcode.ts` exactly; see that file's
//! header comment for the full rationale and verification method (checked
//! against the `python-barcode` reference library, not transcribed from
//! memory).

use crate::types::BarcodeSymbology;

/// Module bit string: '1' = dark bar, '0' = light space, quiet zones excluded.
pub type BarcodeBits = String;

const CODE39_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
const CODE39_PATTERNS: [&str; 43] = [
    "101000111011101", "111010001010111", "101110001010111", "111011100010101", "101000111010111", "111010001110101",
    "101110001110101", "101000101110111", "111010001011101", "101110001011101", "111010100010111", "101110100010111",
    "111011101000101", "101011100010111", "111010111000101", "101110111000101", "101010001110111", "111010100011101",
    "101110100011101", "101011100011101", "111010101000111", "101110101000111", "111011101010001", "101011101000111",
    "111010111010001", "101110111010001", "101010111000111", "111010101110001", "101110101110001", "101011101110001",
    "111000101010111", "100011101010111", "111000111010101", "100010111010111", "111000101110101", "100011101110101",
    "100010101110111", "111000101011101", "100011101011101", "100010001000101", "100010001010001", "100010100010001",
    "101000100010001",
];
const CODE39_EDGE: &str = "100010111011101";

fn code39_bits(data: &str) -> Option<BarcodeBits> {
    let upper = data.to_uppercase();
    if upper.is_empty() {
        return None;
    }
    let mut indices = Vec::with_capacity(upper.len());
    for ch in upper.chars() {
        indices.push(CODE39_CHARS.find(ch)?);
    }
    let mut parts = vec![CODE39_EDGE.to_string()];
    parts.extend(indices.iter().map(|&i| CODE39_PATTERNS[i].to_string()));
    parts.push(CODE39_EDGE.to_string());
    Some(parts.join("0"))
}

const EAN_UPC_EDGE: &str = "101";
const EAN_MIDDLE: &str = "01010";

const EAN_CODES_A: [&str; 10] = ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"];
const EAN_CODES_B: [&str; 10] = ["0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"];
const EAN_CODES_C: [&str; 10] = ["1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"];
const EAN_LEFT_PATTERN: [&str; 10] = ["AAAAAA", "AABABB", "AABBAB", "AABBBA", "ABAABB", "ABBAAB", "ABBBAA", "ABABAB", "ABABBA", "ABBABA"];

fn digit_at(s: &str, i: usize) -> Option<u32> {
    s.as_bytes().get(i).and_then(|b| (*b as char).to_digit(10))
}

fn ean13_check_digit(twelve_digits: &str) -> u32 {
    let mut even_sum = 0u32;
    let mut odd_sum = 0u32;
    for i in 0..12 {
        let d = digit_at(twelve_digits, i).unwrap();
        if i % 2 == 0 {
            even_sum += d;
        } else {
            odd_sum += d;
        }
    }
    (10 - ((even_sum + odd_sum * 3) % 10)) % 10
}

fn ean13_bits(data: &str) -> Option<BarcodeBits> {
    if !(data.len() == 12 || data.len() == 13) || !data.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let base = &data[0..12];
    let check_digit = ean13_check_digit(base);
    let digits = format!("{base}{check_digit}");

    let pattern = EAN_LEFT_PATTERN[digit_at(&digits, 0)? as usize];
    let mut left = String::new();
    for i in 0..6 {
        let table = if pattern.as_bytes()[i] == b'A' { &EAN_CODES_A } else { &EAN_CODES_B };
        left.push_str(table[digit_at(&digits, 1 + i)? as usize]);
    }
    let mut right = String::new();
    for i in 0..6 {
        right.push_str(EAN_CODES_C[digit_at(&digits, 7 + i)? as usize]);
    }
    Some(format!("{EAN_UPC_EDGE}{left}{EAN_MIDDLE}{right}{EAN_UPC_EDGE}"))
}

const UPC_CODES_L: [&str; 10] = ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"];
const UPC_CODES_R: [&str; 10] = ["1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"];

fn upca_check_digit(eleven_digits: &str) -> u32 {
    let mut odd_sum = 0u32; // positions 0,2,4,6,8,10
    let mut even_sum = 0u32; // positions 1,3,5,7,9
    for i in 0..11 {
        let d = digit_at(eleven_digits, i).unwrap();
        if i % 2 == 0 {
            odd_sum += d;
        } else {
            even_sum += d;
        }
    }
    let check = (even_sum + odd_sum * 3) % 10;
    if check == 0 {
        0
    } else {
        10 - check
    }
}

fn upca_bits(data: &str) -> Option<BarcodeBits> {
    if !(data.len() == 11 || data.len() == 12) || !data.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let base = &data[0..11];
    let check_digit = upca_check_digit(base);
    let digits = format!("{base}{check_digit}");

    let mut left = String::new();
    for i in 0..6 {
        left.push_str(UPC_CODES_L[digit_at(&digits, i)? as usize]);
    }
    let mut right = String::new();
    for i in 0..6 {
        right.push_str(UPC_CODES_R[digit_at(&digits, 6 + i)? as usize]);
    }
    Some(format!("{EAN_UPC_EDGE}{left}{EAN_MIDDLE}{right}{EAN_UPC_EDGE}"))
}

// CODE128 subset B: printable ASCII 0x20-0x7E map directly to value (charCode - 32).
const CODE128_CODES: [&str; 106] = [
    "11011001100", "11001101100", "11001100110", "10010011000", "10010001100", "10001001100", "10011001000",
    "10011000100", "10001100100", "11001001000", "11001000100", "11000100100", "10110011100", "10011011100",
    "10011001110", "10111001100", "10011101100", "10011100110", "11001110010", "11001011100", "11001001110",
    "11011100100", "11001110100", "11101101110", "11101001100", "11100101100", "11100100110", "11101100100",
    "11100110100", "11100110010", "11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
    "10001000110", "10110001000", "10001101000", "10001100010", "11010001000", "11000101000", "11000100010",
    "10110111000", "10110001110", "10001101110", "10111011000", "10111000110", "10001110110", "11101110110",
    "11010001110", "11000101110", "11011101000", "11011100010", "11011101110", "11101011000", "11101000110",
    "11100010110", "11101101000", "11101100010", "11100011010", "11101111010", "11001000010", "11110001010",
    "10100110000", "10100001100", "10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
    "10110000100", "10011010000", "10011000010", "10000110100", "10000110010", "11000010010", "11001010000",
    "11110111010", "11000010100", "10001111010", "10100111100", "10010111100", "10010011110", "10111100100",
    "10011110100", "10011110010", "11110100100", "11110010100", "11110010010", "11011011110", "11011110110",
    "11110110110", "10101111000", "10100011110", "10001011110", "10111101000", "10111100010", "11110101000",
    "11110100010", "10111011110", "10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
    "11010011100",
];
const CODE128_START_B: usize = 104;
const CODE128_STOP_SUFFIX: &str = "1100011101011"; // STOP ("11000111010") + termination bar ("11")

fn code128_bits(data: &str) -> Option<BarcodeBits> {
    if data.is_empty() {
        return None;
    }
    let mut values = Vec::with_capacity(data.len());
    for ch in data.chars() {
        let code = ch as u32;
        if !(0x20..=0x7e).contains(&code) {
            return None;
        }
        values.push((code - 0x20) as usize);
    }
    let mut checksum = CODE128_START_B;
    for (i, &v) in values.iter().enumerate() {
        checksum += (i + 1) * v;
    }
    checksum %= 103;

    let mut out = String::from(CODE128_CODES[CODE128_START_B]);
    for &v in &values {
        out.push_str(CODE128_CODES[v]);
    }
    out.push_str(CODE128_CODES[checksum]);
    out.push_str(CODE128_STOP_SUFFIX);
    Some(out)
}

/// Returns `None` (degrade to a labeled placeholder, see docs/DOCUMENT_FORMAT.md) if `data` is invalid for `symbology`.
pub fn generate_barcode_bits(symbology: BarcodeSymbology, data: &str) -> Option<BarcodeBits> {
    match symbology {
        BarcodeSymbology::Code39 => code39_bits(data),
        BarcodeSymbology::Ean13 => ean13_bits(data),
        BarcodeSymbology::Upca => upca_bits(data),
        BarcodeSymbology::Code128 => code128_bits(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every expected bit string below was generated with the `python-barcode`
    // reference library, not transcribed from memory -- see this module's
    // header comment.

    #[test]
    fn matches_python_barcode_ean13_exactly() {
        // EuropeanArticleNumber13('400638133393').build()
        assert_eq!(
            generate_barcode_bits(BarcodeSymbology::Ean13, "400638133393").unwrap(),
            "10100011010100111010111101111010001001011001101010100001010000101000010111010010000101100110101"
        );
    }

    #[test]
    fn matches_python_barcode_upca_exactly() {
        // UniversalProductCodeA('03600029145', make_ean=False).build()
        assert_eq!(
            generate_barcode_bits(BarcodeSymbology::Upca, "03600029145").unwrap(),
            "10100011010111101010111100011010001101000110101010110110011101001100110101110010011101101100101"
        );
    }

    #[test]
    fn matches_python_barcode_code39_exactly_no_checksum() {
        // Code39('CODE39', add_checksum=False).build()
        assert_eq!(
            generate_barcode_bits(BarcodeSymbology::Code39, "CODE39").unwrap(),
            "1000101110111010111011101000101011101011101000101010111000101110111010111000101011101110001010101011100010111010100010111011101"
        );
    }

    #[test]
    fn matches_python_barcode_code128_pure_subset_b() {
        // Code128('ABC123').build() -- short enough that python-barcode
        // stays in subset B the whole way, matching our subset-B-only encoder.
        assert_eq!(
            generate_barcode_bits(BarcodeSymbology::Code128, "ABC123").unwrap(),
            "11010010000101000110001000101100010001000110100111001101100111001011001011100100001011001100011101011"
        );
    }

    #[test]
    fn code128_starts_with_start_b_and_ends_with_stop() {
        let bits = generate_barcode_bits(BarcodeSymbology::Code128, "12345").unwrap();
        assert!(bits.starts_with(CODE128_CODES[CODE128_START_B]));
        assert!(bits.ends_with(CODE128_STOP_SUFFIX));
    }

    #[test]
    fn degrades_to_none_for_invalid_data() {
        assert!(generate_barcode_bits(BarcodeSymbology::Ean13, "not-digits").is_none());
        assert!(generate_barcode_bits(BarcodeSymbology::Upca, "123").is_none());
        assert!(generate_barcode_bits(BarcodeSymbology::Code39, "lower@case").is_none());
        assert!(generate_barcode_bits(BarcodeSymbology::Code128, "").is_none());
    }

    #[test]
    fn code39_is_case_insensitive() {
        assert_eq!(
            generate_barcode_bits(BarcodeSymbology::Code39, "code39"),
            generate_barcode_bits(BarcodeSymbology::Code39, "CODE39")
        );
    }
}
