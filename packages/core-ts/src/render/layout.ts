// Shared char-grid layout for renderers that print into a fixed column width
// (escpos.ts, text.ts). html.ts/svg.ts lay `row` out with real fractional
// widths (flex/proportional) instead, since they aren't grid-constrained.

import type { Align, RowColumn } from "../types.js";

/** Largest-remainder allocation of fractional `width` units into integer chars. */
function allocateColumnWidths(columns: RowColumn[], totalChars: number): number[] {
  const totalUnits = columns.reduce((sum, c) => sum + c.width, 0);
  const raw = columns.map((c) => (c.width / totalUnits) * totalChars);
  const floors = raw.map(Math.floor);
  let remaining = totalChars - floors.reduce((a, b) => a + b, 0);
  const order = raw
    .map((v, i) => ({ i, frac: v - floors[i] }))
    .sort((a, b) => b.frac - a.frac);
  const widths = [...floors];
  for (let k = 0; k < order.length && remaining > 0; k++, remaining--) {
    widths[order[k].i]++;
  }
  return widths.map((w) => Math.max(1, w));
}

function padAlign(text: string, width: number, align: Align = "left"): string {
  const truncated = text.length > width ? text.slice(0, width) : text;
  const gap = width - truncated.length;
  if (gap <= 0) return truncated;
  if (align === "right") return " ".repeat(gap) + truncated;
  if (align === "center") {
    const left = Math.floor(gap / 2);
    return " ".repeat(left) + truncated + " ".repeat(gap - left);
  }
  return truncated + " ".repeat(gap);
}

/**
 * Lays out a `row` node's columns into one fixed-width line of `totalChars`.
 * Overflowing cell text is truncated rather than wrapped to a second line —
 * matches standard receipt-table behavior (node-thermal-printer, escpos-php)
 * rather than the added complexity of syncing multi-line cell heights.
 */
export function layoutRow(columns: RowColumn[], totalChars: number): string {
  const widths = allocateColumnWidths(columns, totalChars);
  return columns.map((c, i) => padAlign(c.text, widths[i], c.align)).join("");
}

/** Greedy word-wrap; a single word longer than `width` is hard-broken. */
export function wrapText(value: string, width: number): string[] {
  const lines: string[] = [];
  for (const paragraph of value.split("\n")) {
    if (paragraph.length === 0) {
      lines.push("");
      continue;
    }
    let line = "";
    for (const word of paragraph.split(" ")) {
      let remaining = word;
      while (remaining.length > width) {
        if (line.length > 0) {
          lines.push(line);
          line = "";
        }
        lines.push(remaining.slice(0, width));
        remaining = remaining.slice(width);
      }
      const candidate = line.length === 0 ? remaining : `${line} ${remaining}`;
      if (candidate.length > width) {
        lines.push(line);
        line = remaining;
      } else {
        line = candidate;
      }
    }
    lines.push(line);
  }
  return lines;
}

/**
 * Wraps `value` to `totalChars` and aligns each resulting line. Left-aligned
 * lines are returned unpadded (nothing to align against on a standalone
 * line) -- padding only matters for center/right.
 */
export function wrapAndAlign(value: string, totalChars: number, align: Align = "left"): string[] {
  const lines = wrapText(value, totalChars);
  if (align === "left") return lines;
  return lines.map((line) => padAlign(line, totalChars, align));
}
