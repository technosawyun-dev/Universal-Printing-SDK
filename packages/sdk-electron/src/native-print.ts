// Native silent print for a4/a3/letter via Electron's own Chromium print
// pipeline -- no print-agent, no shelling out to CUPS/the Windows spooler
// needed for this path (unlike crates/print-agent's /print-silent). See
// docs/ARCHITECTURE.md.
//
// Unverified end-to-end in this environment: launching a real Electron
// BrowserWindow needs a display/GUI runtime this sandbox doesn't have. The
// BrowserWindow constructor is injected specifically so the wiring logic
// here can still be unit-tested against a fake window shape -- see
// src/__tests__/native-print.test.ts.

import type { PrintDocument } from "@printers/core-ts";
import { toHtml } from "@printers/core-ts";
import type { BrowserWindow, BrowserWindowConstructorOptions, WebContentsPrintOptions } from "electron";

export interface NativePrintOptions {
  deviceName?: string;
  copies?: number;
  /** Default true -- set false only for interactive debugging, this path exists specifically to avoid a print dialog. */
  silent?: boolean;
  windowOptions?: BrowserWindowConstructorOptions;
}

type BrowserWindowCtor = new (options?: BrowserWindowConstructorOptions) => BrowserWindow;

/**
 * Renders `document` to HTML in a hidden `BrowserWindow` and prints it via
 * `webContents.print()`. The window is always closed afterward, whether the
 * print succeeded or failed.
 */
export function printDocumentNative(BrowserWindowCtor: BrowserWindowCtor, document: PrintDocument, options: NativePrintOptions = {}): Promise<void> {
  const html = toHtml(document);
  const win = new BrowserWindowCtor({ show: false, ...options.windowOptions });

  const printOptions: WebContentsPrintOptions = {
    silent: options.silent ?? true,
    ...(options.deviceName != null ? { deviceName: options.deviceName } : {}),
    ...(options.copies != null ? { copies: options.copies } : {}),
  };

  return win
    .loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`)
    .then(
      () =>
        new Promise<void>((resolve, reject) => {
          win.webContents.print(printOptions, (success, errorType) => {
            if (success) resolve();
            else reject(new Error(errorType || "print failed"));
          });
        }),
    )
    .finally(() => {
      if (!win.isDestroyed()) win.close();
    });
}
