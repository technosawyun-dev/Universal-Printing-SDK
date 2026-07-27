// Real 1D barcode bar-pattern generation, for HTML/SVG/PDF *preview* only --
// the actual ESC/POS thermal print path (escpos.ts) sends raw data and lets
// the printer's onboard encoder draw the barcode; it never needed this.
//
// Symbol tables (CODE39, EAN-13, UPC-A, CODE128 subset B) are the standard,
// publicly specified bar patterns for each symbology (ISO/IEC 15417,
// EN 797, UPC-A) -- functional specification data, not creative content.
// Verified byte-for-byte against the `python-barcode` reference library
// (github.com/WhyNotHugo/python-barcode) rather than transcribed from
// memory: see the exact matching test vectors in
// __tests__/barcode.test.ts. CODE128 here is subset-B only (no A/C
// switching), matching escpos.ts's own `{B` prefix, which forces the same
// subset on real hardware -- so preview and print always agree.
//
// docs/ARCHITECTURE.md: this closes a real gap found by comparing our
// html.ts/svg.ts against ReceiptLine (github.com/receiptline/receiptline),
// which does generate real barcode graphics for preview instead of a
// placeholder box.

import type { BarcodeSymbology } from "../types.js";

/** Module bit string: '1' = dark bar, '0' = light space, quiet zones excluded. */
export type BarcodeBits = string;

const CODE39_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
const CODE39_PATTERNS = [
  "101000111011101",
  "111010001010111",
  "101110001010111",
  "111011100010101",
  "101000111010111",
  "111010001110101",
  "101110001110101",
  "101000101110111",
  "111010001011101",
  "101110001011101",
  "111010100010111",
  "101110100010111",
  "111011101000101",
  "101011100010111",
  "111010111000101",
  "101110111000101",
  "101010001110111",
  "111010100011101",
  "101110100011101",
  "101011100011101",
  "111010101000111",
  "101110101000111",
  "111011101010001",
  "101011101000111",
  "111010111010001",
  "101110111010001",
  "101010111000111",
  "111010101110001",
  "101110101110001",
  "101011101110001",
  "111000101010111",
  "100011101010111",
  "111000111010101",
  "100010111010111",
  "111000101110101",
  "100011101110101",
  "100010101110111",
  "111000101011101",
  "100011101011101",
  "100010001000101",
  "100010001010001",
  "100010100010001",
  "101000100010001",
];
const CODE39_EDGE = "100010111011101";
const CODE39_MIDDLE = "0";

function code39Bits(data: string): BarcodeBits | null {
  const upper = data.toUpperCase();
  const indices: number[] = [];
  for (const ch of upper) {
    const idx = CODE39_CHARS.indexOf(ch);
    if (idx === -1) return null;
    indices.push(idx);
  }
  if (indices.length === 0) return null;
  const parts = [CODE39_EDGE, ...indices.map((i) => CODE39_PATTERNS[i]), CODE39_EDGE];
  return parts.join(CODE39_MIDDLE);
}

const EAN_UPC_EDGE = "101";
const EAN_MIDDLE = "01010";
const UPC_MIDDLE = "01010";

const EAN_CODES: Record<"A" | "B" | "C", string[]> = {
  A: ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"],
  B: ["0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"],
  C: ["1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"],
};
const EAN_LEFT_PATTERN = ["AAAAAA", "AABABB", "AABBAB", "AABBBA", "ABAABB", "ABBAAB", "ABBBAA", "ABABAB", "ABABBA", "ABBABA"];

function ean13CheckDigit(twelveDigits: string): number {
  let evenSum = 0;
  let oddSum = 0;
  for (let i = 0; i < 12; i++) {
    const d = Number(twelveDigits[i]);
    if (i % 2 === 0) evenSum += d;
    else oddSum += d;
  }
  return (10 - ((evenSum + oddSum * 3) % 10)) % 10;
}

function ean13Bits(data: string): BarcodeBits | null {
  if (!/^\d{12,13}$/.test(data)) return null;
  const base = data.slice(0, 12);
  const checkDigit = ean13CheckDigit(base);
  const digits = base + String(checkDigit);

  const pattern = EAN_LEFT_PATTERN[Number(digits[0])];
  let left = "";
  for (let i = 0; i < 6; i++) {
    const table = pattern[i] === "A" ? EAN_CODES.A : EAN_CODES.B;
    left += table[Number(digits[1 + i])];
  }
  let right = "";
  for (let i = 0; i < 6; i++) {
    right += EAN_CODES.C[Number(digits[7 + i])];
  }
  return EAN_UPC_EDGE + left + EAN_MIDDLE + right + EAN_UPC_EDGE;
}

const UPC_CODES: Record<"L" | "R", string[]> = {
  L: ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"],
  R: ["1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"],
};

function upcaCheckDigit(elevenDigits: string): number {
  let oddSum = 0; // positions 0,2,4,6,8,10 (Python's upc[::2])
  let evenSum = 0; // positions 1,3,5,7,9 (Python's upc[1::2])
  for (let i = 0; i < 11; i++) {
    const d = Number(elevenDigits[i]);
    if (i % 2 === 0) oddSum += d;
    else evenSum += d;
  }
  const check = (evenSum + oddSum * 3) % 10;
  return check === 0 ? 0 : 10 - check;
}

function upcaBits(data: string): BarcodeBits | null {
  if (!/^\d{11,12}$/.test(data)) return null;
  const base = data.slice(0, 11);
  const checkDigit = upcaCheckDigit(base);
  const digits = base + String(checkDigit);

  let left = "";
  for (let i = 0; i < 6; i++) left += UPC_CODES.L[Number(digits[i])];
  let right = "";
  for (let i = 0; i < 6; i++) right += UPC_CODES.R[Number(digits[6 + i])];
  return EAN_UPC_EDGE + left + UPC_MIDDLE + right + EAN_UPC_EDGE;
}

// CODE128 subset B: printable ASCII 0x20-0x7E map directly to value (charCode - 32).
const CODE128_CODES = [
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
const CODE128_START_B = 104;
const CODE128_STOP = "11000111010" + "11";

function code128Bits(data: string): BarcodeBits | null {
  if (data.length === 0) return null;
  const values: number[] = [];
  for (const ch of data) {
    const code = ch.charCodeAt(0);
    if (code < 0x20 || code > 0x7e) return null;
    values.push(code - 0x20);
  }
  let checksum = CODE128_START_B;
  for (let i = 0; i < values.length; i++) checksum += (i + 1) * values[i];
  checksum %= 103;

  const parts = [CODE128_CODES[CODE128_START_B], ...values.map((v) => CODE128_CODES[v]), CODE128_CODES[checksum]];
  return parts.join("") + CODE128_STOP;
}

/** Returns null (degrade to a labeled placeholder, see docs/DOCUMENT_FORMAT.md) if `data` is invalid for `symbology`. */
export function generateBarcodeBits(symbology: BarcodeSymbology, data: string): BarcodeBits | null {
  switch (symbology) {
    case "code39":
      return code39Bits(data);
    case "ean13":
      return ean13Bits(data);
    case "upca":
      return upcaBits(data);
    case "code128":
      return code128Bits(data);
  }
}
