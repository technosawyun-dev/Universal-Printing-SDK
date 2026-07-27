# Print-agent protocol (v1)

The agent (`crates/print-agent`) listens on `http://127.0.0.1:18999` by
default (configurable; chosen to avoid collision with POS_system's existing
`17891` and the Restaurant Platform's `17999` agents, so all three can run
side by side on one machine during any future migration). Every response is
JSON. A WebSocket at `ws://127.0.0.1:18999/events` streams job status and
printer hotplug events.

## Auth

- `Origin` header must match the agent's allowlist (config file +
  `PRINTERS_AGENT_ORIGINS` env override, comma-separated), checked on every
  request except none — even `/health` is origin-checked, matching the
  existing POS_system/Restaurant Platform agents' behavior.
- First contact: caller calls `POST /pair`. The agent shows a native
  OS notification/dialog ("App X wants to connect — allow?"). On approval it
  returns `{ "token": "..." }`. The caller stores this token and sends it as
  `Authorization: Bearer <token>` on every subsequent request.
- Tokens are per-origin, persisted by the agent, revocable via the agent's
  tray/menu UI (`/pair/revoke` for programmatic revocation during dev).

## Endpoints

### `GET /health`
```jsonc
{ "status": "ok", "version": "1.0.0", "platform": "windows" | "macos" | "linux" }
```
No auth required beyond Origin check — used for the "is the agent installed
and running" probe before any pairing UI is shown.

### `POST /pair`
Body: `{ "appName": "POS System" }` → `{ "token": "..." }` (see Auth above).

### `GET /printers`
Returns everything the agent can currently see, across all transports.
```jsonc
{
  "printers": [
    { "id": "windows:EPSON TM-T88V", "name": "EPSON TM-T88V", "kind": "thermal-80",
      "transport": "windows-queue", "status": "ready" },
    { "id": "tcp:192.168.1.50:9100", "name": "Kitchen printer", "kind": "thermal-80",
      "transport": "tcp", "status": "unknown" },
    { "id": "serial:/dev/ttyUSB0", "name": "USB Serial", "kind": "thermal-58",
      "transport": "serial", "status": "ready" },
    { "id": "bt:AA:BB:CC:DD:EE:FF", "name": "MTP-3 BT", "kind": "thermal-58",
      "transport": "bluetooth", "status": "ready" },
    { "id": "windows:HP LaserJet", "name": "HP LaserJet", "kind": "normal",
      "transport": "windows-queue", "status": "ready" }
  ]
}
```
`tcp:` entries only appear once discovered (see `POST /discover`) or
previously configured — the agent does not scan the network unprompted.

### `POST /discover`
Body: `{ "subnet"?: "192.168.1.0/24", "timeoutMs"?: 2000 }` — probes port
9100 across the subnet (defaults to the agent's local subnet) and mDNS, adds
found devices to the `/printers` list for that session.

### `POST /print`
The main entry point — works for any transport once a `printerId` from
`/printers` is known; the agent picks the right transport internally.
```jsonc
// request
{ "printerId": "windows:EPSON TM-T88V", "document": { /* PrintDocument, see DOCUMENT_FORMAT.md */ } }
// response
{ "jobId": "job_123", "status": "queued" }
```
Also accepts pre-rendered bytes for callers that rendered client-side
(e.g. `sdk-web` going through the agent instead of WebUSB):
```jsonc
{ "printerId": "windows:EPSON TM-T88V", "rawBase64": "<escpos bytes>" }
```

### `POST /print-tcp`
Direct network print without a prior `/discover`, for callers that already
know the IP (matches existing POS_system/Restaurant agent behavior).
```jsonc
{ "host": "192.168.1.50", "port": 9100, "document": { ... } }
```

### `POST /print-serial`
```jsonc
{ "port": "COM3", "baudRate": 9600, "document": { ... } }
```

### `POST /print-bluetooth`
```jsonc
{ "address": "AA:BB:CC:DD:EE:FF", "document": { ... } }
```

### `POST /print-silent`
A4/A3/Letter path — renders `document` to PDF via `core-rs`' `toPdf()` and
sends it to the OS print queue with no dialog.
```jsonc
{ "printerId": "windows:HP LaserJet", "document": { ... }, "copies": 1 }
```

### `GET /jobs/:jobId`
```jsonc
{ "jobId": "job_123", "status": "queued" | "printing" | "done" | "error", "error"?: "..." }
```

## WebSocket `/events`

Same auth (Origin + Bearer token via `?token=` query param on the upgrade
request, since browsers can't set headers on WS handshakes). Emits:
```jsonc
{ "type": "job.status", "jobId": "job_123", "status": "printing" }
{ "type": "printers.changed" }   // hotplug — caller should re-fetch /printers
```

## Error shape

All non-2xx responses:
```jsonc
{ "error": { "code": "PRINTER_NOT_FOUND" | "TRANSPORT_ERROR" | "UNAUTHORIZED" | "INVALID_DOCUMENT" | "...", "message": "..." } }
```

## Compatibility with existing agents

This protocol is a superset of what POS_system's Rust agent and the
Restaurant Platform's Node agent already expose (`/health`, `/printers`,
`/print`, `/print-tcp`) — same verbs, same intent, extended with pairing
auth, serial/bluetooth/silent-print endpoints, job status, and hotplug
events. A future migration of those projects onto this agent is additive,
not a breaking rewrite of their client code.
