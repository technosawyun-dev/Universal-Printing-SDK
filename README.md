# Printers Printing — Universal Printing SDK

A cross-platform printing SDK that solves one problem: build a receipt,
invoice, or voucher **once**, then print it — unchanged — from a browser tab,
an Electron/Tauri desktop app, an Android/iOS app, or a background agent, to
whatever the platform can actually reach: POS thermal printers (58mm/80mm,
via USB, TCP:9100, serial, or Bluetooth) or normal printers (A4/A3/Letter, via
the OS print queue), with the caller only needing the printer's OS driver
installed. It's a generalization of printing logic that already existed,
duplicated, across several production projects (POS_system, Global_Link,
pso_window, the Restaurant Platform) — collapsed into one shared document
schema and two deliberate renderer implementations instead of accidentally
drifting per-project code.

## Tech Stack

- **Rust** — `core-rs` (document model + renderers, incl. PDF, plus complex-
  script text shaping via `rustybuzz`/`ab_glyph` with bundled Noto Sans
  Myanmar fonts), `core-ffi` (uniffi bindings for native mobile),
  `print-agent` (HTTP+WS local server binary), `sdk-tauri` (Tauri v2 plugin)
- **TypeScript** — `core-ts` (document model + renderers, twin of `core-rs`),
  `sdk-web` (browser client), `sdk-electron` (Electron bridge)
- **Dart/Flutter** — `sdk_flutter` (Android/iOS plugin, binds to `core-rs` via
  `flutter_rust_bridge`, not `core-ffi` — see `docs/ARCHITECTURE.md` for why)
- Tooling: pnpm workspaces (JS/TS), Cargo workspace (Rust), Vite (demo app),
  Vitest (TS tests), uniffi/flutter_rust_bridge (FFI bridges)

## Architecture

One `PrintDocument` JSON shape (`docs/DOCUMENT_FORMAT.md`) is rendered by two
independent, schema-compatible implementations — `core-ts` for every
JS-capable surface (browser, Electron, Tauri webview) and `core-rs` for
everything with no JS runtime (the print-agent binary, native mobile). Each
platform's SDK then delivers the rendered output through whatever transport
it actually has: `sdk-web` talks WebUSB/Web Serial/Web Bluetooth (BLE-only)
directly from the browser or falls back to the local `print-agent` over
HTTP/WS; `sdk-electron` bundles the agent as a sidecar plus native silent
printing for normal printers; `sdk-tauri` embeds `core-rs` directly with no
separate agent process; `sdk_flutter` embeds `core-rs` via
`flutter_rust_bridge` since mobile OSes can't host a reachable background
agent. Security follows the pairing-token + Origin-allowlist model already
proven in POS_system and the Restaurant Platform's agents (`docs/PROTOCOL.md`).
See `docs/ARCHITECTURE.md` for the full diagram and rationale (including why
two renderers instead of one, and why `sdk_flutter` bypasses `core-ffi`).

## Monorepo Structure

```
apps/
  demo/            Browser demo: PrintDocument editor + live HTML/SVG/ESC-POS preview
  hardware-test/   Single-file browser page (no build/server needed) for testing
                    USB, Bluetooth, and Desktop-agent transports against real printers
  tauri-test/      Standalone Tauri desktop app proving the "embed core-rs directly,
                    no separate agent" integration style
crates/            Rust workspace
  core-rs/         Document model + renderers (ESC/POS, HTML, SVG, plain text, PDF)
  core-ffi/         uniffi bindings over core-rs, for native (non-Flutter) Kotlin/Swift
  print-agent/       Local HTTP+WS print-agent binary (Windows/macOS/Linux)
  sdk-tauri/          Example Tauri v2 plugin reusing core-rs
packages/          pnpm workspace
  core-ts/         TypeScript document model + renderers (twin of core-rs)
  sdk-web/          Browser client SDK (agent client + WebUSB/Web Serial/Web Bluetooth)
  sdk-electron/      Electron main-process bridge (agent sidecar + native silent print)
  sdk_flutter/        Flutter plugin (Android/iOS), binds to core-rs via flutter_rust_bridge
docs/
  ARCHITECTURE.md   System design and rationale
  DOCUMENT_FORMAT.md  The universal PrintDocument JSON schema
  PROTOCOL.md         Print-agent HTTP/WS API
dist/              Gitignored, user-populated: local build output copied here
                    for hand-testing (printers-agent.exe, .apk) — not shipped
TESTING.md         Real-hardware testing log and step-by-step test guide
```

## Key Features

Verified against real hardware (see Testing Status below for exact scope):

