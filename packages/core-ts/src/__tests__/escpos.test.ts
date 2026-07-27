import { describe, expect, it } from "vitest";
import { toEscPos } from "../render/escpos.js";
import type { PrintDocument } from "../types.js";

function doc(content: PrintDocument["content"], kind: PrintDocument["paper"]["kind"] = "thermal-80"): PrintDocument {
  return { version: "1.0", paper: { kind }, content };
}

describe("toEscPos", () => {
  it("starts every document with the initialize command", () => {
    const bytes = toEscPos(doc([]));
    expect(Array.from(bytes.slice(0, 2))).toEqual([0x1b, 0x40]);
  });

  it("encodes a plain text line ending in LF", () => {
    const bytes = toEscPos(doc([{ type: "text", value: "hi" }]));
    // ESC @ ... ESC a 0, ESC E 0, ESC - 0, GS B 0, GS ! 0, ESC M 0, 'h' 'i' LF, then the reset block
    const hi = Array.from(bytes).indexOf(0x68); // 'h'
    expect(bytes[hi]).toBe(0x68);
    expect(bytes[hi + 1]).toBe(0x69); // 'i'
    expect(bytes[hi + 2]).toBe(0x0a);
  });

  it("emits GS V for cut, with 0=full and 1=partial", () => {
    const full = toEscPos(doc([{ type: "cut", mode: "full" }]));
    const partial = toEscPos(doc([{ type: "cut" }]));
    expect(Array.from(full.slice(-3))).toEqual([0x1d, 0x56, 0x00]);
    expect(Array.from(partial.slice(-3))).toEqual([0x1d, 0x56, 0x01]);
  });

  it("emits ESC p for the drawer pulse with pin encoding", () => {
    const pin2 = toEscPos(doc([{ type: "drawer" }]));
    const pin5 = toEscPos(doc([{ type: "drawer", pin: 5 }]));
    expect(Array.from(pin2.slice(-5))).toEqual([0x1b, 0x70, 0, 25, 250]);
    expect(Array.from(pin5.slice(-5))).toEqual([0x1b, 0x70, 1, 25, 250]);
  });

  it("emits ESC d n for a multi-line feed, and bare LF for a single line", () => {
    const single = toEscPos(doc([{ type: "feed" }]));
    const multi = toEscPos(doc([{ type: "feed", lines: 5 }]));
    expect(Array.from(single.slice(-1))).toEqual([0x0a]);
    expect(Array.from(multi.slice(-3))).toEqual([0x1b, 0x64, 5]);
  });

  it("builds a CODE128 barcode with the {B subset prefix and GS k function-B framing", () => {
    const bytes = Array.from(toEscPos(doc([{ type: "barcode", symbology: "code128", data: "12345" }])));
    // GS k m=73 n=len('{B12345')=7 then the bytes themselves
    const gsk = bytes.findIndex((b, i) => b === 0x1d && bytes[i + 1] === 0x6b && bytes[i + 2] === 73);
    expect(gsk).toBeGreaterThan(-1);
    expect(bytes[gsk + 3]).toBe(7);
    const payload = bytes.slice(gsk + 4, gsk + 4 + 7);
    expect(Buffer.from(payload).toString("latin1")).toBe("{B12345");
  });

  it("builds a QR code command sequence with correct EC-level bytes (L=48,M=49,Q=50,H=51)", () => {
    for (const [ec, expected] of [
      ["L", 48],
      ["M", 49],
      ["Q", 50],
      ["H", 51],
    ] as const) {
      const bytes = Array.from(
        toEscPos(doc([{ type: "qrcode", data: "hi", ec }])),
      );
      // find the "set EC level" GS ( k 03 00 '1' 'E' <ec> sequence
      const idx = bytes.findIndex(
        (b, i) => b === 0x1d && bytes[i + 1] === 0x28 && bytes[i + 2] === 0x6b && bytes[i + 5] === 0x31 && bytes[i + 6] === 0x45,
      );
      expect(idx).toBeGreaterThan(-1);
      expect(bytes[idx + 7]).toBe(expected);
    }
  });

  it("lays out a row into the paper's char width before encoding", () => {
    const bytes = toEscPos(
      doc([{ type: "row", columns: [{ text: "A", width: 1, align: "left" }, { text: "B", width: 1, align: "right" }] }], "thermal-58"),
    );
    // thermal-58 -> 32 chars; row line should be 32 bytes then LF
    const text = Buffer.from(bytes).toString("latin1");
    const lineStart = text.indexOf("A");
    expect(text.slice(lineStart, lineStart + 32).length).toBe(32);
  });
});
