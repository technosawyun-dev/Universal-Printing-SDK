# Printers printing

A universal printing SDK: build a receipt/invoice once, print it from a
browser, an Android/iOS app, or a macOS/Windows desktop app, to POS thermal
printers (58mm/80mm) or normal printers (A4/A3/Letter) — with the caller
only needing the printer's OS driver installed.

See `docs/ARCHITECTURE.md` for the system design, `docs/DOCUMENT_FORMAT.md`
for the universal document schema, and `docs/PROTOCOL.md` for the local
print-agent's API.

## Layout

- `packages/core-ts` — TypeScript document model + renderers (ESC/POS, HTML, SVG, plain text)
- `packages/sdk-web` — browser client SDK (agent client, WebUSB, Web Serial, `window.print()` fallback)
- `packages/sdk-electron` — Electron main-process bridge
- `packages/sdk-flutter` — Flutter plugin (Android/iOS)
- `crates/core-rs` — Rust document model + renderers (twin of core-ts, adds PDF)
- `crates/core-ffi` — uniffi bindings over core-rs for mobile
- `crates/print-agent` — local HTTP+WS print-agent binary (Windows/macOS/Linux)
- `crates/sdk-tauri` — example Tauri command wrapper reusing core-rs directly
- `apps/demo` — browser demo: document editor + live preview + print

## Status

Early groundwork — not yet integrated into any of the existing production
projects (POS_system, Global_Link, pso_window, Restaurant Platform,
TK-Project). Meant to be copied/adapted into each of them later.

## Getting started

```bash
pnpm install
pnpm build
pnpm test

cargo build --workspace
cargo test --workspace
```
