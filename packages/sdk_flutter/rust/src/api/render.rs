//! flutter_rust_bridge API surface over `printers-core` (see
//! docs/ARCHITECTURE.md). Unlike `crates/core-ffi` (which uses uniffi, aimed
//! at native Kotlin/Swift apps), this crate is what `packages/sdk_flutter`
//! itself binds against -- uniffi's Dart target is unofficial/experimental,
//! whereas flutter_rust_bridge is the de facto standard for Flutter+Rust.
//! Both wrap the same `printers-core` and stay in sync by construction: no
//! logic is duplicated here, only the JSON-in/bytes-or-string-out boundary.
//!
//! All functions are `#[frb(sync)]` -- rendering is pure CPU-bound work, no
//! I/O, so there's no reason to burn a Dart isolate hop on it.

use flutter_rust_bridge::frb;

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Render a PrintDocument (as JSON, see docs/DOCUMENT_FORMAT.md) to ESC/POS
/// bytes for a thermal-58/thermal-80 printer.
#[frb(sync)]
pub fn render_to_escpos(document_json: String) -> Result<Vec<u8>, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_escpos(&doc).map_err(|e| e.to_string())
}

/// Render a PrintDocument to self-contained HTML, for an in-app preview.
#[frb(sync)]
pub fn render_to_html(document_json: String) -> Result<String, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_html(&doc).map_err(|e| e.to_string())
}

/// Render a PrintDocument to SVG, for a lightweight thumbnail preview.
#[frb(sync)]
pub fn render_to_svg(document_json: String) -> Result<String, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_svg(&doc).map_err(|e| e.to_string())
}

/// Render a PrintDocument to plain text, for debug/fallback display.
#[frb(sync)]
pub fn render_to_plain_text(document_json: String) -> Result<String, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    Ok(printers_core::to_plain_text(&doc))
}

/// Render a PrintDocument to a PDF, for AirPrint (iOS) / Android's
/// PrintManager on the "normal printer" (a4/a3/letter) path.
#[frb(sync)]
pub fn render_to_pdf(document_json: String) -> Result<Vec<u8>, String> {
    let doc = printers_core::validate_document(&document_json).map_err(|e| e.to_string())?;
    printers_core::to_pdf(&doc).map_err(|e| e.to_string())
}

/// Validate a document without rendering it -- lets the app surface a
/// friendly error before attempting to print.
#[frb(sync)]
pub fn validate_document(document_json: String) -> Result<(), String> {
    printers_core::validate_document(&document_json).map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_DOC: &str = r#"{"version":"1.0","paper":{"kind":"thermal-80"},"content":[{"type":"text","value":"hi"},{"type":"cut"}]}"#;
    const INVALID_DOC: &str = r#"{"version":"1.0","paper":{"kind":"custom"},"content":[]}"#;

    #[test]
    fn renders_a_valid_document_through_every_function() {
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
