// Spawns crates/print-agent as a sidecar child process from Electron's main
// process, and waits for it to become healthy before handing back control.
// See docs/ARCHITECTURE.md: sdk-electron bundles the agent binary rather
// than talking to hardware directly, unlike sdk-web.

import { spawn as nodeSpawn, type ChildProcess, type SpawnOptions } from "node:child_process";

const DEFAULT_PORT = 18999;
const DEFAULT_HEALTH_TIMEOUT_MS = 5000;
const HEALTH_POLL_INTERVAL_MS = 100;

export interface AgentSidecarOptions {
  /** Path to the platform-specific `printers-agent` binary bundled with the app. */
  binaryPath: string;
  port?: number;
  /** Passed through as PRINTERS_AGENT_ORIGINS -- see docs/PROTOCOL.md's Auth section. */
  origins?: string[];
  healthTimeoutMs?: number;
  /** Injectable for testing. */
  fetchImpl?: typeof fetch;
  /** Injectable for testing. */
  spawnImpl?: typeof nodeSpawn;
}

export interface AgentSidecar {
  process: ChildProcess;
  port: number;
  stop(): void;
}

async function waitForHealth(port: number, timeoutMs: number, fetchImpl: typeof fetch): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      const res = await fetchImpl(`http://127.0.0.1:${port}/health`, { headers: { Origin: "http://localhost" } });
      if (res.ok) return;
    } catch (e) {
      lastError = e;
    }
    await new Promise((resolve) => setTimeout(resolve, HEALTH_POLL_INTERVAL_MS));
  }
  const suffix = lastError ? `: ${lastError instanceof Error ? lastError.message : String(lastError)}` : "";
  throw new Error(`print-agent did not become healthy within ${timeoutMs}ms${suffix}`);
}

/** Spawns the agent binary and resolves once `/health` responds -- rejects (and kills the process) if it never does. */
export async function startAgentSidecar(options: AgentSidecarOptions): Promise<AgentSidecar> {
  const port = options.port ?? DEFAULT_PORT;
  const spawnImpl = options.spawnImpl ?? nodeSpawn;
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;

  const env: NodeJS.ProcessEnv = { ...process.env, PRINTERS_AGENT_PORT: String(port) };
  if (options.origins?.length) env.PRINTERS_AGENT_ORIGINS = options.origins.join(",");

  const spawnOptions: SpawnOptions = { env, stdio: "pipe" };
  const child = spawnImpl(options.binaryPath, [], spawnOptions);

  try {
    await waitForHealth(port, options.healthTimeoutMs ?? DEFAULT_HEALTH_TIMEOUT_MS, fetchImpl);
  } catch (err) {
    child.kill();
    throw err;
  }

  return {
    process: child,
    port,
    stop: () => child.kill(),
  };
}
