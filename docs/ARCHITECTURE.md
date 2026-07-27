# Architecture

One universal print document, rendered by twin implementations (TypeScript + Rust)
that share the same JSON schema (`docs/DOCUMENT_FORMAT.md`), delivered to a printer
through whichever transport a platform actually has available.

```
                        ┌─────────────────────────┐
                        │   PrintDocument (JSON)   │   <- docs/DOCUMENT_FORMAT.md
                        └────────────┬─────────────┘
                                     │
              ┌──────────────────────┼──────────────────────┐
              │                      │                      │
      packages/core-ts        crates/core-rs                │
      (render in JS/TS)       (render in Rust)               │
              │                      │                      │
   ┌──────────┴─────────┐   ┌────────┴────────┐             │
   │ toEscPos / toHtml   │   │ to_escpos /      │             │
   │ toSvg / toPlainText │   │ to_pdf / to_text │             │
   └──────────┬─────────┘   └────────┬────────┘             │
              │                      │                      │
   ┌──────────┴─────────┐   ┌────────┴─────────────────┐    │
   │ packages/sdk-web    │   │ crates/print-agent        │    │
   │ (browser, direct    │──▶│ (local loopback binary,   │    │
   │  WebUSB/Web Serial/  │   │  Win/macOS/Linux)         │    │
   │  Web Bluetooth (BLE)/│   │  USB • TCP:9100 • Serial  │    │
   │  window.print, or    │   │  Bluetooth • Win RAW spool│    │
   │  talks to the agent) │   │  CUPS • silent PDF spool  │    │
                              └────────┬─────────────────┘    │
                                       │ shares core-rs         │
                              ┌────────┴─────────┐             │
                              │ packages/sdk-tauri │             │
                              │ (embeds core-rs,   │             │
                              │  no separate agent) │             │
                              └────────────────────┘             │
                                                                  │
                              ┌───────────────────────────────────┘
                              │
                     ┌────────┴─────────┐
                     │ packages/sdk-     │
                     │ electron          │
                     │ (bundles the      │
                     │  print-agent      │
                     │  binary as a      │
                     │  sidecar + native │
                     │  silent print for │
                     │  A4/A3/Letter)    │
                     └───────────────────┘

                     ┌───────────────────────────┐      ┌──────────────────────────────┐
                     │ crates/core-ffi (uniffi)   │      │ packages/sdk_flutter/rust      │
                     │ compiles core-rs to a      │      │ (flutter_rust_bridge, see      │
                     │ native library, for native │      │ "Why sdk_flutter doesn't use   │
                     │ Kotlin/Swift apps that skip│      │ core-ffi" below) -- also        │
                     │ Flutter entirely           │      │ compiles core-rs to a native   │
                     └────────────────────────────┘      │ library, embedded directly in  │
                                                          │ the app (no loopback agent on  │
                                                          │ mobile -- not viable there)     │
                                                          └────────────┬────────────────────┘
                                                                       │
                                                          ┌────────────┴───────────────┐
                                                          │ packages/sdk_flutter        │
                                                          │ Android: USB Host/BLE/TCP   │
                                                          │ iOS: Bluetooth/Network only  │
                                                          │ (no raw USB on iOS — Apple  │
                                                          │ restricted); A4 via AirPrint │
                                                          └─────────────────────────────┘
```

## Why sdk_flutter doesn't use core-ffi

`crates/core-ffi` uses `uniffi`, which officially targets Kotlin/Swift/Python —
its Dart output is unofficial/experimental and not something to build a
product on. `flutter_rust_bridge` is the de facto standard for Flutter+Rust
specifically, so `packages/sdk_flutter` binds directly against `core-rs`
through its own generated bridge (`packages/sdk_flutter/rust`, wired via
`cargokit`) rather than routing through `core-ffi`. Both wrap the exact same
`printers-core` functions and stay in sync by construction — neither
duplicates rendering logic, only the FFI boundary differs. `core-ffi` remains
useful for a team building a native (non-Flutter) Kotlin or Swift app.

