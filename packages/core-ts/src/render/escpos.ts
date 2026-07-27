// PrintDocument -> ESC/POS bytes, for thermal-58/thermal-80 printers.
//
// Command bytes are verified against escpos-php's Printer.php (MIT, see
// "Opensource Printers printing/escpos-php") — GS k barcode, GS ( k QR code,
// GS v 0 raster image, GS V cut, ESC p drawer pulse all mirror its output so
// this renders identically to a well-established, widely-deployed library.
// crates/core-rs and packages/sdk_flutter's escpos_builder.dart must agree
// byte-for-byte with this file for the same input document.

import type {
  Align,
  BarcodeNode,
  CutNode,
  DrawerNode,
  FeedNode,
  ImageNode,
  Paper,
  PrintDocument,
  QrCodeNode,
  RawNode,
  RowNode,
  RuleNode,
  TextNode,
} from "../types.js";
import { resolveCharsPerLine } from "../types.js";
import { bandRaster, base64ToBytes, pngToRaster, THERMAL_58_DOTS, THERMAL_80_DOTS } from "./imaging.js";
import { layoutRow } from "./layout.js";

const ESC = 0x1b;
const GS = 0x1d;

class ByteWriter {
  private chunks: number[] = [];

  push(...bytes: number[]): this {
    for (const b of bytes) this.chunks.push(b);
    return this;
  }

  pushBytes(bytes: ArrayLike<number>): this {
    for (let i = 0; i < bytes.length; i++) this.chunks.push(bytes[i]);
    return this;
  }

  toBytes(): Uint8Array {
    return Uint8Array.from(this.chunks);
  }
}

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Math.round(n)));
}

function alignCode(align: Align): number {
  return align === "center" ? 1 : align === "right" ? 2 : 0;
}

/**
 * Best-effort passthrough for the common Latin-1/CP437 overlap; anything
 * above U+00FF becomes '?'. Full codepage switching (CJK, Cyrillic, etc.) is
 * explicitly deferred for v1 — see the escpos-php codepage-database gotcha
 * documented in project memory / docs/ARCHITECTURE.md.
 */
function textToBytes(value: string): number[] {
  const bytes: number[] = [];
  for (const ch of value) {
    const code = ch.codePointAt(0)!;
    bytes.push(code <= 0xff ? code : 0x3f);
  }
  return bytes;
}

function writeText(w: ByteWriter, node: TextNode): void {
  const sizeW = clamp((node.size?.w ?? 1) - 1, 0, 7);
  const sizeH = clamp((node.size?.h ?? 1) - 1, 0, 7);
  w.push(ESC, 0x61, alignCode(node.align ?? "left"));
  w.push(ESC, 0x45, node.bold ? 1 : 0);
  w.push(ESC, 0x2d, node.underline ? 1 : 0);
  w.push(GS, 0x42, node.invert ? 1 : 0);
  w.push(GS, 0x21, (sizeW << 4) | sizeH);
  w.push(ESC, 0x4d, node.font === "b" ? 1 : 0);
  w.pushBytes(textToBytes(node.value));
  w.push(0x0a);
  // Reset to defaults so this node's formatting doesn't leak into the next one.
  w.push(ESC, 0x61, 0, ESC, 0x45, 0, ESC, 0x2d, 0, GS, 0x42, 0, GS, 0x21, 0, ESC, 0x4d, 0);
}

function writeRow(w: ByteWriter, node: RowNode, charsPerLine: number): void {
  w.push(ESC, 0x45, node.bold ? 1 : 0);
  w.pushBytes(textToBytes(layoutRow(node.columns, charsPerLine)));
  w.push(0x0a);
  w.push(ESC, 0x45, 0);
}

function writeRule(w: ByteWriter, node: RuleNode, charsPerLine: number): void {
  w.pushBytes(textToBytes((node.char ?? "-").repeat(charsPerLine)));
  w.push(0x0a);
}

function writeFeed(w: ByteWriter, node: FeedNode): void {
  const lines = node.lines ?? 1;
  if (lines <= 1) w.push(0x0a);
  else w.push(ESC, 0x64, clamp(lines, 1, 255));
}

