// PrintDocument -> SVG, for thumbnails / non-print preview (not a print
// target itself — see docs/DOCUMENT_FORMAT.md renderer contract table).
// Not pixel-accurate to any physical printer; a fixed px-per-mm scale is
// enough for a thumbnail.

import type { Align, PrintDocument, PrintNode } from "../types.js";
import { resolveMarginMm, resolvePageWidthMm } from "../types.js";
import { generateBarcodeBits } from "./barcode.js";
import { qrModules } from "./qr.js";

const PX_PER_MM = 4;
const LINE_HEIGHT = 18;
const FONT_SIZE = 13;

function escapeXml(value: string): string {
  return value.replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&apos;" })[c]!,
  );
}

function anchorFor(align?: Align): { x: (w: number) => number; anchor: string } {
  if (align === "center") return { x: (w) => w / 2, anchor: "middle" };
  if (align === "right") return { x: (w) => w, anchor: "end" };
  return { x: () => 0, anchor: "start" };
}

interface Block {
  height: number;
  svg: string;
}

function textBlock(value: string, contentWidth: number, align: Align | undefined, opts: { bold?: boolean; underline?: boolean; sizeH?: number }): Block {
  const { x, anchor } = anchorFor(align);
  const fontSize = FONT_SIZE * (opts.sizeH ?? 1);
  const height = LINE_HEIGHT * (opts.sizeH ?? 1);
  const attrs = [
    `x="${x(contentWidth)}"`,
    `y="${height * 0.75}"`,
    `text-anchor="${anchor}"`,
    `font-size="${fontSize}"`,
    opts.bold ? `font-weight="bold"` : "",
    opts.underline ? `text-decoration="underline"` : "",
  ]
    .filter(Boolean)
    .join(" ");
  return { height, svg: `<text ${attrs}>${escapeXml(value)}</text>` };
}

function rowBlock(node: Extract<PrintNode, { type: "row" }>, contentWidth: number): Block {
  const totalUnits = node.columns.reduce((sum, c) => sum + c.width, 0);
  let cursor = 0;
  const parts: string[] = [];
  for (const col of node.columns) {
    const w = (col.width / totalUnits) * contentWidth;
    const { x, anchor } = anchorFor(col.align);
    parts.push(
      `<text x="${cursor + x(w)}" y="${LINE_HEIGHT * 0.75}" text-anchor="${anchor}" font-size="${FONT_SIZE}"` +
        `${node.bold ? ' font-weight="bold"' : ""}>${escapeXml(col.text)}</text>`,
    );
    cursor += w;
  }
  return { height: LINE_HEIGHT, svg: parts.join("") };
}

function qrBlock(data: string, ec: NonNullable<Parameters<typeof qrModules>[1]>, contentWidth: number): Block {
  const qr = qrModules(data, ec);
  const px = Math.min(contentWidth, LINE_HEIGHT * 6);
  const moduleSize = px / qr.size;
  const rects: string[] = [];
  for (let row = 0; row < qr.size; row++) {
    for (let col = 0; col < qr.size; col++) {
      if (qr.isDark(row, col)) {
        rects.push(`<rect x="${col * moduleSize}" y="${row * moduleSize}" width="${moduleSize}" height="${moduleSize}"/>`);
      }
    }
  }
  const offsetX = (contentWidth - px) / 2;
  return {
    height: px,
    svg: `<g transform="translate(${offsetX},0)" fill="#000">${rects.join("")}</g>`,
  };
}

function placeholderBarcodeBlock(symbology: string, data: string, contentWidth: number): Block {
  const height = LINE_HEIGHT * 2;
  return {
    height,
    svg:
      `<rect x="0" y="0" width="${contentWidth}" height="${height}" fill="none" stroke="#000"/>` +
      `<text x="${contentWidth / 2}" y="${height / 2 + 4}" text-anchor="middle" font-size="${FONT_SIZE}" font-family="monospace">` +
      `[${escapeXml(symbology.toUpperCase())}: ${escapeXml(data)}]</text>`,
  };
}

function barcodeBlock(node: Extract<PrintNode, { type: "barcode" }>, contentWidth: number): Block {
  const bits = generateBarcodeBits(node.symbology, node.data);
  if (bits === null) {
    // data invalid for this symbology -- degrade gracefully rather than throw
    // (see docs/DOCUMENT_FORMAT.md's renderer contract).
    return placeholderBarcodeBlock(node.symbology, node.data, contentWidth);
  }
  const barHeight = Math.min(node.heightPx ?? 60, LINE_HEIGHT * 4);
  const moduleWidth = contentWidth / bits.length;
  const rects: string[] = [];
  for (let i = 0; i < bits.length; i++) {
    if (bits[i] === "1") rects.push(`<rect x="${i * moduleWidth}" y="0" width="${moduleWidth}" height="${barHeight}"/>`);
  }
  const showText = node.showText !== false;
  const textSvg = showText
    ? `<text x="${contentWidth / 2}" y="${barHeight + FONT_SIZE}" text-anchor="middle" font-size="${FONT_SIZE}" font-family="monospace">${escapeXml(node.data)}</text>`
    : "";
  return { height: barHeight + (showText ? FONT_SIZE + 4 : 0), svg: `<g fill="#000">${rects.join("")}</g>${textSvg}` };
}

function blockFor(node: PrintNode, contentWidth: number): Block | null {
  switch (node.type) {
    case "text":
      return textBlock(node.value, contentWidth, node.align, {
        bold: node.bold,
        underline: node.underline,
        sizeH: node.size?.h,
      });
    case "row":
      return rowBlock(node, contentWidth);
    case "rule":
      return { height: LINE_HEIGHT * 0.5, svg: `<line x1="0" y1="${LINE_HEIGHT * 0.25}" x2="${contentWidth}" y2="${LINE_HEIGHT * 0.25}" stroke="#000"/>` };
    case "feed":
      return { height: LINE_HEIGHT * (node.lines ?? 1), svg: "" };
    case "image": {
      const height = 80;
      return {
        height,
        svg: `<image href="data:image/png;base64,${node.data}" width="${contentWidth}" height="${height}" preserveAspectRatio="xMidYMid meet"/>`,
      };
    }
    case "barcode":
      return barcodeBlock(node, contentWidth);
    case "qrcode":
      return qrBlock(node.data, node.ec ?? "M", contentWidth);
    case "cut":
    case "drawer":
    case "raw":
      return null; // hardware-only actions; nothing to preview
  }
}

export function toSvg(doc: PrintDocument): string {
  const widthMm = resolvePageWidthMm(doc.paper);
  const marginMm = resolveMarginMm(doc.paper);
  const widthPx = widthMm * PX_PER_MM;
  const marginPx = marginMm * PX_PER_MM;
  const contentWidth = widthPx - marginPx * 2;

  let cursorY = marginPx;
  const groups: string[] = [];
  for (const node of doc.content) {
    const block = blockFor(node, contentWidth);
    if (block === null) continue;
    if (block.svg) groups.push(`<g transform="translate(${marginPx},${cursorY})">${block.svg}</g>`);
    cursorY += block.height;
  }
  const heightPx = cursorY + marginPx;

  return `<svg width="${widthPx}" height="${heightPx}" viewBox="0 0 ${widthPx} ${heightPx}" xmlns="http://www.w3.org/2000/svg" font-family="Arial, sans-serif">` +
    `<rect width="${widthPx}" height="${heightPx}" fill="#fff"/>` +
    groups.join("") +
    `</svg>`;
}
