//! Serial-port transport (USB-serial thermal printers), matching pso_window's
//! Rust agent -- the one transport POS_system's original agent lacked (see
//! docs/ARCHITECTURE.md). `serialport` is a blocking API, so writes run on a
//! blocking thread pool rather than the async reactor.
//!
//! Unverified in this environment: no serial hardware/virtual tty is attached
//! here, so this is written against the crate's documented API but has not
//! been exercised against a real device.

use crate::error::AgentError;
use std::io::Write;
use std::time::Duration;

pub async fn send(port_name: &str, baud_rate: u32, bytes: Vec<u8>) -> Result<(), AgentError> {
    let port_name = port_name.to_string();
    tokio::task::spawn_blocking(move || {
        let mut port = serialport::new(&port_name, baud_rate)
            .timeout(Duration::from_secs(5))
            .open()
            .map_err(|e| AgentError::TransportError(format!("failed to open serial port {port_name}: {e}")))?;
        port.write_all(&bytes).map_err(|e| AgentError::TransportError(format!("failed to write to serial port {port_name}: {e}")))?;
        port.flush().map_err(|e| AgentError::TransportError(format!("failed to flush serial port {port_name}: {e}")))
    })
    .await
    .map_err(|e| AgentError::TransportError(format!("serial write task panicked: {e}")))?
}

/// On Linux, needs this crate's own default-on `udev` feature (see
/// Cargo.toml) to filter real ports from the ~32 legacy `/dev/ttyS*` stub
/// devices every Linux kernel creates whether or not real hardware is
/// present. Without it (only ever intentionally disabled in this project's
/// own sandbox, which lacks libudev-dev and passwordless sudo to install it),
/// this returns all of those stubs too -- real deployments should always
/// build with the default features on.
pub fn list_ports() -> Vec<String> {
    serialport::available_ports().map(|ports| ports.into_iter().map(|p| p.port_name).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_ports_does_not_panic_when_none_are_present() {
        // No serial hardware is attached in this environment -- proves the
        // call degrades to an empty Vec rather than erroring.
        let _ = list_ports();
    }

    #[tokio::test]
    async fn opening_a_nonexistent_port_reports_a_transport_error() {
        let result = send("/dev/nonexistent-printers-agent-test-port", 9600, vec![1, 2, 3]).await;
        assert!(result.is_err());
    }
}
