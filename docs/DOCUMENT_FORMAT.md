# PrintDocument format (v1.0)

The one JSON shape every renderer (`core-ts`, `core-rs`) and every SDK
consumes. Build a document once from your order/invoice/voucher data; render
it to ESC/POS bytes for a thermal printer, or to HTML/PDF for a normal
printer, without writing platform-specific layout code twice.

Inspired by ReceiptLine's "one document, many backends" design, expressed as
structured JSON instead of markdown-like text — easier to generate
programmatically and to validate identically in TypeScript and Rust.

## Top level

```jsonc
{
  "version": "1.0",
  "paper": {
    "kind": "thermal-58" | "thermal-80" | "a4" | "a3" | "letter" | "custom",
    "widthMm": 80,          // required only when kind == "custom"
    "marginMm": 0,          // optional, default 0 for thermal, 10 for a4/a3/letter
    "charsPerLine": 48       // optional override; defaults: thermal-58 -> 32, thermal-80 -> 48
  },
  "meta": {
    "title": "Receipt #1042",  // optional, used as PDF/window title
    "encoding": "cp437",       // optional, ESC/POS codepage hint, default cp437
    "locale": "en-US"          // optional
  },
  "content": [ /* Node[], see below */ ]
}
```

`paper.kind` is the single switch a renderer uses to decide char-width
(thermal) vs. page geometry (normal printers). Everything else in `content`
is renderer-agnostic.

## Content nodes

Every node has a required `type`. Renderers that don't support a node type
(e.g. `barcode` on a renderer with no symbology library available) must
degrade gracefully — render nothing rather than throwing — and should log a
warning.

### `text`
```jsonc
{ "type": "text", "value": "Thank you for your order",
  "align": "left" | "center" | "right",   // default "left"
  "bold": false, "underline": false, "invert": false,
  "size": { "w": 1, "h": 1 },              // 1-8 each, ESC/POS character scale
  "font": "a" | "b"                        // default "a"
}
```

### `row`
Column layout — the standard "item name … qty … price" line. Column widths
are fractional units (`fr`, like CSS grid) so the same row definition adapts
to 32-, 42-, and 48-column thermal widths and to A4 point-widths alike.
```jsonc
{ "type": "row",
  "columns": [
    { "text": "Latte", "width": 3, "align": "left" },
    { "text": "x2",    "width": 1, "align": "center" },
    { "text": "6,000", "width": 2, "align": "right" }
  ],
  "bold": false
}
```

### `rule`
Horizontal divider.
```jsonc
{ "type": "rule", "char": "-" }   // default "-"; renderer maps to a border line for HTML/PDF
```

### `feed`
```jsonc
{ "type": "feed", "lines": 1 }
```

### `image`
Raster image (logo, etc). `data` is a base64 PNG. Thermal renderers convert
to an ESC/POS raster/column bit-image; HTML/PDF renderers place it as-is.
```jsonc
{ "type": "image", "data": "<base64 png>", "widthPx": 200, "align": "center" }
```

### `barcode`
```jsonc
{ "type": "barcode", "symbology": "code128" | "code39" | "ean13" | "upca",
  "data": "0123456789012", "heightPx": 60, "showText": true }
```

### `qrcode`
```jsonc
{ "type": "qrcode", "data": "https://...", "size": 6, "ec": "M" }
```

### `cut`
Thermal-only; no-op for normal printers.
```jsonc
{ "type": "cut", "mode": "full" | "partial" }   // default "partial"
```

### `drawer`
Cash-drawer kick pulse. Thermal-only; no-op for normal printers.
```jsonc
{ "type": "drawer", "pin": 2 | 5 }   // default 2
```

### `raw`
Escape hatch for printer-specific ESC/POS bytes the schema doesn't model
yet. Ignored by HTML/SVG/PDF renderers.
```jsonc
{ "type": "raw", "escposBase64": "<base64 bytes>" }
```

## Renderer contract

Both `core-ts` and `core-rs` must expose the same four entry points and
agree on output for the same input document:

| Function          | Output                          | Used for                          |
|--------------------|----------------------------------|------------------------------------|
| `toEscPos(doc)`     | `Uint8Array` / `Vec<u8>`         | thermal-58 / thermal-80            |
| `toHtml(doc)`       | `string` (self-contained HTML)  | browser preview, `window.print()`  |
| `toSvg(doc)`        | `string` (SVG)                   | thumbnails, non-print preview      |
| `toPlainText(doc)`  | `string`                         | debug, non-graphical fallback      |

`core-rs` additionally exposes `toPdf(doc) -> Vec<u8>`, used by the
print-agent's silent A4/A3/Letter path (rendering PDF in a browser JS
context is unnecessary — the agent handles that transport).

## Compatibility note

`paper.kind` values map to physical printers as follows — this is the same
mapping already used across POS_system / Global_Link / pso_window, kept
intentionally so migrating those projects onto this schema later is a data
mapping exercise, not a rewrite:

| kind         | typical width | typical use            |
|---------------|----------------|--------------------------|
| `thermal-58`  | 32 chars       | small receipts           |
| `thermal-80`  | 48 chars       | receipts, kitchen tickets|
| `a4`/`a3`/`letter` | page geometry | invoices, vouchers, warranty certs |
