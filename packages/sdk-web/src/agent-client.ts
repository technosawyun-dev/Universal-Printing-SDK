// HTTP/WS client for crates/print-agent, per docs/PROTOCOL.md. This is the
// "talks to the agent" half of sdk-web (see docs/ARCHITECTURE.md) -- the
// other half (webusb.ts/web-serial.ts/web-bluetooth.ts) talks to hardware
// directly, bypassing the agent entirely.

import type { PrintDocument } from "@printers/core-ts";
import { AgentClientError } from "./errors.js";

const DEFAULT_BASE_URL = "http://127.0.0.1:18999";

export interface AgentPrinter {
  id: string;
  name: string;
  kind: string;
  transport: string;
  status: string;
}

export type JobState = "queued" | "printing" | "done" | "error";

export interface JobStatus {
  jobId: string;
  status: JobState;
  error?: string;
}

export type AgentEvent = { type: "job.status"; jobId: string; status: JobState } | { type: "printers.changed" };

export interface AgentClientOptions {
  /** Default: http://127.0.0.1:18999 -- see docs/PROTOCOL.md for why that port. */
  baseUrl?: string;
  /** Injectable for testing; defaults to the global fetch. */
  fetchImpl?: typeof fetch;
  /** Injectable for testing; defaults to the global WebSocket. */
  webSocketCtor?: typeof WebSocket;
  /** Restores a previously-paired token (e.g. loaded from localStorage) without re-pairing. */
  token?: string;
  /**
   * Per-request timeout in milliseconds. Default 2500, matching the value
   * POS_system's existing print-agent client already uses in production --
   * without one, a hung/unresponsive agent leaves the caller waiting on an
   * unresolved fetch() indefinitely.
   */
  timeoutMs?: number;
}

const DEFAULT_TIMEOUT_MS = 2500;

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  const globalBtoa = (globalThis as { btoa?: (s: string) => string }).btoa;
  if (typeof globalBtoa === "function") return globalBtoa(binary);
  const nodeBuffer = (globalThis as { Buffer?: { from(s: string, enc: string): { toString(enc: string): string } } }).Buffer;
  if (nodeBuffer) return nodeBuffer.from(binary, "binary").toString("base64");
  throw new Error("No base64 encoder available in this environment (need btoa or Buffer)");
}

/**
 * Talks to a locally running `crates/print-agent` over HTTP + WebSocket.
 * Every method after `pair()` requires a token; call `pair()` once per
 * origin (the agent shows the user a native "allow this app?" prompt) and
 * persist the returned token yourself (e.g. localStorage) to skip re-pairing
 * on future page loads -- pass it back via `AgentClientOptions.token`.
 */
export class AgentClient {
  private baseUrl: string;
  private fetchImpl: typeof fetch;
  private webSocketCtor: typeof WebSocket;
  private token: string | null;
  private timeoutMs: number;

  constructor(options: AgentClientOptions = {}) {
    this.baseUrl = options.baseUrl ?? DEFAULT_BASE_URL;
    this.fetchImpl = options.fetchImpl ?? globalThis.fetch?.bind(globalThis);
    this.webSocketCtor = options.webSocketCtor ?? (globalThis as unknown as { WebSocket?: typeof WebSocket }).WebSocket!;
    this.token = options.token ?? null;
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  getToken(): string | null {
    return this.token;
  }

  setToken(token: string | null): void {
    this.token = token;
  }

  private async request<T>(method: string, path: string, body?: unknown, auth = true): Promise<T> {
    const headers: Record<string, string> = {};
    if (body !== undefined) headers["Content-Type"] = "application/json";
    if (auth) {
      if (!this.token) throw new AgentClientError("UNAUTHORIZED", "not paired yet -- call pair() first");
      headers.Authorization = `Bearer ${this.token}`;
    }

    let res: Response;
    try {
      res = await this.fetchImpl(`${this.baseUrl}${path}`, {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
        signal: AbortSignal.timeout(this.timeoutMs),
      });
    } catch (e) {
      if (e instanceof Error && e.name === "TimeoutError") {
        throw new AgentClientError("AGENT_TIMEOUT", `no response from the print agent within ${this.timeoutMs}ms -- is it running?`);
      }
      throw e;
    }
    const json = await res.json().catch(() => null);
    if (!res.ok) {
      const err = (json as { error?: { code: string; message: string } } | null)?.error ?? {
        code: "UNKNOWN",
        message: `request failed with status ${res.status}`,
      };
      throw new AgentClientError(err.code, err.message);
    }
    return json as T;
  }

  async health(): Promise<{ status: string; version: string; platform: string }> {
    return this.request("GET", "/health", undefined, false);
  }

  async pair(appName: string): Promise<string> {
    const { token } = await this.request<{ token: string }>("POST", "/pair", { appName }, false);
    this.token = token;
    return token;
  }

  async pairRevoke(): Promise<void> {
    await this.request("POST", "/pair/revoke");
    this.token = null;
  }

  async listPrinters(): Promise<AgentPrinter[]> {
    const { printers } = await this.request<{ printers: AgentPrinter[] }>("GET", "/printers");
    return printers;
  }

  async discover(subnet?: string, timeoutMs?: number): Promise<AgentPrinter[]> {
    const { printers } = await this.request<{ printers: AgentPrinter[] }>("POST", "/discover", { subnet, timeoutMs });
    return printers;
  }

  async print(printerId: string, document: PrintDocument): Promise<JobStatus> {
    return this.request("POST", "/print", { printerId, document });
  }

  async printRawBytes(printerId: string, bytes: Uint8Array): Promise<JobStatus> {
    return this.request("POST", "/print", { printerId, rawBase64: bytesToBase64(bytes) });
  }

  async printTcp(host: string, port: number, document: PrintDocument): Promise<JobStatus> {
    return this.request("POST", "/print-tcp", { host, port, document });
  }

  async printSerial(port: string, baudRate: number | undefined, document: PrintDocument): Promise<JobStatus> {
    return this.request("POST", "/print-serial", { port, baudRate, document });
  }

  async printBluetooth(address: string, document: PrintDocument): Promise<JobStatus> {
    return this.request("POST", "/print-bluetooth", { address, document });
  }

  async printSilent(printerId: string, document: PrintDocument, copies?: number): Promise<JobStatus> {
    return this.request("POST", "/print-silent", { printerId, document, copies });
  }

  async getJob(jobId: string): Promise<JobStatus> {
    return this.request("GET", `/jobs/${encodeURIComponent(jobId)}`);
  }

  /**
   * Opens `/events` and streams job-status/printer-hotplug notifications.
   * Returns an unsubscribe function that closes the socket. The pairing
   * token travels as `?token=` since browsers can't set headers on a
   * WebSocket handshake -- see docs/PROTOCOL.md.
   */
  connectEvents(onEvent: (event: AgentEvent) => void): () => void {
    if (!this.token) throw new AgentClientError("UNAUTHORIZED", "not paired yet -- call pair() first");
    const wsUrl = `${this.baseUrl.replace(/^http/, "ws")}/events?token=${encodeURIComponent(this.token)}`;
    const ws = new this.webSocketCtor(wsUrl);
    ws.addEventListener("message", (ev: MessageEvent) => {
      try {
        onEvent(JSON.parse(ev.data as string) as AgentEvent);
      } catch {
        // ignore malformed events rather than crashing the caller's page
      }
    });
    return () => ws.close();
  }
}
