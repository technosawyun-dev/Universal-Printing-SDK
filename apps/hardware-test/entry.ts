// Bundled entry point for the standalone hardware-test page (see
// apps/hardware-test/index.html). Exposes everything the page's inline
// script needs on `window.PrintersSDK`, bundled into one dependency-free
// file via esbuild so the page works from a plain double-clicked file://
// URL -- no dev server, no Node.js, no build tools needed on the test
// machine.

import { toEscPos, toPlainText, validateDocument } from "@printers/core-ts";
import type { PrintDocument } from "@printers/core-ts";
import {
  AgentClient,
  listPairedWebUsbPrinters,
  printViaWebBluetooth,
  printViaWebUsb,
  requestWebBluetoothPrinter,
  requestWebUsbPrinter,
} from "@printers/sdk-web";

(globalThis as any).PrintersSDK = {
  toEscPos,
  toPlainText,
  validateDocument,
  AgentClient,
  requestWebUsbPrinter,
  listPairedWebUsbPrinters,
  printViaWebUsb,
  requestWebBluetoothPrinter,
  printViaWebBluetooth,
};

export type { PrintDocument };
