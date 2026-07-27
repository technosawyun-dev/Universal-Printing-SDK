// Standalone Tauri v2 desktop app proving the SDK's "embedded" integration
// pattern: no separate print-agent process, no HTTP round trip. The webview
// calls the printers-sdk-tauri plugin (via invoke) to render a PrintDocument
// to ESC/POS bytes inside this app's own Rust binary, then calls one of the
// two commands below -- also running inside this same binary -- to actually
// deliver those bytes to a printer. This is the pattern a real desktop app
// (e.g. Global Link) would use instead of running printers-agent.exe
// alongside itself.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use printers_print_agent::transport::{tcp, windows_queue};

#[tauri::command]
async fn send_tcp(host: String, port: u16, bytes: Vec<u8>) -> Result<(), String> {
    tcp::send(&host, port, &bytes).await.map_err(|e| e.message().to_string())
}

/// On non-Windows this always returns a clear NOT_IMPLEMENTED error (see
/// crates/print-agent/src/transport/windows_queue.rs) -- expected when
/// running this app on Linux/macOS for development. On Windows it sends raw
/// bytes to a printer already installed in the Windows print queue.
#[tauri::command]
async fn send_windows(printer_name: String, bytes: Vec<u8>) -> Result<(), String> {
    windows_queue::send_raw(&printer_name, bytes).await.map_err(|e| e.message().to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(printers_sdk_tauri::init())
        .invoke_handler(tauri::generate_handler![send_tcp, send_windows])
        .run(tauri::generate_context!())
        .expect("error while running printers-tauri-test");
}
