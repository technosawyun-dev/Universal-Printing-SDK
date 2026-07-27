import { afterEach, describe, expect, it, vi } from "vitest";
import type { PrintDocument } from "@printers/core-ts";
import { openPrintPreview, printViaWindowPrint } from "../print.js";

const DOC: PrintDocument = { version: "1.0", paper: { kind: "a4" }, content: [{ type: "text", value: "hi" }] };

function fakeWindow() {
  return {
    document: { open: vi.fn(), write: vi.fn(), close: vi.fn() },
    focus: vi.fn(),
    print: vi.fn(),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("openPrintPreview", () => {
  it("writes the rendered HTML into a newly opened window", () => {
    const popup = fakeWindow();
    const windowOpen = vi.fn().mockReturnValue(popup);
    vi.stubGlobal("window", { open: windowOpen });

    const result = openPrintPreview(DOC);
    expect(result).toBe(popup);
    expect(popup.document.open).toHaveBeenCalled();
    expect(popup.document.write).toHaveBeenCalledWith(expect.stringContaining("hi"));
    expect(popup.document.close).toHaveBeenCalled();
  });

  it("returns null when the popup is blocked", () => {
    vi.stubGlobal("window", { open: vi.fn().mockReturnValue(null) });
    expect(openPrintPreview(DOC)).toBeNull();
  });
});

describe("printViaWindowPrint", () => {
  it("focuses and calls print() on the opened window", () => {
    const popup = fakeWindow();
    vi.stubGlobal("window", { open: vi.fn().mockReturnValue(popup) });

    printViaWindowPrint(DOC);
    expect(popup.focus).toHaveBeenCalled();
    expect(popup.print).toHaveBeenCalled();
  });

  it("throws a clear error when the popup is blocked", () => {
    vi.stubGlobal("window", { open: vi.fn().mockReturnValue(null) });
    expect(() => printViaWindowPrint(DOC)).toThrow(/user gesture/);
  });
});
