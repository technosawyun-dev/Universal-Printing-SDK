# Testing this project

This is a plain-language guide for testing the Printers SDK with **real
printers**, plus how to re-run the automated test suites if you make
changes. Written so it works after moving to a different computer (e.g.
Ubuntu → Windows) with nothing pre-installed.

**Already proven on real hardware (2026-07-27, on the Ubuntu machine, an
80mm USB+Bluetooth thermal printer, model "Y58_08DA"):**
- The Desktop agent panel — pairing, listing, and printing — fully works
  end-to-end through a real browser, clicked by hand, printing real receipts
  (this uncovered and fixed a real bug: the agent had no CORS support at
  all, so browser pairing could never have worked before this fix).
- The same agent's Bluetooth (classic SPP) transport, driven directly via
  its HTTP API.
- The Tauri desktop app's render step (webview → Rust plugin → bytes).

**Also learned, specific to this printer model:** its USB connects fine via
WebUSB but can never print through it on Linux (the `usblp` kernel driver
already owns the device — the same reason the CUPS queue was needed at
all), and its Bluetooth is classic SPP, which Web Bluetooth (BLE-only) can
never reach, on any browser. Neither is a bug — for a printer like this,
**the Desktop agent panel is the one to actually use**, not the direct
USB/Bluetooth browser panels. Your printer may differ (a real BLE printer
would work fine over Web Bluetooth, for instance).

**Also proven on real hardware (2026-07-30, on the Windows laptop, the same
Y58_08DA 80mm USB+Bluetooth thermal printer, plus the Windows-installed
`POS-58` driver and `Microsoft Print to PDF`):**
- `pnpm install` / `pnpm build` / `pnpm -r test` (86 tests) and
  `cargo test --workspace` (98 tests, up from 95 — this session added
  Bluetooth unit tests) both pass for real on Windows, not just cross-compiled.
- `printers-agent.exe` builds, runs, and its `/health` endpoint reports
  `"platform":"windows"`.
- The Tauri desktop test app (`apps/tauri-test/`) builds and opens a real
  window on Windows for the first time — it had only ever run on Linux
  before. (Needed one fix to build at all: `icons/icon.ico` didn't exist,
  only `icons/icon.png` — generated via .NET `System.Drawing` since
  `tauri-build`'s Windows resource step requires an `.ico` even with
  bundling disabled.)
- RAW ESC/POS to the installed `POS-58` driver, for both thermal-58 and
  thermal-80 document profiles: byte-inspected (via a printer bound to a
  local-file port instead of the real USB port) and confirmed correct —
  starts with the ESC/POS init command, ends with the requested cut command.
- Silent (no-dialog) GDI printing to A4 and Letter: **found and fixed a real
  bug** — `print_document_gdi` was passing a null `DEVMODE` to `CreateDCW`,
  so every job silently used the printer driver's default paper size
  (Letter) regardless of what the document declared; an A4 receipt printed
  as Letter with no error. Fixed by fetching the printer's DEVMODE via
  `DocumentPropertiesW` and setting `dmPaperSize`/`dmPaperWidth`/
  `dmPaperLength` before `CreateDCW`. Verified with real captured PDFs: A4 is
  now 595x842pt, Letter 612x792pt (both were 612x792pt before the fix).
- WiFi/network printing: verified against a local TCP:9100 listener
  standing in for a network printer — byte-inspected and confirmed correct
  ESC/POS.
- **Classic Bluetooth on Windows, previously not implemented at all,
  built from scratch this session** (`crates/print-agent/src/transport/bluetooth.rs`,
  a raw `AF_BTH`/`BTHPROTO_RFCOMM` Winsock socket — no extra runtime
  dependency beyond `ws2_32.dll`, which ships on every Windows install).
  **Confirmed on real hardware**: printed an actual receipt to the paired
  Y58_08DA printer over classic Bluetooth SPP from Windows, and visually
  confirmed the physical printout matched the sent content exactly.
