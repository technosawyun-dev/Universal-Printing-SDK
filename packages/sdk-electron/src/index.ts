export * from "./agent-sidecar.js";
export * from "./native-print.js";
// Re-exported so a main-process caller can talk to the sidecar it just
// started without a separate dependency on sdk-web.
export { AgentClient } from "@printers/sdk-web";
export type { AgentClientOptions, AgentEvent, AgentPrinter, JobState, JobStatus } from "@printers/sdk-web";
