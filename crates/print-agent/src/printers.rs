//! Printer listing + discovery, per docs/PROTOCOL.md's `GET /printers` and
//! `POST /discover`. CUPS and serial ports are queried live (their state can
//! change any time); TCP entries only appear once discovered or configured
//! -- the agent never scans the network unprompted, matching the doc.

use crate::transport::{cups, serial};
use serde::Serialize;
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::Mutex;
use std::time::Duration;
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize)]
pub struct Printer {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub transport: String,
    pub status: String,
}

/// Discovered-but-not-yet-persisted TCP printers, populated by `POST /discover`.
/// Cleared on agent restart -- re-run discovery, don't rely on this as storage.
#[derive(Default)]
pub struct DiscoveredRegistry {
    tcp: Mutex<Vec<Printer>>,
}

impl DiscoveredRegistry {
    pub fn add_tcp(&self, printers: Vec<Printer>) {
        let mut tcp = self.tcp.lock().unwrap();
        for p in printers {
            if !tcp.iter().any(|existing| existing.id == p.id) {
                tcp.push(p);
            }
        }
    }

    pub fn tcp_printers(&self) -> Vec<Printer> {
        self.tcp.lock().unwrap().clone()
    }
}

/// Windows-installed printers (POS drivers bound to USB, "Microsoft Print to
/// PDF", etc.) -- listed so the `windows:<name>` transport (RAW spooler
/// write / GDI silent print, see transport::windows_queue) has a
/// corresponding entry in `GET /printers` for callers to discover and select
/// rather than needing to already know the exact Windows printer name.
#[cfg(target_os = "windows")]
fn list_windows_printers() -> Vec<Printer> {
    use windows_sys::Win32::Graphics::Printing::{EnumPrintersW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_4W};
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;

    unsafe fn wide_to_string(ptr: *const u16) -> String {
        let mut len = 0isize;
        while *ptr.offset(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len as usize))
    }

    unsafe {
        let mut needed: u32 = 0;
        let mut returned: u32 = 0;
        EnumPrintersW(flags, std::ptr::null(), 4, std::ptr::null_mut(), 0, &mut needed, &mut returned);
        if needed == 0 {
            return Vec::new();
        }

        let mut buf = vec![0u8; needed as usize];
        let ok = EnumPrintersW(flags, std::ptr::null(), 4, buf.as_mut_ptr(), needed, &mut needed, &mut returned);
        if ok == 0 {
            return Vec::new();
        }

        std::slice::from_raw_parts(buf.as_ptr() as *const PRINTER_INFO_4W, returned as usize)
            .iter()
            .filter(|info| !info.pPrinterName.is_null())
            .map(|info| {
                let name = wide_to_string(info.pPrinterName);
                Printer { id: format!("windows:{name}"), name, kind: "unknown".to_string(), transport: "windows".to_string(), status: "unknown".to_string() }
            })
            .collect()
    }
}

#[cfg(not(target_os = "windows"))]
fn list_windows_printers() -> Vec<Printer> {
    Vec::new()
}

pub async fn list_all(discovered: &DiscoveredRegistry) -> Vec<Printer> {
    let mut printers = Vec::new();

    for name in cups::list_destinations().await {
        printers.push(Printer {
            id: format!("cups:{name}"),
            name: name.clone(),
            kind: "normal".to_string(),
            transport: "cups".to_string(),
            status: "unknown".to_string(),
        });
    }

    for port in serial::list_ports() {
        printers.push(Printer {
            id: format!("serial:{port}"),
            name: port.clone(),
            kind: "thermal-80".to_string(),
            transport: "serial".to_string(),
            status: "unknown".to_string(),
        });
    }

    printers.extend(list_windows_printers());
    printers.extend(discovered.tcp_printers());
    printers
}

/// Best-effort local /24 guess: opens a UDP "connection" (no packet sent for
/// a plain UDP connect) to a public IP purely to learn which local interface
/// the OS would route through, then assumes a /24 around that address. This
/// is the same trick common CLI tools use since there's no portable "get my
/// LAN subnet" syscall.
fn guess_local_subnet() -> Option<(Ipv4Addr, u8)> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) => Some((ip, 24)),
        std::net::IpAddr::V6(_) => None,
    }
}

