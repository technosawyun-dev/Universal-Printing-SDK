import { describe, expect, it } from "vitest";
import { toPlainText } from "../render/text.js";
import type { PrintDocument } from "../types.js";

function doc(content: PrintDocument["content"]): PrintDocument {
  return { version: "1.0", paper: { kind: "thermal-58" }, content };
}

describe("toPlainText", () => {
  it("renders a rule as a full-width line of the repeated char", () => {
    const out = toPlainText(doc([{ type: "rule" }]));
    expect(out).toBe("-".repeat(32));
  });

  it("renders a row using proportional column layout", () => {
    const out = toPlainText(
      doc([{ type: "row", columns: [{ text: "Item", width: 1, align: "left" }, { text: "1.00", width: 1, align: "right" }] }]),
    );
    expect(out.length).toBe(32);
    expect(out.startsWith("Item")).toBe(true);
    expect(out.trimEnd().endsWith("1.00")).toBe(true);
  });

  it("emits blank lines for feed", () => {
    const out = toPlainText(doc([{ type: "text", value: "a" }, { type: "feed", lines: 2 }, { type: "text", value: "b" }]));
    expect(out.split("\n")).toEqual(["a", "", "", "b"]);
  });

  it("degrades image/barcode/qrcode to bracketed placeholders", () => {
    const out = toPlainText(
      doc([
        { type: "image", data: "..." },
        { type: "barcode", symbology: "code128", data: "12345" },
        { type: "qrcode", data: "https://example.com" },
      ]),
    );
    expect(out).toContain("[image]");
    expect(out).toContain("[barcode: code128 12345]");
    expect(out).toContain("[qrcode: https://example.com]");
  });

  it("no-ops cut/drawer/raw nodes", () => {
    const out = toPlainText(doc([{ type: "cut" }, { type: "drawer" }, { type: "raw", escposBase64: "AA==" }]));
    expect(out).toBe("");
  });
});
