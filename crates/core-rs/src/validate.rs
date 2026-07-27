//! Mirrors `packages/core-ts/src/validate.ts`. serde's own deserialization
//! already rejects most shape problems (unknown node `type`, missing
//! required fields, wrong JSON types) -- this only adds the handful of
//! cross-field rules serde's derive can't express.

use crate::types::{PaperKind, PrintDocument};
use crate::CoreError;

/// Parses and validates a PrintDocument from JSON. Returns `CoreError::InvalidDocument`
/// on any structural or cross-field problem.
pub fn validate_document(json: &str) -> Result<PrintDocument, CoreError> {
    let doc: PrintDocument =
        serde_json::from_str(json).map_err(|e| CoreError::InvalidDocument(e.to_string()))?;

    if doc.version != "1.0" {
        return Err(CoreError::InvalidDocument(format!(
            "unsupported version '{}', expected '1.0'",
            doc.version
        )));
    }
    if doc.paper.kind == PaperKind::Custom && doc.paper.width_mm.is_none() {
        return Err(CoreError::InvalidDocument(
            "paper.widthMm is required when paper.kind is 'custom'".to_string(),
        ));
    }
    for (i, node) in doc.content.iter().enumerate() {
        if let crate::types::PrintNode::Row(row) = node {
            if row.columns.is_empty() {
                return Err(CoreError::InvalidDocument(format!(
                    "content[{i}]: row node must have at least one column"
                )));
            }
        }
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_minimal_valid_document() {
        let doc = validate_document(r#"{"version":"1.0","paper":{"kind":"thermal-80"},"content":[{"type":"text","value":"hello"}]}"#).unwrap();
        assert_eq!(doc.paper.kind, PaperKind::Thermal80);
    }

    #[test]
    fn rejects_an_unknown_node_type() {
        let err = validate_document(r#"{"version":"1.0","paper":{"kind":"thermal-80"},"content":[{"type":"not-a-real-type"}]}"#);
        assert!(err.is_err());
    }

    #[test]
    fn requires_width_mm_when_paper_kind_is_custom() {
        let err = validate_document(r#"{"version":"1.0","paper":{"kind":"custom"},"content":[]}"#);
        assert!(err.is_err());
    }

    #[test]
    fn accepts_custom_paper_with_width_mm() {
        let doc = validate_document(r#"{"version":"1.0","paper":{"kind":"custom","widthMm":120},"content":[]}"#).unwrap();
        assert_eq!(doc.paper.width_mm, Some(120.0));
    }

    #[test]
    fn rejects_a_row_with_zero_columns() {
        let err = validate_document(r#"{"version":"1.0","paper":{"kind":"thermal-58"},"content":[{"type":"row","columns":[]}]}"#);
        assert!(err.is_err());
    }

    #[test]
    fn rejects_an_unsupported_version() {
        let err = validate_document(r#"{"version":"2.0","paper":{"kind":"thermal-58"},"content":[]}"#);
        assert!(err.is_err());
    }
}
