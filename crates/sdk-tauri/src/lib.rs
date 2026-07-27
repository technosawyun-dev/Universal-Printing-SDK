//! Example Tauri v2 plugin wrapping `core-rs`. Unlike `crates/print-agent`,
//! a Tauri app embeds `core-rs` directly in its own Rust binary -- no
//! separate agent process, no HTTP round trip (see docs/ARCHITECTURE.md).
//! A real app registers this with `.plugin(printers_sdk_tauri::init())` in
//! its `tauri::Builder`, then calls these as regular Tauri commands from
//! the webview via `invoke("plugin:printers-sdk|render_to_escpos", ...)`.
//!
//! This crate only wraps rendering (bytes/HTML/text out); it deliberately
//! does not open a delivery transport itself -- an app author chooses how to
//! get bytes to a printer (Tauri's own shell/process plugin for a local
//! agent-less transport, or wiring up USB/serial/CUPS calls directly) since
//! that's app-specific, not something to bake into the wrapper.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[tauri::command]
fn render_to_escpos(document_json: String) -> Result<Vec<u8>, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_escpos(&doc).map_err(|e| e.to_string())
}

#[tauri::command]
fn render_to_html(document_json: String) -> Result<String, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_html(&doc).map_err(|e| e.to_string())
}

#[tauri::command]
fn render_to_svg(document_json: String) -> Result<String, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_svg(&doc).map_err(|e| e.to_string())
}

#[tauri::command]
fn render_to_plain_text(document_json: String) -> Result<String, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    Ok(printers_core::to_plain_text(&doc))
}

#[tauri::command]
fn render_to_pdf(document_json: String) -> Result<Vec<u8>, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_pdf(&doc).map_err(|e| e.to_string())
}

#[tauri::command]
fn validate_document(document_json: String) -> Result<(), String> {
    printers_core::validate_document(&document_json).map(|_| ()).map_err(|e| e.to_string())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("printers-sdk")
        .invoke_handler(tauri::generate_handler![
            render_to_escpos,
            render_to_html,
            render_to_svg,
            render_to_plain_text,
            render_to_pdf,
            validate_document,
        ])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_DOC: &str = r#"{"version":"1.0","paper":{"kind":"thermal-80"},"content":[{"type":"text","value":"hi"},{"type":"cut"}]}"#;
    const INVALID_DOC: &str = r#"{"version":"1.0","paper":{"kind":"custom"},"content":[]}"#;

    #[test]
    fn renders_a_valid_document_through_every_command() {
        assert!(render_to_escpos(VALID_DOC.to_string()).unwrap().starts_with(&[0x1b, 0x40]));
        assert!(render_to_html(VALID_DOC.to_string()).unwrap().contains("hi"));
        assert!(render_to_svg(VALID_DOC.to_string()).unwrap().starts_with("<svg"));
        assert_eq!(render_to_plain_text(VALID_DOC.to_string()).unwrap(), "hi");
        assert!(render_to_pdf(VALID_DOC.to_string()).unwrap().starts_with(b"%PDF"));
        assert!(validate_document(VALID_DOC.to_string()).is_ok());
    }

    #[test]
    fn rejects_an_invalid_document_before_rendering() {
        assert!(render_to_escpos(INVALID_DOC.to_string()).is_err());
        assert!(validate_document(INVALID_DOC.to_string()).is_err());
    }
}
