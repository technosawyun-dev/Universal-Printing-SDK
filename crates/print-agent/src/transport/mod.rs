//! Transport dispatch. A `printerId` (from `GET /printers`) is
//! `"<transport>:<address>"`; `send_to_printer` parses the prefix and routes
//! to the matching module. Direct endpoints (`/print-tcp`, `/print-serial`,
//! `/print-bluetooth`) skip the printerId lookup and call a transport module
//! directly, matching docs/PROTOCOL.md.

pub mod bluetooth;
pub mod cups;
pub mod serial;
pub mod tcp;
pub mod windows_queue;

use crate::error::AgentError;

pub async fn send_to_printer(printer_id: &str, bytes: Vec<u8>) -> Result<(), AgentError> {
    let (kind, address) = printer_id
        .split_once(':')
        .ok_or_else(|| AgentError::PrinterNotFound(format!("malformed printerId '{printer_id}'")))?;
    match kind {
        "tcp" => {
            let (host, port) = address.rsplit_once(':').ok_or_else(|| AgentError::PrinterNotFound(format!("malformed tcp printerId '{printer_id}'")))?;
            let port: u16 = port.parse().map_err(|_| AgentError::PrinterNotFound(format!("malformed tcp port in '{printer_id}'")))?;
            tcp::send(host, port, &bytes).await
        }
        "serial" => serial::send(address, 9600, bytes).await,
        "bt" => bluetooth::send(address, bytes).await,
        "windows" => windows_queue::send_raw(address, bytes).await,
        "cups" => cups::send_raw(address, &bytes).await,
        _ => Err(AgentError::PrinterNotFound(format!("unknown transport '{kind}' in printerId '{printer_id}'"))),
    }
}

/// Silent (no-dialog) print of a PrintDocument to a "normal" (a4/a3/letter)
/// printer, for `POST /print-silent`. Takes both the pre-rendered PDF bytes
/// (used by the CUPS path, which natively understands PDF) and the document
/// itself (used by the Windows GDI path, which draws it directly -- see
/// windows_queue.rs -- rather than needing a bundled PDF-rasterization
/// engine).
pub async fn print_silent(printer_id: &str, doc: &printers_core::PrintDocument, pdf_bytes: &[u8], copies: u32) -> Result<(), AgentError> {
    let (kind, address) = printer_id
        .split_once(':')
        .ok_or_else(|| AgentError::PrinterNotFound(format!("malformed printerId '{printer_id}'")))?;
    match kind {
        "cups" => cups::print_pdf(address, pdf_bytes, copies).await,
        "windows" => windows_queue::print_document_gdi(address, doc, copies).await,
        _ => Err(AgentError::PrinterNotFound(format!("'{kind}' is not a normal-printer transport"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_a_printer_id_with_no_transport_prefix() {
        let result = send_to_printer("no-colon-here", vec![]).await;
        assert!(matches!(result, Err(AgentError::PrinterNotFound(_))));
    }

    #[tokio::test]
    async fn rejects_an_unknown_transport_prefix() {
        let result = send_to_printer("carrier-pigeon:loft-1", vec![]).await;
        assert!(matches!(result, Err(AgentError::PrinterNotFound(_))));
    }

    #[tokio::test]
    async fn dispatches_tcp_printer_ids_to_the_tcp_transport() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 3];
            tokio::io::AsyncReadExt::read_exact(&mut socket, &mut buf).await.unwrap();
            buf
        });
        let printer_id = format!("tcp:{}:{}", addr.ip(), addr.port());
        send_to_printer(&printer_id, vec![1, 2, 3]).await.unwrap();
        assert_eq!(server.await.unwrap(), vec![1, 2, 3]);
    }
}
