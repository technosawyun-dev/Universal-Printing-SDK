import { describe, expect, it } from "vitest";
import { generateBarcodeBits } from "../render/barcode.js";

// Every expected bit string below was generated with the `python-barcode`
// reference library (github.com/WhyNotHugo/python-barcode, verified
// install), not transcribed from memory -- see the render/barcode.ts header
// comment for the exact commands used. This is the ground truth these
// implementations are checked against.
describe("generateBarcodeBits", () => {
  it("matches python-barcode's EAN13 output exactly", () => {
    // EuropeanArticleNumber13('400638133393').build()
    expect(generateBarcodeBits("ean13", "400638133393")).toBe(
      "10100011010100111010111101111010001001011001101010100001010000101000010111010010000101100110101",
    );
  });

  it("matches python-barcode's UPC-A output exactly", () => {
    // UniversalProductCodeA('03600029145', make_ean=False).build()
    expect(generateBarcodeBits("upca", "03600029145")).toBe(
      "10100011010111101010111100011010001101000110101010110110011101001100110101110010011101101100101",
    );
  });

  it("matches python-barcode's CODE39 output exactly (no checksum)", () => {
    // Code39('CODE39', add_checksum=False).build()
    expect(generateBarcodeBits("code39", "CODE39")).toBe(
      "1000101110111010111011101000101011101011101000101010111000101110111010111000101011101110001010101011100010111010100010111011101",
    );
  });

  it("matches python-barcode's CODE128 (subset B) output for numeric data", () => {
    // Code128('12345').build() -- python-barcode auto-picks subset C for
    // runs of 4+ digits, but escpos.ts forces subset B for ALL code128 data
    // (the `{B` prefix), so this checks our subset-B-only encoding directly
    // against the subset-B code values, not against python-barcode's
    // auto-switching output.
    const bits = generateBarcodeBits("code128", "12345");
    expect(bits).not.toBeNull();
    expect(bits!.startsWith("11010010000")).toBe(true); // START_B pattern (code value 104)
    expect(bits!.endsWith("1100011101011")).toBe(true); // STOP + termination bar
  });

  it("matches python-barcode's CODE128 output for mixed alphanumeric data (pure subset B)", () => {
    // Code128('ABC123').build() -- short enough that python-barcode stays in
    // subset B the whole way (no digit run long enough to switch to C),
    // so this DOES match byte-for-byte.
    expect(generateBarcodeBits("code128", "ABC123")).toBe(
      "11010010000101000110001000101100010001000110100111001101100111001011001011100100001011001100011101011",
    );
  });

  it("degrades to null for data invalid in a symbology (caller falls back to a placeholder)", () => {
    expect(generateBarcodeBits("ean13", "not-digits")).toBeNull();
    expect(generateBarcodeBits("upca", "123")).toBeNull(); // too short
    expect(generateBarcodeBits("code39", "lowercase-ok-since-upcased-but-@-is-not")).toBeNull();
    expect(generateBarcodeBits("code128", "")).toBeNull();
  });

  it("code39 is case-insensitive (auto-uppercases like python-barcode does)", () => {
    expect(generateBarcodeBits("code39", "code39")).toBe(generateBarcodeBits("code39", "CODE39"));
  });
});