## Why two renderer implementations instead of one

Every desktop/browser surface (plain web, Electron, Tauri-webview) can run
TypeScript, so `core-ts` covers all of them from one implementation. But the
print-agent binary and the mobile apps need a renderer with **no JS runtime
dependency** — hence `core-rs`. This mirrors reality already visible across
the existing projects (POS_system has a TS `escpos.ts` *and* a Rust agent;
POS_system_mobile has an independent Dart implementation) — we're reducing
three independent ESC/POS implementations down to two deliberate ones that
share one schema, instead of accidentally drifting.

## Browser Bluetooth is BLE-only

`sdk-web` uses the Web Bluetooth API to print directly to BLE thermal
printers from Chrome/Edge (desktop and Android) without going through the
agent. This has two hard platform limits, not implementation gaps: Web
Bluetooth cannot reach classic Bluetooth (SPP/RFCOMM) printers — only BLE —
and it doesn't exist in Safari, so **no browser-Bluetooth path exists on
iOS**. Classic-Bluetooth printers and iOS Bluetooth printing still work via
`crates/print-agent` (desktop) and `packages/sdk_flutter` (mobile,
native Bluetooth APIs) respectively — this gap is browser-tab-only.

## Why the agent can't reach mobile

Desktop browsers can hold a persistent loopback connection to a locally
installed background process (`http://127.0.0.1:PORT`) because desktop OSes
allow long-running local servers and the browser has no sandboxing barrier
against `localhost`. Mobile OSes don't allow either half of that: iOS and
modern Android restrict background daemons, and a mobile browser tab cannot
reliably reach a local server the way a desktop one can. So mobile printing
is always **native app code**, not a browser-reachable agent. The Rust core
logic is still shared — just delivered as an embedded library (FFI) instead
of a network service.

## Trust / security model

Same shape as the print-agent pattern already proven in POS_system and the
Restaurant Platform: an **Origin allowlist** checked on every request, plus a
**pairing token** issued on first run (shown to the user once, stored by the
calling app) required as a Bearer token on all endpoints except `/health` and
`/pair`. This is deliberately lighter than QZ Tray's full certificate/CA
trust-store model — appropriate here because every caller is one of your own
apps on domains you control, not arbitrary third-party sites. If a future
product needs to let *external* developers integrate against the agent, the
pairing token model can be upgraded to signed requests without changing the
document format or transport layer.

## Directory layout

```
packages/
  core-ts/          TypeScript document model + renderers
  sdk-web/           Browser client SDK (agent client + WebUSB + Web Serial + Web Bluetooth (BLE) + window.print fallback)
  sdk-electron/       Electron main-process bridge
  sdk_flutter/        Flutter plugin (Dart ESC/POS builder + flutter_rust_bridge over core-rs)
    rust/              The flutter_rust_bridge crate (cargokit-built per platform), see "Why sdk_flutter doesn't use core-ffi"
crates/
  core-rs/           Rust document model + renderers (twin of core-ts)
  core-ffi/           uniffi bindings over core-rs, for native (non-Flutter) Kotlin/Swift apps
  print-agent/        Local HTTP+WS agent binary (desktop)
  sdk-tauri/          Example Tauri v2 plugin reusing core-rs
apps/
  demo/               Browser demo: document editor + live preview + print
docs/
  ARCHITECTURE.md     This file
  DOCUMENT_FORMAT.md  The universal PrintDocument JSON schema
  PROTOCOL.md         Print-agent HTTP/WS API
```

## Non-goals for this repo

- This repo does not modify any existing project (POS_system, Global_Link,
  pso_window, Restaurant Platform, TK-Project). It is groundwork meant to be
  copied/adapted into each project later, on its own timeline.
- No cloud/backend service. Everything here runs on the user's own device or
  in their own browser/app.
