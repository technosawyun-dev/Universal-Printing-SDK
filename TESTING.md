# Testing this project

This is a plain-language guide for testing the Printers SDK with **real
printers**, plus how to re-run the automated test suites if you make
changes. Written so it works after moving to a different computer (e.g.
Ubuntu → Windows) with nothing pre-installed.

## Quick summary of what to use for what

| Your printer connects via... | What to test with |
|---|---|
| WiFi / network (has its own IP address) | The desktop agent (`dist/printers-agent.exe` on Windows) + the hardware test page's "Desktop agent" panel, using the WiFi box |
| USB cable | Either the hardware test page's "USB printer" panel (Chrome/Edge), or the desktop agent if the printer is already installed as a normal Windows printer |
| Bluetooth | The hardware test page's "Bluetooth printer" panel (Chrome/Edge, BLE printers only), **or** the desktop agent if your printer uses classic Bluetooth (only works on Linux right now — see Known Gaps below) |
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
printer. On Windows, a ready-to-run copy is already built for you:

**`dist/printers-agent.exe`** — just double-click it. A console window will
open and stay open (that's normal — it's running the server). Leave it
running while you test.

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

- **Classic Bluetooth printers via the desktop agent** only work on Linux
  right now (returns a clear error on Windows/macOS instead of failing
  silently).
- **Silent (no-dialog) printing to a normal A4/Letter printer on Windows**
  was just implemented this session and has never been tested against a
  real Windows printer driver — if you try it and something looks wrong
  (mis-positioned text, wrong size, etc.), that's the most likely place to
  report a bug.
- **iOS** has no build at all yet (needs a Mac).
