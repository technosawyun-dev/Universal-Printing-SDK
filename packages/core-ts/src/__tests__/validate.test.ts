import { describe, expect, it } from "vitest";
import { InvalidDocumentError, validateDocument } from "../validate.js";

describe("validateDocument", () => {
  it("accepts a minimal valid document", () => {
    const doc = validateDocument({
      version: "1.0",
      paper: { kind: "thermal-80" },
      content: [{ type: "text", value: "hello" }],
    });
    expect(doc.paper.kind).toBe("thermal-80");
  });

  it("rejects an unknown node type", () => {
    expect(() =>
      validateDocument({
        version: "1.0",
        paper: { kind: "thermal-80" },
        content: [{ type: "not-a-real-type" }],
      }),
    ).toThrow(InvalidDocumentError);
  });

  it("requires widthMm when paper.kind is custom", () => {
    expect(() =>
      validateDocument({
        version: "1.0",
        paper: { kind: "custom" },
        content: [],
      }),
    ).toThrow(InvalidDocumentError);
  });

  it("accepts custom paper with widthMm", () => {
    const doc = validateDocument({
      version: "1.0",
      paper: { kind: "custom", widthMm: 120 },
      content: [],
    });
    expect(doc.paper.widthMm).toBe(120);
  });

  it("rejects a row with zero columns", () => {
    expect(() =>
      validateDocument({
        version: "1.0",
        paper: { kind: "thermal-58" },
        content: [{ type: "row", columns: [] }],
      }),
    ).toThrow(InvalidDocumentError);
  });
});
