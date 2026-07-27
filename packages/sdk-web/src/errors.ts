/** Mirrors the agent's error shape from docs/PROTOCOL.md: `{ error: { code, message } }`. */
export class AgentClientError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "AgentClientError";
  }
}

/** Thrown when a browser API this transport depends on (WebUSB, Web Serial, Web Bluetooth) isn't available. */
export class UnsupportedTransportError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UnsupportedTransportError";
  }
}
