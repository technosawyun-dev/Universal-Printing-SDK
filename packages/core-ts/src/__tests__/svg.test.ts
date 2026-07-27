import { describe, expect, it } from "vitest";
import { toSvg } from "../render/svg.js";
import type { PrintDocument } from "../types.js";

function doc(content: PrintDocument["content"], kind: PrintDocument["paper"]["kind"] = "thermal-80"): PrintDocument {
  return { version: "1.0", paper: { kind }, content };
}

describe("toSvg", () => {
  it("produces a valid svg root element sized from the paper width", () => {
    const svg = toSvg(doc([{ type: "text", value: "hi" }]));
    expect(svg.startsWith("<svg")).toBe(true);
    expect(svg).toContain('width="320"'); // 80mm * 4 px/mm
  });

  it("stacks nodes vertically, growing the canvas height", () => {
    const one = toSvg(doc([{ type: "text", value: "a" }]));
    const three = toSvg(doc([{ type: "text", value: "a" }, { type: "text", value: "b" }, { type: "text", value: "c" }]));
    const heightOf = (svg: string) => Number(/height="(\d+)"/.exec(svg)![1]);
    expect(heightOf(three)).toBeGreaterThan(heightOf(one));
  });

  it("renders a real barcode as bars scaled to the content width, plus HRI text below", () => {
    const svg = toSvg(doc([{ type: "barcode", symbology: "code39", data: "ABC123" }]));
    expect(svg).toContain("<rect");
    expect(svg).toContain("ABC123");
  });

  it("degrades to a labeled placeholder box when data is invalid for the symbology", () => {
    const svg = toSvg(doc([{ type: "barcode", symbology: "code39", data: "lower@case" }]));
    expect(svg).toContain("[CODE39: lower@case]");
  });

  it("renders qrcode as filled rects from the qrcode package's modules", () => {
    const svg = toSvg(doc([{ type: "qrcode", data: "https://example.com" }]));
    expect(svg).toContain("<rect");
  });

  it("skips cut/drawer/raw nodes without adding height", () => {
    const empty = toSvg(doc([]));
    const withNoOps = toSvg(doc([{ type: "cut" }, { type: "drawer" }, { type: "raw", escposBase64: "AA==" }]));
    expect(withNoOps).toBe(empty);
  });
});