- `GET /printers` didn't enumerate Windows-installed printers at all (only
  CUPS/serial/discovered-TCP), so a printer like `POS-58` had no way to
  appear in the hardware-test page's "Desktop agent" dropdown even though
  the backend could already print to it directly by name. Fixed by adding
  `EnumPrintersW`-based listing (same technique already proven in the
  Sawyun POS project's production print-agent). **Confirmed via the actual
  browser page**: paired, listed printers (now showing
  `windows:POS-58(copy of 2)`), selected it, clicked print, and a real
  receipt came out.

What's *not* proven yet: Android (in progress — Flutter/Android SDK
installed this session, build pending) and everything macOS/iOS (no Mac was
available this session; macOS's Bluetooth path is still `NotImplemented`
the same as before, and iOS still has no build at all).

## Quick summary of what to use for what

| Your printer connects via... | What to test with |
|---|---|
| WiFi / network (has its own IP address) | The desktop agent (`dist/printers-agent.exe` on Windows) + the hardware test page's "Desktop agent" panel, using the WiFi box |
| USB cable | Either the hardware test page's "USB printer" panel (Chrome/Edge), or the desktop agent if the printer is already installed as a normal Windows printer |
| Bluetooth | The hardware test page's "Bluetooth printer" panel (Chrome/Edge, BLE printers only), **or** the desktop agent if your printer uses classic Bluetooth (only works on Linux right now — see Known Gaps below) |
| Android phone/tablet | `dist/printers-flutter-test.apk` — see Part 1d |
| A Tauri/desktop-app-style integration test | `apps/tauri-test/` — see Part 1e |
| No real printer yet | Open `apps/demo/dist/index.html` (browser preview) or just read `apps/hardware-test/index.html`'s log output |

---

## Part 1 — Testing with a real printer

### 1a. The hardware test page (no install needed)

`apps/hardware-test/index.html` is a single self-contained page — just
**double-click it to open it in Chrome or Edge**. No server, no Node.js, no
build step. It has three panels: USB, Bluetooth, and "Desktop agent" (for
WiFi printers or anything routed through the background agent).

Each panel has numbered buttons — click them in order (1, then 2, etc.).
There's a log at the bottom showing exactly what happened, including error
messages if something fails.

> If the USB/Bluetooth buttons do nothing or show a security error: some
> browsers restrict those APIs when a page is opened directly as a file.
> If that happens, serve the `apps/hardware-test` folder with any simple
> local web server instead of double-clicking it (e.g. if Python is
> installed: open a terminal in that folder and run `python -m http.server
> 8080`, then visit `http://localhost:8080` instead of the file directly).

### 1b. The desktop agent (for WiFi printers, Windows-installed USB printers, or classic Bluetooth)

The "agent" is a small background program that actually talks to the
printer. `dist/` is gitignored and not shipped in the repo, so you need to
build it yourself first (see "To rebuild the agent yourself later" below for
the exact command) — the result lands at `target\release\printers-agent.exe`;
copy or symlink it to `dist\printers-agent.exe` if you want the paths below
to match exactly, or just substitute the `target\release\` path throughout.

**`dist/printers-agent.exe`** — once built, just double-click it. A console
window will open and stay open (that's normal — it's running the server).
Leave it running while you test.

**Important first-time setup:** because the hardware test page is opened as
a local file, you need to tell the agent to allow that. Before double
clicking the .exe, set this once per Windows session:

1. Press `Win + R`, type `cmd`, press Enter (opens a black command window).
2. Type this and press Enter:
   ```
   set PRINTERS_AGENT_ORIGINS=null
   ```
3. In that same window, run the agent by typing its full path, e.g.:
   ```
   "C:\path\to\dist\printers-agent.exe"
   ```
   (Or drag-and-drop the .exe file into the command window instead of typing
   the path, then press Enter.)

You should see: `printers-agent listening on http://127.0.0.1:18999`. Leave
this window open. Now go to the hardware test page, click "1. Pair with
agent" — you're testing for real from this point on.

**If you'd rather not use the command line:** just double-click the .exe
normally. It will still work for WiFi printer testing via the "Print via
WiFi" box specifically, since that one doesn't require pairing from a
file:// page in the same way -- but if pairing fails with an "origin not
allowed" error, use the steps above instead.

