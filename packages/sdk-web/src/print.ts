// window.print() fallback -- for a normal (a4/a3/letter) printer with no
// agent and no direct transport, via the OS's own print dialog. See
// docs/ARCHITECTURE.md: sdk-web's last-resort path.

import type { PrintDocument } from "@printers/core-ts";
import { toHtml } from "@printers/core-ts";

/** Opens the rendered document in a new tab/window for preview, without printing. Returns null if the popup was blocked. */
export function openPrintPreview(document: PrintDocument): Window | null {
  const html = toHtml(document);
  const win = window.open("", "_blank");
  if (!win) return null;
  win.document.open();
  win.document.write(html);
  win.document.close();
  return win;
}

/**
 * Opens the document and triggers the browser's print dialog. Must be called
 * from a user gesture (e.g. a click handler) -- otherwise the popup blocker
 * will silently prevent the window from opening.
 */
export function printViaWindowPrint(document: PrintDocument): void {
  const win = openPrintPreview(document);
  if (!win) {
    throw new Error("window.open was blocked -- printViaWindowPrint must be called from a user gesture (e.g. a click handler)");
  }
  win.focus();
  win.print();
}
