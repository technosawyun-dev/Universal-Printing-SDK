//! Bluetooth (classic RFCOMM/SPP) transport for desktop -- added per the
//! user's explicit request to support Bluetooth thermal printers from
//! Windows/macOS/Linux, not just mobile (see docs/ARCHITECTURE.md's
//! "Browser Bluetooth is BLE-only" note for the related browser-side gap).
//!
//! Only implemented for Linux here, via BlueZ's D-Bus API through the `bluer`
//! crate -- there is no comparably mature safe Rust crate for classic
//! Bluetooth RFCOMM on macOS (IOBluetooth) or Windows (Win32 Bluetooth APIs)
//! at time of writing. macOS/Windows return `NotImplemented` rather than a
//! silently-wrong implementation.
//!
//! Verified against real hardware 2026-07-27: a paired Y58 80mm thermal
//! printer over classic Bluetooth SPP, driven through the full HTTP agent
//! (POST /print-bluetooth -> this module), produced correct printed output.
//! Most SPP thermal printers listen on RFCOMM channel 1, hence the default
//! below (confirmed correct against that same printer).

use crate::error::AgentError;

pub const DEFAULT_CHANNEL: u8 = 1;

#[cfg(target_os = "linux")]
pub async fn send(address: &str, bytes: Vec<u8>) -> Result<(), AgentError> {
    use bluer::rfcomm::{SocketAddr, Stream};
    use bluer::Address;
    use tokio::io::AsyncWriteExt;

    let addr: Address = address.parse().map_err(|_| AgentError::TransportError(format!("invalid Bluetooth address: {address}")))?;
    let session = bluer::Session::new().await.map_err(|e| AgentError::TransportError(format!("failed to start BlueZ session: {e}")))?;
    let adapter = session.default_adapter().await.map_err(|e| AgentError::TransportError(format!("no Bluetooth adapter: {e}")))?;
    adapter.set_powered(true).await.map_err(|e| AgentError::TransportError(format!("failed to power on Bluetooth adapter: {e}")))?;

    let target = SocketAddr::new(addr, DEFAULT_CHANNEL);
    let mut stream = Stream::connect(target).await.map_err(|e| AgentError::TransportError(format!("Bluetooth connect to {address} failed: {e}")))?;
    stream.write_all(&bytes).await.map_err(|e| AgentError::TransportError(format!("Bluetooth write to {address} failed: {e}")))?;
    stream.flush().await.map_err(|e| AgentError::TransportError(format!("Bluetooth flush to {address} failed: {e}")))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn send(_address: &str, _bytes: Vec<u8>) -> Result<(), AgentError> {
    Err(AgentError::NotImplemented(
        "Bluetooth printing is not yet implemented on this platform (only Linux/BlueZ so far) -- use the desktop agent's TCP/USB/Serial transports, or print from the mobile app".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_an_invalid_bluetooth_address() {
        // On Linux this fails at address parsing before ever touching BlueZ;
        // on other platforms it's NotImplemented either way.
        let result = send("not-a-mac-address", vec![1, 2, 3]).await;
        assert!(result.is_err());
    }
}
