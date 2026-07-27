import { describe, expect, it } from "vitest";
import {
  isThermal,
  resolveCharsPerLine,
  resolveMarginMm,
  resolvePageHeightMm,
  resolvePageWidthMm,
} from "../types.js";

describe("paper resolution helpers", () => {
  it("defaults charsPerLine per paper kind", () => {
    expect(resolveCharsPerLine({ kind: "thermal-58" })).toBe(32);
    expect(resolveCharsPerLine({ kind: "thermal-80" })).toBe(48);
    expect(resolveCharsPerLine({ kind: "a4" })).toBe(80);
  });

  it("lets an explicit charsPerLine override the default", () => {
    expect(resolveCharsPerLine({ kind: "thermal-80", charsPerLine: 42 })).toBe(42);
  });

  it("defaults margin to 0 for thermal and 10 for page paper", () => {
    expect(resolveMarginMm({ kind: "thermal-58" })).toBe(0);
    expect(resolveMarginMm({ kind: "a4" })).toBe(10);
    expect(resolveMarginMm({ kind: "a4", marginMm: 5 })).toBe(5);
  });

  it("resolves page width per kind, and requires widthMm for custom", () => {
    expect(resolvePageWidthMm({ kind: "thermal-58" })).toBe(58);
    expect(resolvePageWidthMm({ kind: "a4" })).toBe(210);
    expect(resolvePageWidthMm({ kind: "custom", widthMm: 100 })).toBe(100);
    expect(() => resolvePageWidthMm({ kind: "custom" })).toThrow();
  });

  it("resolves page height only for bounded page paper, null for rolls", () => {
    expect(resolvePageHeightMm({ kind: "a4" })).toBe(297);
    expect(resolvePageHeightMm({ kind: "thermal-80" })).toBeNull();
  });

  it("identifies thermal paper kinds", () => {
    expect(isThermal({ kind: "thermal-58" })).toBe(true);
    expect(isThermal({ kind: "thermal-80" })).toBe(true);
    expect(isThermal({ kind: "a4" })).toBe(false);
  });
});