**To rebuild the agent yourself later** (e.g. after a code change), you'd
need Rust installed on Windows and run (from the repo root):
```
cargo build -p printers-print-agent --bin printers-agent --release
```
The new .exe appears in `target\release\printers-agent.exe`.

### 1c. What to actually test, in order

1. **WiFi printer, if you have one**: agent running → hardware test page →
   "Desktop agent" panel → type the printer's IP address into the WiFi box
   → "Print via WiFi". This is the most reliable path — start here.
2. **USB printer**: plug it in → hardware test page → "USB printer" panel →
   "Connect USB printer" (a picker will pop up, choose your printer) →
   "Print test receipt".
3. **Bluetooth printer**: pair it with Windows normally first (Settings →
   Bluetooth) → hardware test page → "Bluetooth printer" panel → "Connect
   Bluetooth printer" → "Print test receipt". If nothing shows up in the
   picker, your printer likely uses classic Bluetooth, not BLE — Web
   Bluetooth can't reach it (see Known Gaps).

Whatever happens (success or failure), the **Log box at the bottom of the
page tells you exactly what occurred** — copy that text back if you need
help debugging something.

### 1d. Android app (`dist/printers-flutter-test.apk`)

`dist/` is gitignored and not shipped, so build the debug APK yourself first
(see "To rebuild this APK yourself later" below) — it needs Android Studio's
SDK and Flutter installed, there's no way to skip that step.

1. Copy the built APK (`packages/sdk_flutter/example/build/app/outputs/flutter-apk/app-debug.apk`)
   to your Android phone (USB cable,
   or upload it somewhere and download it on the phone).
2. Open the file on the phone. Android will warn about installing from an
   unknown source the first time — allow it (this is normal for a debug
   build not from the Play Store).
3. Open the app. It has three panels: **USB**, **Bluetooth**, **WiFi/TCP**.
   - USB: tap "List USB devices" (you may need a USB-OTG cable/adapter to
     plug the printer into an Android phone), then "Print test" next to it.
   - Bluetooth: tap "Scan 5s". **Important:** this only finds BLE
     (Bluetooth Low Energy) printers. Many cheap "Y-series" thermal
     printers use classic Bluetooth (SPP) instead, and simply will not
     appear in this list — that's a real current limitation of the
     Android/Flutter side, not a bug you're triggering. If your printer
     doesn't show up here, use the WiFi/TCP panel instead if your printer
     also supports network printing, or fall back to testing that printer
     via the Windows desktop agent instead.
   - WiFi/TCP: type your printer's IP address and port (default 9100),
     tap "Send test receipt".
4. Grant Bluetooth/location permissions if Android prompts for them when
   you tap "Scan" — without them the scan will silently find nothing.

**To rebuild this APK yourself later** (e.g. after a code change), you need
Flutter and Android Studio's SDK installed, then from
`packages/sdk_flutter/example`:
```
flutter build apk --debug
```
If this fails with `rustc` crashing (`SIGSEGV`) while compiling `printpdf`,
that's a real cross-compiler stack-overflow issue this session hit too —
fix it by setting this once per terminal session before building:
```
export RUST_MIN_STACK=67108864
```
The APK appears at
`packages/sdk_flutter/example/build/app/outputs/flutter-apk/app-debug.apk`.

### 1e. Tauri desktop test app (`apps/tauri-test/`)

