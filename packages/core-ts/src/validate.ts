import { z } from "zod";
import type { PrintDocument } from "./types.js";

const align = z.enum(["left", "center", "right"]);

const textNode = z.object({
  type: z.literal("text"),
  value: z.string(),
  align: align.optional(),
  bold: z.boolean().optional(),
  underline: z.boolean().optional(),
  invert: z.boolean().optional(),
  size: z.object({ w: z.number().int().min(1).max(8), h: z.number().int().min(1).max(8) }).optional(),
  font: z.enum(["a", "b"]).optional(),
});

const rowColumn = z.object({
  text: z.string(),
  width: z.number().positive(),
  align: align.optional(),
});

const rowNode = z.object({
  type: z.literal("row"),
  columns: z.array(rowColumn).min(1),
  bold: z.boolean().optional(),
});

const ruleNode = z.object({
  type: z.literal("rule"),
  char: z.string().length(1).optional(),
});

const feedNode = z.object({
  type: z.literal("feed"),
  lines: z.number().int().min(1).optional(),
});

const imageNode = z.object({
  type: z.literal("image"),
  data: z.string().min(1),
  widthPx: z.number().positive().optional(),
  align: align.optional(),
});

const barcodeNode = z.object({
  type: z.literal("barcode"),
  symbology: z.enum(["code128", "code39", "ean13", "upca"]),
  data: z.string().min(1),
  heightPx: z.number().positive().optional(),
  showText: z.boolean().optional(),
});

const qrcodeNode = z.object({
  type: z.literal("qrcode"),
  data: z.string().min(1),
  size: z.number().int().min(1).max(16).optional(),
  ec: z.enum(["L", "M", "Q", "H"]).optional(),
});

const cutNode = z.object({
  type: z.literal("cut"),
  mode: z.enum(["full", "partial"]).optional(),
});

const drawerNode = z.object({
  type: z.literal("drawer"),
  pin: z.union([z.literal(2), z.literal(5)]).optional(),
});

const rawNode = z.object({
  type: z.literal("raw"),
  escposBase64: z.string().min(1),
});

const printNode = z.discriminatedUnion("type", [
  textNode,
  rowNode,
  ruleNode,
  feedNode,
  imageNode,
  barcodeNode,
  qrcodeNode,
  cutNode,
  drawerNode,
  rawNode,
]);

const paper = z
  .object({
    kind: z.enum(["thermal-58", "thermal-80", "a4", "a3", "letter", "custom"]),
    widthMm: z.number().positive().optional(),
    marginMm: z.number().min(0).optional(),
    charsPerLine: z.number().int().positive().optional(),
  })
  .superRefine((p, ctx) => {
    if (p.kind === "custom" && p.widthMm == null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "paper.widthMm is required when paper.kind is 'custom'",
        path: ["widthMm"],
      });
    }
  });

const printDocumentSchema = z.object({
  version: z.literal("1.0"),
  paper,
  meta: z
    .object({
      title: z.string().optional(),
      encoding: z.string().optional(),
      locale: z.string().optional(),
    })
    .optional(),
  content: z.array(printNode),
});

export class InvalidDocumentError extends Error {
  constructor(message: string, public issues: z.ZodIssue[]) {
    super(message);
    this.name = "InvalidDocumentError";
  }
}

/** Throws InvalidDocumentError on failure; returns a type-narrowed PrintDocument on success. */
export function validateDocument(input: unknown): PrintDocument {
  const result = printDocumentSchema.safeParse(input);
  if (!result.success) {
    throw new InvalidDocumentError(
      `invalid PrintDocument: ${result.error.issues.map((i) => `${i.path.join(".")}: ${i.message}`).join("; ")}`,
      result.error.issues,
    );
  }
  return result.data as PrintDocument;
}
