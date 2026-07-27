// PrintDocument model — mirrors docs/DOCUMENT_FORMAT.md exactly.
// crates/core-rs must accept/produce the identical JSON shape.

export type PaperKind = "thermal-58" | "thermal-80" | "a4" | "a3" | "letter" | "custom";

export interface Paper {
  kind: PaperKind;
  /** required only when kind === "custom" */
  widthMm?: number;
  /** default 0 for thermal, 10 for a4/a3/letter */
  marginMm?: number;
  /** override; defaults: thermal-58 -> 32, thermal-80 -> 48 */
  charsPerLine?: number;
}

export interface DocMeta {
  title?: string;
  /** ESC/POS codepage hint, default cp437 */
  encoding?: string;
  locale?: string;
}

export type Align = "left" | "center" | "right";

export interface TextNode {
  type: "text";
  value: string;
  align?: Align;
  bold?: boolean;
  underline?: boolean;
  invert?: boolean;
  size?: { w: number; h: number };
  font?: "a" | "b";
}

export interface RowColumn {
  text: string;
  width: number;
  align?: Align;
}

export interface RowNode {
  type: "row";
  columns: RowColumn[];
  bold?: boolean;
}

export interface RuleNode {
  type: "rule";
  char?: string;
}

export interface FeedNode {
  type: "feed";
  lines?: number;
}

export interface ImageNode {
  type: "image";
  /** base64 PNG */
  data: string;
  widthPx?: number;
  align?: Align;
}

export type BarcodeSymbology = "code128" | "code39" | "ean13" | "upca";

export interface BarcodeNode {
  type: "barcode";
  symbology: BarcodeSymbology;
  data: string;
  heightPx?: number;
  showText?: boolean;
}

export interface QrCodeNode {
  type: "qrcode";
  data: string;
  size?: number;
  ec?: "L" | "M" | "Q" | "H";
}

export interface CutNode {
  type: "cut";
  mode?: "full" | "partial";
}

export interface DrawerNode {
  type: "drawer";
  pin?: 2 | 5;
}

export interface RawNode {
  type: "raw";
  escposBase64: string;
}

export type PrintNode =
  | TextNode
  | RowNode
  | RuleNode
  | FeedNode
  | ImageNode
  | BarcodeNode
  | QrCodeNode
  | CutNode
  | DrawerNode
  | RawNode;

export interface PrintDocument {
  version: "1.0";
  paper: Paper;
  meta?: DocMeta;
  content: PrintNode[];
}

/** thermal-58 -> 32, thermal-80 -> 48, per docs/DOCUMENT_FORMAT.md compatibility table */
export function resolveCharsPerLine(paper: Paper): number {
  if (paper.charsPerLine != null) return paper.charsPerLine;
  if (paper.kind === "thermal-58") return 32;
  if (paper.kind === "thermal-80") return 48;
  // a4/a3/letter/custom: renderers that need a char grid (plain text) fall back to 80
  return 80;
}

export function resolveMarginMm(paper: Paper): number {
  if (paper.marginMm != null) return paper.marginMm;
  return paper.kind === "thermal-58" || paper.kind === "thermal-80" ? 0 : 10;
}

/** width in mm for page-geometry renderers (html/svg/pdf) */
export function resolvePageWidthMm(paper: Paper): number {
  switch (paper.kind) {
    case "thermal-58":
      return 58;
    case "thermal-80":
      return 80;
    case "a4":
      return 210;
    case "a3":
      return 297;
    case "letter":
      return 215.9;
    case "custom":
      if (paper.widthMm == null) {
        throw new Error("paper.widthMm is required when paper.kind is 'custom'");
      }
      return paper.widthMm;
  }
}

export function isThermal(paper: Paper): boolean {
  return paper.kind === "thermal-58" || paper.kind === "thermal-80";
}

/** page height in mm for normal (non-thermal) paper; thermal is unbounded (roll), returns null */
export function resolvePageHeightMm(paper: Paper): number | null {
  switch (paper.kind) {
    case "a4":
      return 297;
    case "a3":
      return 420;
    case "letter":
      return 279.4;
    default:
      return null;
  }
}