function writeImage(w: ByteWriter, node: ImageNode, paper: Paper): void {
  const maxDots = paper.kind === "thermal-58" ? THERMAL_58_DOTS : THERMAL_80_DOTS;
  const targetWidth = node.widthPx != null ? Math.min(node.widthPx, maxDots) : maxDots;
  const raster = pngToRaster(node.data, targetWidth);
  const align = node.align ?? "center";
  w.push(ESC, 0x61, alignCode(align));
  for (const band of bandRaster(raster)) {
    w.push(
      GS,
      0x76,
      0x30,
      0x00,
      raster.bytesPerRow & 0xff,
      (raster.bytesPerRow >> 8) & 0xff,
      band.rows & 0xff,
      (band.rows >> 8) & 0xff,
    );
    w.pushBytes(band.bits);
  }
  w.push(ESC, 0x61, 0);
}

const BARCODE_TYPE: Record<BarcodeNode["symbology"], number> = {
  upca: 65,
  ean13: 67,
  code39: 69,
  code128: 73,
};

function writeBarcode(w: ByteWriter, node: BarcodeNode): void {
  const heightPx = clamp(node.heightPx ?? 60, 1, 255);
  w.push(GS, 0x68, heightPx); // GS h n -- barcode height
  w.push(GS, 0x77, 3); // GS w n -- module width
  w.push(GS, 0x48, node.showText === false ? 0 : 2); // GS H n -- HRI text below (default) or none
  const data = node.symbology === "code128" ? `{B${node.data}` : node.data;
  const bytes = textToBytes(data);
  w.push(GS, 0x6b, BARCODE_TYPE[node.symbology], bytes.length);
  w.pushBytes(bytes);
}

const QR_EC_LEVEL: Record<NonNullable<QrCodeNode["ec"]>, number> = { L: 48, M: 49, Q: 50, H: 51 };

function writeQrCode(w: ByteWriter, node: QrCodeNode): void {
  const size = clamp(node.size ?? 6, 1, 16);
  const ecByte = QR_EC_LEVEL[node.ec ?? "M"];
  const data = new TextEncoder().encode(node.data);
  w.push(GS, 0x28, 0x6b, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00); // select model 2
  w.push(GS, 0x28, 0x6b, 0x03, 0x00, 0x31, 0x43, size); // module size
  w.push(GS, 0x28, 0x6b, 0x03, 0x00, 0x31, 0x45, ecByte); // error correction level
  const storeLen = data.length + 3;
  w.push(GS, 0x28, 0x6b, storeLen & 0xff, (storeLen >> 8) & 0xff, 0x31, 0x50, 0x30); // store data
  w.pushBytes(data);
  w.push(GS, 0x28, 0x6b, 0x03, 0x00, 0x31, 0x51, 0x30); // print
}

function writeCut(w: ByteWriter, node: CutNode): void {
  w.push(GS, 0x56, node.mode === "full" ? 0 : 1);
}

function writeDrawer(w: ByteWriter, node: DrawerNode): void {
  w.push(ESC, 0x70, node.pin === 5 ? 1 : 0, 25, 250);
}

function writeRaw(w: ByteWriter, node: RawNode): void {
  w.pushBytes(base64ToBytes(node.escposBase64));
}

export function toEscPos(doc: PrintDocument): Uint8Array {
  const w = new ByteWriter();
  const charsPerLine = resolveCharsPerLine(doc.paper);
  w.push(ESC, 0x40); // initialize
  for (const node of doc.content) {
    switch (node.type) {
      case "text":
        writeText(w, node);
        break;
      case "row":
        writeRow(w, node, charsPerLine);
        break;
      case "rule":
        writeRule(w, node, charsPerLine);
        break;
      case "feed":
        writeFeed(w, node);
        break;
      case "image":
        writeImage(w, node, doc.paper);
        break;
      case "barcode":
        writeBarcode(w, node);
        break;
      case "qrcode":
        writeQrCode(w, node);
        break;
      case "cut":
        writeCut(w, node);
        break;
      case "drawer":
        writeDrawer(w, node);
        break;
      case "raw":
        writeRaw(w, node);
        break;
    }
  }
  return w.toBytes();
}
