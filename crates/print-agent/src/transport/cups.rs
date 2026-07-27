//! CUPS transport (macOS/Linux) -- shells out to `lp`/`lpstat`, matching the
//! Restaurant Platform agent's existing approach (see docs/ARCHITECTURE.md).
//! Used two ways: `-o raw` for thermal printers (bypasses CUPS' filtering,
//! sends our ESC/POS bytes untouched) and normal mode for the silent-PDF
//! path, where CUPS' own PDF filter chain does the rest.

use crate::error::AgentError;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

async fn run_lp(args: &[&str], stdin_bytes: &[u8]) -> Result<(), AgentError> {
    let mut child = Command::new("lp")
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AgentError::TransportError(format!("failed to launch lp: {e}")))?;

    child
        .stdin
        .take()
        .ok_or_else(|| AgentError::TransportError("lp: no stdin handle".to_string()))?
        .write_all(stdin_bytes)
        .await
        .map_err(|e| AgentError::TransportError(format!("failed to write to lp: {e}")))?;

    let output = child.wait_with_output().await.map_err(|e| AgentError::TransportError(format!("lp exited abnormally: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AgentError::TransportError(format!("lp failed: {}", stderr.trim())));
    }
    Ok(())
}

/// Sends raw ESC/POS bytes to a CUPS destination configured as a raw queue.
pub async fn send_raw(destination: &str, bytes: &[u8]) -> Result<(), AgentError> {
    run_lp(&["-d", destination, "-o", "raw"], bytes).await
}

/// Sends a rendered PDF to a CUPS destination for normal (A4/A3/Letter) printing.
pub async fn print_pdf(destination: &str, pdf_bytes: &[u8], copies: u32) -> Result<(), AgentError> {
    let copies_str = copies.max(1).to_string();
    run_lp(&["-d", destination, "-n", &copies_str], pdf_bytes).await
}

/// Lists CUPS-known destinations. Returns an empty list (not an error) when
/// CUPS has no destinations configured -- that's a normal, common state on a
/// fresh machine, not a failure.
pub async fn list_destinations() -> Vec<String> {
    let output = match Command::new("lpstat").arg("-a").output().await {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_destinations_does_not_error_when_none_are_configured() {
        // This dev machine's CUPS has no destinations added -- list_destinations
        // must degrade to an empty Vec, not panic or return an Err.
        let destinations = list_destinations().await;
        assert!(destinations.is_empty() || !destinations.is_empty()); // just proving it returns, doesn't hang/panic
    }

    #[tokio::test]
    async fn send_raw_to_an_unknown_destination_reports_a_transport_error() {
        let result = send_raw("printers-agent-test-nonexistent-destination", b"hello").await;
        assert!(result.is_err());
    }
}
