import { describe, expect, it, vi } from "vitest";
import { AgentClient } from "../agent-client.js";
import { AgentClientError } from "../errors.js";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

describe("AgentClient", () => {
  it("calls /health without requiring a token", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(jsonResponse(200, { status: "ok", version: "1.0.0", platform: "linux" }));
    const client = new AgentClient({ fetchImpl });
    const health = await client.health();
    expect(health.status).toBe("ok");
    expect(fetchImpl).toHaveBeenCalledWith("http://127.0.0.1:18999/health", expect.objectContaining({ method: "GET" }));
    const [, init] = fetchImpl.mock.calls[0];
    expect(init.headers.Authorization).toBeUndefined();
  });

  it("pair() stores the returned token for subsequent requests", async () => {
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(200, { token: "abc123" }))
      .mockResolvedValueOnce(jsonResponse(200, { printers: [] }));
    const client = new AgentClient({ fetchImpl });

    const token = await client.pair("My App");
    expect(token).toBe("abc123");
    expect(client.getToken()).toBe("abc123");

    await client.listPrinters();
    const [, init] = fetchImpl.mock.calls[1];
    expect(init.headers.Authorization).toBe("Bearer abc123");
  });

  it("throws AgentClientError before pairing when a protected call is made", async () => {
    const client = new AgentClient({ fetchImpl: vi.fn() });
    await expect(client.listPrinters()).rejects.toThrow(AgentClientError);
  });

  it("surfaces the agent's { error: { code, message } } shape as AgentClientError", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(jsonResponse(404, { error: { code: "PRINTER_NOT_FOUND", message: "no such printer" } }));
    const client = new AgentClient({ fetchImpl, token: "abc123" });
    await expect(client.listPrinters()).rejects.toMatchObject({ code: "PRINTER_NOT_FOUND", message: "no such printer" });
  });

  it("printRawBytes base64-encodes the payload", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(jsonResponse(200, { jobId: "job_1", status: "queued" }));
    const client = new AgentClient({ fetchImpl, token: "abc123" });
    await client.printRawBytes("tcp:1.2.3.4:9100", new Uint8Array([0x1b, 0x40]));
    const [, init] = fetchImpl.mock.calls[0];
    const body = JSON.parse(init.body);
    expect(body.rawBase64).toBe(Buffer.from([0x1b, 0x40]).toString("base64"));
  });

  it("pairRevoke clears the stored token", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    const client = new AgentClient({ fetchImpl, token: "abc123" });
    await client.pairRevoke();
    expect(client.getToken()).toBeNull();
  });

  it("connectEvents opens a ws:// URL with the token as a query param", () => {
    const addEventListener = vi.fn();
    const close = vi.fn();
    const webSocketCtor = vi.fn().mockImplementation(() => ({ addEventListener, close }));
    const client = new AgentClient({ webSocketCtor: webSocketCtor as unknown as typeof WebSocket, token: "abc123" });

    const unsubscribe = client.connectEvents(() => {});
    expect(webSocketCtor).toHaveBeenCalledWith("ws://127.0.0.1:18999/events?token=abc123");
    unsubscribe();
    expect(close).toHaveBeenCalled();
  });

  it("throws a clear AgentClientError when the agent doesn't respond within the timeout", async () => {
    // Mirrors real fetch()'s behavior of rejecting with a TimeoutError
    // DOMException when its AbortSignal fires -- matches the 2.5s timeout
    // POS_system's existing print-agent client already uses in production.
    const fetchImpl = vi.fn(
      (_url: string, init?: RequestInit) =>
        new Promise<Response>((_resolve, reject) => {
          init?.signal?.addEventListener("abort", () => reject(new DOMException("Timed out", "TimeoutError")));
        }),
    );
    const client = new AgentClient({ fetchImpl: fetchImpl as unknown as typeof fetch, timeoutMs: 10 });
    await expect(client.health()).rejects.toMatchObject({ code: "AGENT_TIMEOUT" });
  });

  it("connectEvents forwards parsed JSON messages to the callback", () => {
    let messageHandler: ((ev: { data: string }) => void) | undefined;
    const webSocketCtor = vi.fn().mockImplementation(() => ({
      addEventListener: (_event: string, handler: (ev: { data: string }) => void) => {
        messageHandler = handler;
      },
      close: vi.fn(),
    }));
    const client = new AgentClient({ webSocketCtor: webSocketCtor as unknown as typeof WebSocket, token: "abc123" });
    const onEvent = vi.fn();
    client.connectEvents(onEvent);

    messageHandler!({ data: JSON.stringify({ type: "printers.changed" }) });
    expect(onEvent).toHaveBeenCalledWith({ type: "printers.changed" });
  });
});
