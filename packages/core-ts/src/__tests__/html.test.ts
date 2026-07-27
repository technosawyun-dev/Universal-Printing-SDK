import { describe, expect, it } from "vitest";
import { toHtml } from "../render/html.js";
import type { PrintDocument } from "../types.js";

function doc(content: PrintDocument["content"], kind: PrintDocument["paper"]["kind"] = "a4"): PrintDocument {
  return { version: "1.0", paper: { kind }, meta: { title: "Receipt #1" }, content };
}

describe("toHtml", () => {
  it("produces self-contained HTML with the doc title", () => {
    const html = toHtml(doc([{ type: "text", value: "Hello" }]));
    expect(html).toContain("<title>Receipt #1</title>");
    expect(html).toContain("Hello");
  });

  it("escapes HTML-significant characters in text content", () => {
    const html = toHtml(doc([{ type: "text", value: "<script>&\"'" }]));
    expect(html).not.toContain("<script>&\"'");
    expect(html).toContain("&lt;script&gt;&amp;&quot;&#39;");
  });

  it("lays out row columns with proportional flex widths", () => {
    const html = toHtml(
      doc([{ type: "row", columns: [{ text: "Item", width: 3, align: "left" }, { text: "1.00", width: 1, align: "right" }] }]),
    );
    expect(html).toContain("flex:3 0 0");
    expect(html).toContain("flex:1 0 0");
  });

  it("renders a real barcode as bars, plus HRI text below by default", () => {
    const html = toHtml(doc([{ type: "barcode", symbology: "ean13", data: "0123456789012" }]));
    expect(html).toContain("<svg");
    expect(html).toContain("<rect");
    expect(html).toContain("0123456789012");
  });

  it("omits HRI text when showText is false", () => {
    const html = toHtml(doc([{ type: "barcode", symbology: "ean13", data: "0123456789012", showText: false }]));
    expect(html).not.toContain("0123456789012");
  });

  it("degrades to a labeled placeholder box when data is invalid for the symbology", () => {
    const html = toHtml(doc([{ type: "barcode", symbology: "ean13", data: "not-digits" }]));
    expect(html).toContain("[EAN13: not-digits]");
  });

  it("renders qrcode as an inline SVG built from the qrcode package's modules", () => {
    const html = toHtml(doc([{ type: "qrcode", data: "https://example.com" }]));
    expect(html).toContain("<svg");
    expect(html).toContain("<rect");
  });

  it("no-ops cut/drawer/raw nodes", () => {
    const html = toHtml(doc([{ type: "cut" }, { type: "drawer" }, { type: "raw", escposBase64: "AA==" }]));
    expect(html).toContain("<body>");
  });

  it("sets page size from paper geometry", () => {
    const html = toHtml(doc([], "a4"));
    expect(html).toContain("size: 210mm 297mm");
  });
});
