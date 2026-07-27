// Browser demo exercising packages/core-ts directly: edit a PrintDocument as
// JSON, see it validated live, and preview it through all four renderers.
// This is the apps/demo referenced in docs/ARCHITECTURE.md -- it does not
// talk to a print-agent or a real printer; window.print() is the only
// "print" path here (see packages/sdk-web for the real transports).

import { InvalidDocumentError, toEscPos, toHtml, toPlainText, toSvg, validateDocument } from "@printers/core-ts";
import type { PrintDocument } from "@printers/core-ts";

const SAMPLE_DOC: PrintDocument = {
  version: "1.0",
  paper: { kind: "thermal-80" },
  meta: { title: "Sample receipt" },
  content: [
    { type: "text", value: "PRINTERS SDK", align: "center", bold: true, size: { w: 2, h: 2 } },
    { type: "text", value: "123 Demo Street, Yangon", align: "center" },
    { type: "rule" },
    { type: "row", columns: [{ text: "Item", width: 3, align: "left" }, { text: "Qty", width: 1, align: "center" }, { text: "Total", width: 2, align: "right" }] },
    { type: "row", columns: [{ text: "Latte", width: 3, align: "left" }, { text: "2", width: 1, align: "center" }, { text: "6,000", width: 2, align: "right" }] },
    { type: "row", columns: [{ text: "Croissant", width: 3, align: "left" }, { text: "1", width: 1, align: "center" }, { text: "3,500", width: 2, align: "right" }] },
    { type: "rule" },
    { type: "row", columns: [{ text: "TOTAL", width: 3, align: "left" }, { text: "", width: 1, align: "center" }, { text: "9,500", width: 2, align: "right" }], bold: true },
    { type: "feed", lines: 1 },
    { type: "qrcode", data: "https://example.com/receipt/1042", size: 5, ec: "M" },
    { type: "feed", lines: 1 },
    { type: "text", value: "Thank you for your order!", align: "center" },
    { type: "cut" },
  ],
};

const editor = document.querySelector<HTMLTextAreaElement>("#editor")!;
const errorBox = document.querySelector<HTMLDivElement>("#error")!;
const htmlPreviewFrame = document.querySelector<HTMLIFrameElement>("#html-preview")!;
const svgPreview = document.querySelector<HTMLDivElement>("#svg-preview")!;
const textPreview = document.querySelector<HTMLPreElement>("#text-preview")!;
const hexPreview = document.querySelector<HTMLPreElement>("#hex-preview")!;
const printBtn = document.querySelector<HTMLButtonElement>("#print-btn")!;

function toHex(bytes: Uint8Array): string {
  const rows: string[] = [];
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hex = Array.from(chunk).map((b) => b.toString(16).padStart(2, "0")).join(" ");
    const ascii = Array.from(chunk).map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : ".")).join("");
    rows.push(`${i.toString(16).padStart(6, "0")}  ${hex.padEnd(47)}  ${ascii}`);
  }
  return rows.join("\n") || "(empty document)";
}

let lastHtml = "";

function render(): void {
  errorBox.hidden = true;
  errorBox.textContent = "";

  let parsed: unknown;
  try {
    parsed = JSON.parse(editor.value);
  } catch (e) {
    showError(e instanceof Error ? e.message : String(e));
    return;
  }

  let doc: PrintDocument;
  try {
    doc = validateDocument(parsed);
  } catch (e) {
    showError(e instanceof InvalidDocumentError ? e.message : String(e));
    return;
  }

  lastHtml = toHtml(doc);
  htmlPreviewFrame.srcdoc = lastHtml;
  svgPreview.innerHTML = toSvg(doc);
  textPreview.textContent = toPlainText(doc);
  hexPreview.textContent = toHex(toEscPos(doc));
  printBtn.disabled = false;
}

function showError(message: string): void {
  errorBox.hidden = false;
  errorBox.textContent = message;
  printBtn.disabled = true;
}

printBtn.addEventListener("click", () => {
  const win = window.open("", "_blank", "width=420,height=700");
  if (!win) return;
  win.document.open();
  win.document.write(lastHtml);
  win.document.close();
  win.focus();
  win.print();
});

for (const tabBtn of document.querySelectorAll<HTMLButtonElement>(".preview-tabs button")) {
  tabBtn.addEventListener("click", () => {
    for (const b of document.querySelectorAll(".preview-tabs button")) b.classList.remove("active");
    for (const p of document.querySelectorAll(".preview-panel")) p.classList.remove("active");
    tabBtn.classList.add("active");
    document.querySelector(`.preview-panel[data-panel="${tabBtn.dataset.tab}"]`)!.classList.add("active");
  });
}

editor.addEventListener("input", render);
editor.value = JSON.stringify(SAMPLE_DOC, null, 2);
render();
