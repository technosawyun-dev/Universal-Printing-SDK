//! FFI surface over `printers-core`, exported via uniffi so Android (Kotlin)
//! and iOS (Swift) can call the same document-rendering logic that
//! `crates/print-agent` uses on desktop, without needing a loopback agent
//! process — see docs/ARCHITECTURE.md ("Why the agent can't reach mobile").
//!
//! The FFI boundary trades in JSON strings (matching docs/DOCUMENT_FORMAT.md)
//! rather than native structs, so this crate doesn't need to mirror every
//! type across three languages — Kotlin/Swift/Dart build the document as
//! JSON (or via a thin typed builder in sdk-flutter) and this layer just
//! deserializes, renders, and hands back bytes/strings.

use printers_core::PrintDocument;

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum RenderError {
    #[error("invalid PrintDocument JSON: {message}")]
    InvalidDocument { message: String },
    #[error("render error: {message}")]
    Render { message: String },
}

impl From<printers_core::CoreError> for RenderError {
    fn from(err: printers_core::CoreError) -> Self {
        match err {
            printers_core::CoreError::InvalidDocument(message) => RenderError::InvalidDocument { message },
            printers_core::CoreError::Render(message) => RenderError::Render { message },
        }
    }
}

/// Parses AND validates (cross-field rules like "custom paper needs widthMm",
/// not just JSON shape) via `printers_core::validate_document`, so every FFI
/// entry point rejects the same malformed documents core-ts's callers would.
fn parse(document_json: &str) -> Result<PrintDocument, RenderError> {
    Ok(printers_core::validate_document(document_json)?)
}

/// Render a PrintDocument (as JSON, see docs/DOCUMENT_FORMAT.md) to ESC/POS
/// bytes for a thermal-58/thermal-80 printer.
#[uniffi::export]
pub fn render_to_escpos(document_json: String) -> Result<Vec<u8>, RenderError> {
    let doc = parse(&document_json)?;
    Ok(printers_core::to_escpos(&doc)?)
}

/// Render a PrintDocument to self-contained HTML, for an in-app preview.
#[uniffi::export]
pub fn render_to_html(document_json: String) -> Result<String, RenderError> {
    let doc = parse(&document_json)?;
    Ok(printers_core::to_html(&doc)?)
}

/// Render a PrintDocument to plain text, for debug/fallback display.
#[uniffi::export]
pub fn render_to_plain_text(document_json: String) -> Result<String, RenderError> {
    let doc = parse(&document_json)?;
    Ok(printers_core::to_plain_text(&doc))
}

/// Render a PrintDocument to a PDF, for AirPrint (iOS) / Android's
/// PrintManager on the "normal printer" (a4/a3/letter) path.
#[uniffi::export]
pub fn render_to_pdf(document_json: String) -> Result<Vec<u8>, RenderError> {
    let doc = parse(&document_json)?;
    Ok(printers_core::to_pdf(&doc)?)
}

/// Validate a document without rendering it — lets the app surface a
/// friendly error before attempting to print.
#[uniffi::export]
pub fn validate_document(document_json: String) -> Result<(), RenderError> {
    parse(&document_json).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_DOC: &str = r#"{"version":"1.0","paper":{"kind":"thermal-80"},"content":[{"type":"text","value":"hi"},{"type":"cut"}]}"#;
    const INVALID_DOC: &str = r#"{"version":"1.0","paper":{"kind":"custom"},"content":[]}"#;

    #[test]
    fn renders_a_valid_document_through_every_entry_point() {
        assert!(render_to_escpos(VALID_DOC.to_string()).unwrap().starts_with(&[0x1b, 0x40]));
        assert!(render_to_html(VALID_DOC.to_string()).unwrap().contains("hi"));
        assert_eq!(render_to_plain_text(VALID_DOC.to_string()).unwrap(), "hi");
        assert!(render_to_pdf(VALID_DOC.to_string()).unwrap().starts_with(b"%PDF"));
        assert!(validate_document(VALID_DOC.to_string()).is_ok());
    }

    #[test]
    fn rejects_an_invalid_document_before_rendering() {
        assert!(matches!(render_to_escpos(INVALID_DOC.to_string()), Err(RenderError::InvalidDocument { .. })));
        assert!(matches!(validate_document(INVALID_DOC.to_string()), Err(RenderError::InvalidDocument { .. })));
    }
}
