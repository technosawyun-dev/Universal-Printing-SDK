// PrintDocument -> plain text, for debug / non-graphical fallback.
// Per docs/DOCUMENT_FORMAT.md's renderer contract, node types this renderer
// can't represent (image/barcode/qrcode) degrade to a placeholder line and
// log a warning rather than throwing.

import type { PrintDocument } from "../types.js";
import { resolveCharsPerLine } from "../types.js";
import { layoutRow, wrapAndAlign } from "./layout.js";

export function toPlainText(doc: PrintDocument): string {
  const charsPerLine = resolveCharsPerLine(doc.paper);
  const lines: string[] = [];
  for (const node of doc.content) {
    switch (node.type) {
      case "text":
        lines.push(...wrapAndAlign(node.value, charsPerLine, node.align ?? "left"));
        break;
      case "row":
        lines.push(layoutRow(node.columns, charsPerLine));
        break;
      case "rule":
        lines.push((node.char ?? "-").repeat(charsPerLine));
        break;
      case "feed":
        for (let i = 0; i < (node.lines ?? 1); i++) lines.push("");
        break;
      case "image":
        console.warn("toPlainText: 'image' node has no text representation, printing a placeholder");
        lines.push("[image]");
        break;
      case "barcode":
        console.warn(`toPlainText: 'barcode' node (${node.symbology}) has no text representation, printing a placeholder`);
        lines.push(`[barcode: ${node.symbology} ${node.data}]`);
        break;
      case "qrcode":
        console.warn("toPlainText: 'qrcode' node has no text representation, printing a placeholder");
        lines.push(`[qrcode: ${node.data}]`);
        break;
      case "cut":
      case "drawer":
      case "raw":
        break; // hardware-only actions; no-op for plain text
    }
  }
  return lines.join("\n");
}