- **Local print-agent** (`crates/print-agent`) — pairing, printer discovery,
  and printing, driven end-to-end through a real browser (CORS bug found and
  fixed in the process)
- **CUPS/RAW printing on Linux and Windows-driver printing** — confirmed
  correct ESC/POS byte output (init command → payload → cut command) for
  thermal-58 and thermal-80 profiles
- **Classic Bluetooth (SPP) printing** via the agent — Linux (BlueZ) and
  Windows (raw `AF_BTH`/`BTHPROTO_RFCOMM` Winsock socket, built and hardware-
  confirmed) both print real receipts
- **Silent (no-dialog) A4/Letter printing** via Windows GDI — paper-size bug
  found and fixed (was always using the driver default regardless of the
  document's declared size); verified against captured PDFs at correct
  dimensions (A4 595x842pt, Letter 612x792pt)
- **TCP/network (port 9100) printing** — byte-inspected and confirmed correct
  against a local listener standing in for a network printer
- **Windows printer enumeration** (`EnumPrintersW`) — driver-installed
  printers now show up in the agent's `/printers` list and are printable by
  name (previously missing entirely; fixed and confirmed via the browser page)
- **Tauri desktop integration** — the embed-core-rs-directly pattern builds
  and runs a real window on both Linux and Windows

Implemented but **not yet hardware-verified**: Web Bluetooth/BLE direct
browser printing (works in principle, but no BLE printer was available to
test), Android app transports beyond what's noted below, macOS (any
transport), iOS (no build exists yet), and Myanmar/complex-script text
shaping in the ESC/POS and PDF renderers (visually previewed to PNG/PDF via
`core-rs`'s example binaries, but not yet printed to a real thermal printer
or confirmed against `TESTING.md`'s hardware log).

## Getting Started / Local Dev

```bash
# JS/TS packages
pnpm install
pnpm build          # pnpm -r --if-present run build
pnpm test           # pnpm -r --if-present run test
pnpm lint
pnpm typecheck

# Rust crates
cargo build --workspace
cargo test --workspace

# Flutter package (needs Flutter SDK)
cd packages/sdk_flutter && flutter test
```

To try printing without writing any code, open
`apps/hardware-test/index.html` directly in Chrome/Edge (no server needed),
alongside the agent — either `cargo run -p printers-print-agent --bin
printers-agent`, or a release build copied into `dist/` (gitignored, not
shipped in the repo — build it yourself with `cargo build --release -p
printers-print-agent`). Full step-by-step instructions, including known
per-platform gotchas, are in `TESTING.md`.

## Testing Status

This project has real hardware-testing history, not just unit tests — but
coverage is uneven across platforms. From `TESTING.md`:

- **Proven on real hardware** (Ubuntu, 2026-07-27 and Windows, 2026-07-30,
  both against the same 80mm USB+Bluetooth thermal printer): the desktop
  agent's pairing/listing/printing flow through a real browser; classic
  Bluetooth SPP printing on both Linux and Windows; USB-driver (CUPS on
  Linux, `POS-58` driver on Windows) RAW ESC/POS printing, byte-verified;
  silent A4/Letter GDI printing on Windows, paper-size-verified; TCP/network
  printing, byte-verified; the Tauri test app building and rendering on both
  Linux and Windows; Windows printer enumeration end-to-end through the
  browser page.
- **Automated suites passing for real on Windows** (not just cross-compiled):
  `pnpm -r test` (86 tests) and `cargo test --workspace` (98 tests).
- **Not yet proven**: Android (Flutter/Android SDK installed, build pending
  at last session), Web Bluetooth/BLE against a real BLE printer, and all of
  macOS/iOS — macOS Bluetooth is still an explicit `NotImplemented`, and
  there is no iOS build yet (both require a Mac that wasn't available).
- **Known, intentional gaps** (not silent failures): WebUSB direct-browser
  printing is blocked whenever the OS already has a driver bound to the
  printer's USB interface (by OS design, not a bug here) — use the desktop
  agent instead in that case; Android's Bluetooth scan only finds BLE, not
  classic SPP, printers.

The previous version of this README described the project only as "early
groundwork, not yet integrated into any production project." That framing is
now stale for the transport/rendering layer itself: real hardware runs,
real bugs (agent CORS, Windows GDI paper size, missing Windows printer
enumeration) have been found and fixed against physical printers on Linux and
Windows. It remains accurate, however, that this SDK has not yet been
integrated *into* any of the existing production apps (POS_system,
Global_Link, pso_window, Restaurant Platform, TK-Project) — that migration is
still future work, on each project's own timeline.
