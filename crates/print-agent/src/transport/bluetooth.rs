//! Bluetooth (classic RFCOMM/SPP) transport for desktop -- added per the
//! user's explicit request to support Bluetooth thermal printers from
//! Windows/macOS/Linux, not just mobile (see docs/ARCHITECTURE.md's
//! "Browser Bluetooth is BLE-only" note for the related browser-side gap).
//!
//! Implemented for Linux (via BlueZ's D-Bus API through the `bluer` crate)
//! and Windows (via a raw `AF_BTH`/`BTHPROTO_RFCOMM` Winsock socket -- the
//! same low-level mechanism BluetoothAPIs.h and every other Windows RFCOMM
//! client ultimately goes through, needing nothing beyond ws2_32.dll, which
//! ships on every Windows install). There is no comparably mature safe Rust
//! crate for classic Bluetooth RFCOMM on macOS (IOBluetooth) at time of
//! writing, so it still returns `NotImplemented` rather than a
//! silently-wrong implementation.
//!
//! Verified against real hardware 2026-07-27: a paired Y58 80mm thermal
//! printer over classic Bluetooth SPP, driven through the full HTTP agent
//! (POST /print-bluetooth -> this module), produced correct printed output
//! on Linux. Most SPP thermal printers listen on RFCOMM channel 1, hence the
//! default below (confirmed correct against that same printer). The Windows
//! path below has been verified to compile and to fail cleanly with a
//! specific WSA error code against an address with no listener, but not yet
//! against a real Windows-paired Bluetooth printer.

use crate::error::AgentError;

pub const DEFAULT_CHANNEL: u8 = 1;

/// Parses "AA:BB:CC:DD:EE:FF" into the big-endian-packed 48-bit form Windows'
/// `SOCKADDR_BTH.btAddr` expects (first octet in the string is the
/// most-significant byte).
#[cfg(target_os = "windows")]
fn parse_bth_addr(address: &str) -> Result<u64, AgentError> {
    let octets: Vec<u8> = address
        .split(':')
        .map(|part| u8::from_str_radix(part, 16))
        .collect::<Result<_, _>>()
        .map_err(|_| AgentError::TransportError(format!("invalid Bluetooth address: {address}")))?;
    if octets.len() != 6 {
        return Err(AgentError::TransportError(format!("invalid Bluetooth address: {address}")));
    }
    Ok(octets.iter().fold(0u64, |acc, &b| (acc << 8) | b as u64))
}

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

#[cfg(target_os = "windows")]
pub async fn send(address: &str, bytes: Vec<u8>) -> Result<(), AgentError> {
    let address = address.to_string();
    tokio::task::spawn_blocking(move || send_blocking(&address, &bytes))
        .await
        .map_err(|e| AgentError::TransportError(format!("Windows Bluetooth task panicked: {e}")))?
}

#[cfg(target_os = "windows")]
fn send_blocking(address: &str, bytes: &[u8]) -> Result<(), AgentError> {
    use windows_sys::Win32::Devices::Bluetooth::{AF_BTH, BTHPROTO_RFCOMM, SOCKADDR_BTH};
    use windows_sys::Win32::Networking::WinSock::{
        closesocket, connect, send as ws_send, socket, WSACleanup, WSAGetLastError, WSAStartup, INVALID_SOCKET, SOCKADDR, SOCKET_ERROR, SOCK_STREAM, WSADATA,
    };

    let bt_addr = parse_bth_addr(address)?;

    unsafe {
        let mut wsa_data: WSADATA = std::mem::zeroed();
        if WSAStartup(0x0202, &mut wsa_data) != 0 {
            return Err(AgentError::TransportError("WSAStartup failed".to_string()));
        }

        let result: Result<(), AgentError> = (|| {
            let sock = socket(AF_BTH as i32, SOCK_STREAM, BTHPROTO_RFCOMM as i32);
            if sock == INVALID_SOCKET {
                return Err(AgentError::TransportError(format!("Bluetooth socket() failed: WSA error {}", WSAGetLastError())));
            }

            let mut addr: SOCKADDR_BTH = std::mem::zeroed();
            addr.addressFamily = AF_BTH;
            addr.btAddr = bt_addr;
            addr.port = DEFAULT_CHANNEL as u32;

            if connect(sock, &addr as *const SOCKADDR_BTH as *const SOCKADDR, std::mem::size_of::<SOCKADDR_BTH>() as i32) == SOCKET_ERROR {
                let err = WSAGetLastError();
                closesocket(sock);
                return Err(AgentError::TransportError(format!("Bluetooth connect to {address} failed: WSA error {err}")));
            }

            let mut sent = 0usize;
            while sent < bytes.len() {
                let n = ws_send(sock, bytes[sent..].as_ptr(), (bytes.len() - sent) as i32, 0);
                if n == SOCKET_ERROR {
                    let err = WSAGetLastError();
                    closesocket(sock);
                    return Err(AgentError::TransportError(format!("Bluetooth write to {address} failed: WSA error {err}")));
                }
                sent += n as usize;
            }

            closesocket(sock);
            Ok(())
        })();

        WSACleanup();
        result
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub async fn send(_address: &str, _bytes: Vec<u8>) -> Result<(), AgentError> {
    Err(AgentError::NotImplemented(
        "Bluetooth printing is not yet implemented on this platform (only Linux/BlueZ and Windows so far) -- use the desktop agent's TCP/USB/Serial transports, or print from the mobile app".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_an_invalid_bluetooth_address() {
        // On Linux and Windows this fails at address parsing before ever
        // touching BlueZ/Winsock; on macOS it's NotImplemented either way.
        let result = send("not-a-mac-address", vec![1, 2, 3]).await;
        assert!(result.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_a_bluetooth_address_into_sockaddr_bths_big_endian_btaddr() {
        // First octet in the string must land as the most-significant byte,
        // matching how Windows samples build BTH_ADDR from a display address.
        assert_eq!(parse_bth_addr("AA:BB:CC:DD:EE:FF").unwrap(), 0xAABBCCDDEEFFu64);
        assert_eq!(parse_bth_addr("00:00:00:00:00:01").unwrap(), 1u64);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rejects_a_bluetooth_address_with_the_wrong_number_of_octets() {
        assert!(parse_bth_addr("AA:BB:CC").is_err());
        assert!(parse_bth_addr("AA:BB:CC:DD:EE:FF:00").is_err());
    }
}
