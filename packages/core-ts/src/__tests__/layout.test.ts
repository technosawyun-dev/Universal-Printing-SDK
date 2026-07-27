import { describe, expect, it } from "vitest";
import { layoutRow, wrapAndAlign, wrapText } from "../render/layout.js";

describe("layoutRow", () => {
  it("splits columns proportionally to their width units", () => {
    const line = layoutRow(
      [
        { text: "Latte", width: 3, align: "left" },
        { text: "x2", width: 1, align: "center" },
        { text: "6,000", width: 2, align: "right" },
      ],
      32,
    );
    expect(line.length).toBe(32);
    expect(line.startsWith("Latte")).toBe(true);
    expect(line.trimEnd().endsWith("6,000")).toBe(true);
  });

  it("truncates cell text that overflows its column width", () => {
    const line = layoutRow([{ text: "A very long item name", width: 1, align: "left" }], 8);
    expect(line).toBe("A very l");
  });
});

describe("wrapText", () => {
  it("wraps on word boundaries without exceeding width", () => {
    const lines = wrapText("the quick brown fox jumps", 10);
    for (const line of lines) expect(line.length).toBeLessThanOrEqual(10);
    expect(lines.join(" ").replace(/\s+/g, " ")).toContain("quick");
  });

  it("hard-breaks a single word longer than the width", () => {
    const lines = wrapText("supercalifragilistic", 5);
    expect(lines.every((l) => l.length <= 5)).toBe(true);
  });

  it("preserves explicit newlines as paragraph breaks", () => {
    const lines = wrapText("first\nsecond", 20);
    expect(lines).toEqual(["first", "second"]);
  });
});

describe("wrapAndAlign", () => {
  it("pads every wrapped line to the target width", () => {
    const lines = wrapAndAlign("hi", 10, "center");
    expect(lines[0].length).toBe(10);
  });
});
