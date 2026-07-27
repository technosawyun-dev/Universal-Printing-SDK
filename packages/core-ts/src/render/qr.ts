// Shared QR-matrix helper for html.ts and svg.ts previews. escpos.ts does
// NOT use this — thermal printers generate the QR code on-device via the
// `GS ( k` command sequence (see escpos.ts), this is only for on-screen
// preview where there's no printer to do it for us.

import { create } from "qrcode";
import type { QrCodeNode } from "../types.js";

export interface QrMatrix {
  size: number;
  isDark(row: number, col: number): boolean;
}

export function qrModules(data: string, ec: NonNullable<QrCodeNode["ec"]> = "M"): QrMatrix {
  const qr = create(data, { errorCorrectionLevel: ec });
  return {
    size: qr.modules.size,
    isDark: (row: number, col: number) => qr.modules.get(row, col) === 1,
  };
}