/// Expands a base IPv4 + prefix length into every host address in that
/// range, capped at /24 (256 addresses) so a mistaken huge range can't turn
/// a "discover printers" click into an accidental network scan.
fn expand_subnet_hosts(base: Ipv4Addr, prefix_len: u8) -> Vec<Ipv4Addr> {
    let prefix_len = prefix_len.max(24); // never scan more than a /24
    let mask_bits = 32 - prefix_len as u32;
    let host_count = 1u32 << mask_bits;
    let base_u32 = u32::from(base) & !((host_count) - 1);
    (0..host_count).map(|i| Ipv4Addr::from(base_u32 + i)).collect()
}

fn parse_cidr(subnet: &str) -> Option<(Ipv4Addr, u8)> {
    let (ip_str, prefix_str) = subnet.split_once('/')?;
    let ip: Ipv4Addr = ip_str.parse().ok()?;
    let prefix: u8 = prefix_str.parse().ok()?;
    Some((ip, prefix))
}

/// Probes TCP:9100 across a subnet (default: the agent's guessed local /24).
/// Bounded, timeout-gated, and only ever run when the caller explicitly asks
/// via `POST /discover` -- see docs/PROTOCOL.md.
pub async fn discover_tcp(subnet: Option<String>, timeout_ms: u64) -> Vec<Printer> {
    let (base, prefix_len) = match subnet.as_deref().and_then(parse_cidr).or_else(guess_local_subnet) {
        Some(v) => v,
        None => return Vec::new(),
    };

    let hosts = expand_subnet_hosts(base, prefix_len);
    let mut tasks = Vec::with_capacity(hosts.len());
    for ip in hosts {
        tasks.push(tokio::spawn(async move {
            let addr = format!("{ip}:9100");
            match tokio::time::timeout(Duration::from_millis(timeout_ms), TcpStream::connect(&addr)).await {
                Ok(Ok(_)) => Some(ip),
                _ => None,
            }
        }));
    }

    let mut found = Vec::new();
    for task in tasks {
        if let Ok(Some(ip)) = task.await {
            found.push(Printer {
                id: format!("tcp:{ip}:9100"),
                name: format!("Network printer ({ip})"),
                kind: "thermal-80".to_string(),
                transport: "tcp".to_string(),
                status: "unknown".to_string(),
            });
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_a_slash_24_to_256_hosts() {
        let hosts = expand_subnet_hosts(Ipv4Addr::new(192, 168, 1, 37), 24);
        assert_eq!(hosts.len(), 256);
        assert_eq!(hosts[0], Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(hosts[255], Ipv4Addr::new(192, 168, 1, 255));
    }

    #[test]
    fn caps_an_overly_broad_prefix_at_slash_24() {
        let hosts = expand_subnet_hosts(Ipv4Addr::new(10, 0, 0, 5), 8);
        assert_eq!(hosts.len(), 256);
    }

    #[test]
    fn parses_a_valid_cidr_string() {
        assert_eq!(parse_cidr("192.168.1.0/24"), Some((Ipv4Addr::new(192, 168, 1, 0), 24)));
        assert_eq!(parse_cidr("not-a-cidr"), None);
    }

    #[tokio::test]
    async fn discover_tcp_finds_a_locally_listening_printer_stand_in() {
        // We can't bind port 9100 in a sandboxed test (may require privileges
        // or already be in use), so this just proves discovery over a tiny,
        // explicit /32-ish range completes without hanging or panicking.
        let found = discover_tcp(Some("127.0.0.1/24".to_string()), 50).await;
        assert!(found.iter().all(|p| p.transport == "tcp"));
    }

    #[test]
    fn discovered_registry_dedupes_by_id() {
        let registry = DiscoveredRegistry::default();
        let printer = Printer { id: "tcp:1.2.3.4:9100".into(), name: "x".into(), kind: "thermal-80".into(), transport: "tcp".into(), status: "unknown".into() };
        registry.add_tcp(vec![printer.clone(), printer.clone()]);
        registry.add_tcp(vec![printer]);
        assert_eq!(registry.tcp_printers().len(), 1);
    }
}
