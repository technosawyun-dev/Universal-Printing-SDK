import { describe, expect, it, vi } from "vitest";
import type { PrintDocument } from "@printers/core-ts";
import { printDocumentNative } from "../native-print.js";

const DOC: PrintDocument = { version: "1.0", paper: { kind: "a4" }, content: [{ type: "text", value: "hi" }] };

function fakeBrowserWindowCtor(printImpl: (opts: unknown, cb: (success: boolean, errorType: string) => void) => void) {
  const loadURL = vi.fn().mockResolvedValue(undefined);
  const close = vi.fn();
  const isDestroyed = vi.fn().mockReturnValue(false);
  const ctor = vi.fn().mockImplementation(() => ({
    loadURL,
    webContents: { print: vi.fn().mockImplementation(printImpl) },
    close,
    isDestroyed,
  }));
  return { ctor, loadURL, close, isDestroyed };
}

describe("printDocumentNative", () => {
  it("loads the rendered HTML as a data URL and prints silently by default", async () => {
    const { ctor, loadURL, close } = fakeBrowserWindowCtor((_opts, cb) => cb(true, ""));
    await printDocumentNative(ctor as never, DOC);

    expect(loadURL).toHaveBeenCalledWith(expect.stringContaining("data:text/html;charset=utf-8,"));
    const instance = ctor.mock.results[0]!.value as { webContents: { print: ReturnType<typeof vi.fn> } };
    expect(instance.webContents.print).toHaveBeenCalledWith(expect.objectContaining({ silent: true }), expect.any(Function));
    expect(close).toHaveBeenCalled();
  });

  it("passes deviceName and copies through when provided", async () => {
    const { ctor } = fakeBrowserWindowCtor((_opts, cb) => cb(true, ""));
    await printDocumentNative(ctor as never, DOC, { deviceName: "EPSON TM-T88V", copies: 2, silent: false });

    const instance = ctor.mock.results[0]!.value as { webContents: { print: ReturnType<typeof vi.fn> } };
    expect(instance.webContents.print).toHaveBeenCalledWith(
      expect.objectContaining({ silent: false, deviceName: "EPSON TM-T88V", copies: 2 }),
      expect.any(Function),
    );
  });

  it("rejects with the driver's errorType when printing fails, and still closes the window", async () => {
    const { ctor, close } = fakeBrowserWindowCtor((_opts, cb) => cb(false, "printing_failed"));
    await expect(printDocumentNative(ctor as never, DOC)).rejects.toThrow("printing_failed");
    expect(close).toHaveBeenCalled();
  });

  it("does not double-close an already-destroyed window", async () => {
    const { ctor, close, isDestroyed } = fakeBrowserWindowCtor((_opts, cb) => cb(true, ""));
    isDestroyed.mockReturnValue(true);
    await printDocumentNative(ctor as never, DOC);
    expect(close).not.toHaveBeenCalled();
  });
});
