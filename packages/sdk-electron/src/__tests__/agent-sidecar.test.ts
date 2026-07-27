import { describe, expect, it, vi } from "vitest";
import { startAgentSidecar } from "../agent-sidecar.js";

function fakeChildProcess() {
  return { kill: vi.fn() } as unknown as import("node:child_process").ChildProcess;
}

describe("startAgentSidecar", () => {
  it("spawns the binary with the requested port and origins in its environment", async () => {
    const child = fakeChildProcess();
    const spawnImpl = vi.fn().mockReturnValue(child);
    const fetchImpl = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));

    const sidecar = await startAgentSidecar({
      binaryPath: "/opt/app/printers-agent",
      port: 19000,
      origins: ["http://localhost:5173"],
      spawnImpl,
      fetchImpl,
    });

    expect(spawnImpl).toHaveBeenCalledWith(
      "/opt/app/printers-agent",
      [],
      expect.objectContaining({ env: expect.objectContaining({ PRINTERS_AGENT_PORT: "19000", PRINTERS_AGENT_ORIGINS: "http://localhost:5173" }) }),
    );
    expect(sidecar.port).toBe(19000);
    expect(sidecar.process).toBe(child);
  });

  it("polls /health until it succeeds", async () => {
    const fetchImpl = vi
      .fn()
      .mockRejectedValueOnce(new Error("ECONNREFUSED"))
      .mockRejectedValueOnce(new Error("ECONNREFUSED"))
      .mockResolvedValueOnce(new Response(null, { status: 200 }));

    const sidecar = await startAgentSidecar({
      binaryPath: "/opt/app/printers-agent",
      spawnImpl: vi.fn().mockReturnValue(fakeChildProcess()),
      fetchImpl,
      healthTimeoutMs: 2000,
    });

    expect(fetchImpl).toHaveBeenCalledTimes(3);
    expect(sidecar.port).toBe(18999);
  });

  it("kills the process and throws if the agent never becomes healthy", async () => {
    const child = fakeChildProcess();
    const fetchImpl = vi.fn().mockResolvedValue(new Response(null, { status: 500 }));

    await expect(
      startAgentSidecar({
        binaryPath: "/opt/app/printers-agent",
        spawnImpl: vi.fn().mockReturnValue(child),
        fetchImpl,
        healthTimeoutMs: 150,
      }),
    ).rejects.toThrow(/did not become healthy/);

    expect(child.kill).toHaveBeenCalled();
  });

  it("stop() kills the child process", async () => {
    const child = fakeChildProcess();
    const sidecar = await startAgentSidecar({
      binaryPath: "/opt/app/printers-agent",
      spawnImpl: vi.fn().mockReturnValue(child),
      fetchImpl: vi.fn().mockResolvedValue(new Response(null, { status: 200 })),
    });
    sidecar.stop();
    expect(child.kill).toHaveBeenCalled();
  });
});