This is a small standalone desktop app (not Global Link's real code) that
proves out the alternate integration style: the rendering code runs
embedded directly inside the app's own program, with no separate
`printers-agent.exe` process alongside it — the pattern a real desktop app
like Global Link would actually use.

Proven on Linux originally, and now also on Windows (2026-07-30): builds
clean and opens a real window (needed one fix first — `icons/icon.ico` was
missing, only `icons/icon.png` existed; `tauri-build`'s Windows resource
step requires an `.ico` even with bundling disabled). If you're setting this
up fresh on another Windows machine (Tauri apps can't be cross-compiled to
Windows from Linux, unlike the plain agent .exe), once Rust is installed on
Windows:
```
cd apps/tauri-test/src-tauri
cargo build --release
```
Run the resulting `.exe` (in `target\release\`). It has the same "1. render,
then 2. choose a delivery method" flow as the browser hardware-test page:
click "Render to ESC/POS" first, then either "Send via TCP" (for a
network printer) or "Send via Windows spooler" (type the exact name of a
printer already installed in Windows).

---

## Part 2 — Re-running the automated test suites

You only need this if you're changing code, not for normal printer testing.
Each package has its own tests; here's how to run all of them.

### Prerequisites (install once)

- **Node.js** (v20+) and **pnpm** (`npm install -g pnpm`) — for the
  TypeScript packages (`core-ts`, `sdk-web`, `sdk-electron`, `apps/demo`).
- **Rust** (via [rustup.rs](https://rustup.rs)) — for the Rust crates
  (`core-rs`, `core-ffi`, `print-agent`, `sdk-tauri`).
- **Flutter** (via [flutter.dev](https://flutter.dev)) — only if testing
  `sdk_flutter`.

### Running everything

From the repo root:

```bash
# Install JS dependencies (first time only, or after pulling changes)
pnpm install

# Run every TypeScript package's tests
pnpm -r test

# Run every Rust crate's tests
cargo test --workspace

# Run the Flutter package's tests
cd packages/sdk_flutter
flutter test
```

All of these should print "passed"/"ok" for everything, with zero failures.
If something fails after a change you made, the error output tells you
which file/test broke.

### Windows-specific note

The Rust code that talks to Windows (`crates/print-agent/src/transport/windows_queue.rs`)
can only be *fully* built and tested with the `cargo test --workspace`
command **on an actual Windows machine** (or by cross-compiling, which
needs extra setup `cargo test --workspace` doesn't do automatically). If
you're on Windows now, this is your chance to run it for real —
`cargo test --workspace` there exercises code paths that were previously
only cross-compiled from Linux, never run.

---

## Known gaps (things that won't work yet, on purpose, not silently broken)

- **Classic Bluetooth printers via the desktop agent** now work on both
  Linux (BlueZ) and Windows (a raw `AF_BTH`/`BTHPROTO_RFCOMM` Winsock
  socket, added and hardware-confirmed 2026-07-30) — only macOS still
  returns a clear `NotImplemented` error instead of failing silently. This
  same limitation is why some printers won't show up in the Android app's
  Bluetooth scan either (see 1d) — the Android side only supports BLE, not
  classic SPP, at all right now.
- ~~Silent (no-dialog) printing to a normal A4/Letter printer on Windows~~
  **fixed 2026-07-30**: it was ignoring the document's declared paper size
  entirely (always printing at the driver's default, typically Letter, even
  for an A4 document) because `CreateDCW` was never given a `DEVMODE`. Now
  fetches and sets the correct paper size before creating the device
  context; verified against real printer output.
- **iOS** has no build at all yet (needs a Mac).
- ~~The Tauri desktop test app (1e) has only been run on Linux~~ **also
  confirmed on Windows 2026-07-30**: builds clean (after adding the missing
  `icons/icon.ico`) and opens a real window.
- **macOS** hasn't been touched since the original Linux/Windows testing —
  no session has run any of this on real macOS hardware yet, so the
  CUPS-based RAW path there (code-identical to Linux's) is unverified in
  practice, on top of the Bluetooth gap above.
- **Direct browser printing without the agent** (WebUSB) is blocked by
  design whenever Windows already has a driver claiming the printer's USB
  interface (confirmed 2026-07-30 against a real printer) — the OS only
  allows one driver bound to a USB interface at a time, so a driver-claimed
  printer needs the Desktop Agent panel (which now lists Windows-installed
  printers automatically, see below) rather than the direct USB/Bluetooth
  browser panels. This mirrors the same-shaped Linux limitation noted above
  for WebUSB/`usblp`.
- Fixed 2026-07-30: `GET /printers` never listed Windows-installed printers
  (only CUPS/serial/discovered-TCP), so the hardware-test page's "Desktop
  agent" dropdown had no way to show a driver-installed printer like
  `POS-58` even though the backend could already print to it by name. Now
  enumerates via `EnumPrintersW`; confirmed end-to-end through the actual
  browser page.
