// PrintDocument -> self-contained HTML, for browser preview / window.print().
// Unlike escpos.ts/text.ts, HTML isn't grid-constrained, so `row` columns use
// real proportional (flexbox) widths instead of a fixed character grid.

import type { Align, PrintDocument, PrintNode } from "../types.js";
import { isThermal, resolveMarginMm, resolvePageHeightMm, resolvePageWidthMm } from "../types.js";
import { generateBarcodeBits } from "./barcode.js";
import { qrModules } from "./qr.js";

function escapeHtml(value: string): string {
  return value.replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!,
  );
}

function alignCss(align?: Align): string {
  return align === "center" || align === "right" ? align : "left";
}

function renderQrSvg(data: string, ec: NonNullable<Parameters<typeof qrModules>[1]>, modulePx = 4): string {
  const qr = qrModules(data, ec);
  const rects: string[] = [];
  for (let row = 0; row < qr.size; row++) {
    for (let col = 0; col < qr.size; col++) {
      if (qr.isDark(row, col)) rects.push(`<rect x="${col}" y="${row}" width="1" height="1"/>`);
    }
  }
  const px = qr.size * modulePx;
  return (
    `<svg width="${px}" height="${px}" viewBox="0 0 ${qr.size} ${qr.size}" shape-rendering="crispEdges">` +
    `<rect width="${qr.size}" height="${qr.size}" fill="#fff"/><g fill="#000">${rects.join("")}</g></svg>`
  );
}

function renderBarcodeSvg(bits: string, heightPx: number, moduleWidth = 2): string {
  const rects: string[] = [];
  for (let i = 0; i < bits.length; i++) {
    if (bits[i] === "1") rects.push(`<rect x="${i * moduleWidth}" y="0" width="${moduleWidth}" height="${heightPx}"/>`);
  }
  const width = bits.length * moduleWidth;
  return (
    `<svg width="${width}" height="${heightPx}" viewBox="0 0 ${width} ${heightPx}" shape-rendering="crispEdges">` +
    `<g fill="#000">${rects.join("")}</g></svg>`
  );
}

function placeholderBarcode(symbology: string, data: string): string {
  return (
    `<div style="border:1px solid #000;padding:4px 8px;display:inline-block;` +
    `text-align:center;font-family:monospace">[${symbology.toUpperCase()}: ${escapeHtml(data)}]</div>`
  );
}

function renderNode(node: PrintNode): string {
  switch (node.type) {
    case "text": {
      const style = [
        `text-align:${alignCss(node.align)}`,
        node.bold ? "font-weight:bold" : "",
        node.underline ? "text-decoration:underline" : "",
        node.invert ? "background:#000;color:#fff" : "",
        `font-size:${(node.size?.h ?? 1) * 100}%`,
        node.font === "b" ? "font-family:monospace" : "",
      ]
        .filter(Boolean)
        .join(";");
      return `<div style="${style}">${escapeHtml(node.value)}</div>`;
    }
    case "row": {
      const cols = node.columns
        .map(
          (c) =>
            `<div style="flex:${c.width} 0 0;text-align:${alignCss(c.align)};overflow:hidden;white-space:nowrap">${escapeHtml(c.text)}</div>`,
        )
        .join("");
      return `<div style="display:flex;gap:4px;${node.bold ? "font-weight:bold" : ""}">${cols}</div>`;
    }
    case "rule":
      return `<hr style="border:none;border-top:1px solid #000;margin:4px 0"/>`;
    case "feed":
      return `<div style="height:${node.lines ?? 1}em"></div>`;
    case "image": {
      const widthAttr = node.widthPx != null ? ` width="${node.widthPx}"` : "";
      return `<div style="text-align:${alignCss(node.align)}"><img alt="" src="data:image/png;base64,${node.data}"${widthAttr}/></div>`;
    }
    case "barcode": {
      const bits = generateBarcodeBits(node.symbology, node.data);
      if (bits === null) {
        // data invalid for this symbology -- degrade gracefully rather than throw
        // (see docs/DOCUMENT_FORMAT.md's renderer contract).
        return placeholderBarcode(node.symbology, node.data);
      }
      const hri = node.showText === false ? "" : `<div style="font-family:monospace;font-size:11px">${escapeHtml(node.data)}</div>`;
      return `<div style="text-align:center">${renderBarcodeSvg(bits, node.heightPx ?? 60)}${hri}</div>`;
    }
    case "qrcode":
      return `<div style="text-align:center">${renderQrSvg(node.data, node.ec ?? "M")}</div>`;
    case "cut":
    case "drawer":
    case "raw":
      return ""; // hardware-only actions; no-op for HTML/PDF preview
  }
}

export function toHtml(doc: PrintDocument): string {
  const widthMm = resolvePageWidthMm(doc.paper);
  const marginMm = resolveMarginMm(doc.paper);
  const heightMm = resolvePageHeightMm(doc.paper);
  const thermal = isThermal(doc.paper);
  const title = doc.meta?.title ?? "Print";
  const body = doc.content.map(renderNode).join("\n");
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
<style>
  @page { size: ${heightMm ? `${widthMm}mm ${heightMm}mm` : `${widthMm}mm auto`}; margin: ${marginMm}mm; }
  body {
    margin: 0 auto;
    width: ${widthMm}mm;
    padding: ${marginMm}mm;
    box-sizing: border-box;
    font-family: ${thermal ? "'Courier New', monospace" : "Arial, sans-serif"};
    font-size: ${thermal ? "12px" : "14px"};
  }
</style>
</head>
<body>
${body}
</body>
</html>
`;
}
