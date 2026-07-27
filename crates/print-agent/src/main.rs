use printers_print_agent::{auth, build_app, state::AppState, DEFAULT_PORT};
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main]
async fn main() -> ExitCode {
    let port: u16 = std::env::var("PRINTERS_AGENT_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_PORT);
    let addr = format!("127.0.0.1:{port}");

    // A bind failure here is a routine, expected-in-the-wild condition (most
    // commonly: the agent is already running), not a bug -- report it
    // plainly and exit instead of panicking with a stack trace. See
    // docs/ARCHITECTURE.md: QZ Tray's WebsocketPorts class treats a taken
    // port as normal operating conditions, not a crash.
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "printers-agent: port {port} is already in use -- is another instance of the agent already running? \
(set PRINTERS_AGENT_PORT to use a different port)"
            );
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("printers-agent: failed to bind {addr}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let state = Arc::new(AppState::new(auth::allowed_origins()));
    let app = build_app(state);
    println!("printers-agent listening on http://{addr} (see docs/PROTOCOL.md)");
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("printers-agent: server error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
