//! PrintDocument model -- mirrors `packages/core-ts/src/types.ts` and
//! `docs/DOCUMENT_FORMAT.md` exactly. Any renderer accepting/producing this
//! JSON shape must agree with the TypeScript implementation byte-for-byte.

use serde::{Deserialize, Serialize};

use crate::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperKind {
    #[serde(rename = "thermal-58")]
    Thermal58,
    #[serde(rename = "thermal-80")]
    Thermal80,
    #[serde(rename = "a4")]
    A4,
    #[serde(rename = "a3")]
    A3,
    #[serde(rename = "letter")]
    Letter,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub kind: PaperKind,
    /// required only when kind == "custom"
    #[serde(rename = "widthMm", default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
    /// default 0 for thermal, 10 for a4/a3/letter
    #[serde(rename = "marginMm", default, skip_serializing_if = "Option::is_none")]
    pub margin_mm: Option<f64>,
    /// override; defaults: thermal-58 -> 32, thermal-80 -> 48
    #[serde(rename = "charsPerLine", default, skip_serializing_if = "Option::is_none")]
    pub chars_per_line: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// ESC/POS codepage hint, default cp437
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextSize {
    pub w: u8,
    pub h: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    #[default]
    A,
    B,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNode {
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invert: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TextSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Font>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowColumn {
    pub text: String,
    pub width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowNode {
    pub columns: Vec<RowColumn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub char: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageNode {
    /// base64 PNG
    pub data: String,
    #[serde(rename = "widthPx", default, skip_serializing_if = "Option::is_none")]
    pub width_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarcodeSymbology {
    Code128,
    Code39,
    Ean13,
    Upca,
}

impl BarcodeSymbology {
    /// Lowercase name matching the JSON schema value (docs/DOCUMENT_FORMAT.md), used by
    /// renderers that need a display label rather than the raw ESC/POS type byte.
    pub fn name(self) -> &'static str {
        match self {
            BarcodeSymbology::Code128 => "code128",
            BarcodeSymbology::Code39 => "code39",
            BarcodeSymbology::Ean13 => "ean13",
            BarcodeSymbology::Upca => "upca",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeNode {
    pub symbology: BarcodeSymbology,
    pub data: String,
    #[serde(rename = "heightPx", default, skip_serializing_if = "Option::is_none")]
    pub height_px: Option<f64>,
    #[serde(rename = "showText", default, skip_serializing_if = "Option::is_none")]
    pub show_text: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QrEcLevel {
    L,
    M,
    Q,
    H,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeNode {
    pub data: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ec: Option<QrEcLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CutMode {
    Full,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<CutMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawerNode {
    /// 2 or 5
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawNode {
    #[serde(rename = "escposBase64")]
    pub escpos_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PrintNode {
    #[serde(rename = "text")]
    Text(TextNode),
    #[serde(rename = "row")]
    Row(RowNode),
    #[serde(rename = "rule")]
    Rule(RuleNode),
    #[serde(rename = "feed")]
    Feed(FeedNode),
    #[serde(rename = "image")]
    Image(ImageNode),
    #[serde(rename = "barcode")]
    Barcode(BarcodeNode),
    #[serde(rename = "qrcode")]
    QrCode(QrCodeNode),
    #[serde(rename = "cut")]
    Cut(CutNode),
    #[serde(rename = "drawer")]
    Drawer(DrawerNode),
    #[serde(rename = "raw")]
    Raw(RawNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDocument {
    pub version: String,
    pub paper: Paper,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<DocMeta>,
    pub content: Vec<PrintNode>,
}

/// thermal-58 -> 32, thermal-80 -> 48, per docs/DOCUMENT_FORMAT.md's compatibility table
pub fn resolve_chars_per_line(paper: &Paper) -> u32 {
    if let Some(c) = paper.chars_per_line {
        return c;
    }
    match paper.kind {
        PaperKind::Thermal58 => 32,
        PaperKind::Thermal80 => 48,
        // a4/a3/letter/custom: renderers that need a char grid (plain text) fall back to 80
        _ => 80,
    }
}

pub fn resolve_margin_mm(paper: &Paper) -> f64 {
    if let Some(m) = paper.margin_mm {
        return m;
    }
    match paper.kind {
        PaperKind::Thermal58 | PaperKind::Thermal80 => 0.0,
        _ => 10.0,
    }
}

/// width in mm for page-geometry renderers (html/svg/pdf)
pub fn resolve_page_width_mm(paper: &Paper) -> Result<f64, CoreError> {
    Ok(match paper.kind {
        PaperKind::Thermal58 => 58.0,
        PaperKind::Thermal80 => 80.0,
        PaperKind::A4 => 210.0,
        PaperKind::A3 => 297.0,
        PaperKind::Letter => 215.9,
        PaperKind::Custom => paper.width_mm.ok_or_else(|| {
            CoreError::InvalidDocument("paper.widthMm is required when paper.kind is 'custom'".to_string())
        })?,
    })
}

pub fn is_thermal(paper: &Paper) -> bool {
    matches!(paper.kind, PaperKind::Thermal58 | PaperKind::Thermal80)
}

/// page height in mm for normal (non-thermal) paper; thermal is unbounded (roll), returns None
pub fn resolve_page_height_mm(paper: &Paper) -> Option<f64> {
    match paper.kind {
        PaperKind::A4 => Some(297.0),
        PaperKind::A3 => Some(420.0),
        PaperKind::Letter => Some(279.4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper(kind: PaperKind) -> Paper {
        Paper { kind, width_mm: None, margin_mm: None, chars_per_line: None }
    }

    #[test]
    fn defaults_chars_per_line_per_paper_kind() {
        assert_eq!(resolve_chars_per_line(&paper(PaperKind::Thermal58)), 32);
        assert_eq!(resolve_chars_per_line(&paper(PaperKind::Thermal80)), 48);
        assert_eq!(resolve_chars_per_line(&paper(PaperKind::A4)), 80);
    }

    #[test]
    fn lets_explicit_chars_per_line_override_default() {
        let mut p = paper(PaperKind::Thermal80);
        p.chars_per_line = Some(42);
        assert_eq!(resolve_chars_per_line(&p), 42);
    }

    #[test]
    fn defaults_margin_to_0_for_thermal_and_10_for_page_paper() {
        assert_eq!(resolve_margin_mm(&paper(PaperKind::Thermal58)), 0.0);
        assert_eq!(resolve_margin_mm(&paper(PaperKind::A4)), 10.0);
        let mut p = paper(PaperKind::A4);
        p.margin_mm = Some(5.0);
        assert_eq!(resolve_margin_mm(&p), 5.0);
    }

    #[test]
    fn resolves_page_width_per_kind_and_requires_width_mm_for_custom() {
        assert_eq!(resolve_page_width_mm(&paper(PaperKind::Thermal58)).unwrap(), 58.0);
        assert_eq!(resolve_page_width_mm(&paper(PaperKind::A4)).unwrap(), 210.0);
        let mut custom = paper(PaperKind::Custom);
        custom.width_mm = Some(100.0);
        assert_eq!(resolve_page_width_mm(&custom).unwrap(), 100.0);
        assert!(resolve_page_width_mm(&paper(PaperKind::Custom)).is_err());
    }

    #[test]
    fn resolves_page_height_only_for_bounded_page_paper() {
        assert_eq!(resolve_page_height_mm(&paper(PaperKind::A4)), Some(297.0));
        assert_eq!(resolve_page_height_mm(&paper(PaperKind::Thermal80)), None);
    }

    #[test]
    fn identifies_thermal_paper_kinds() {
        assert!(is_thermal(&paper(PaperKind::Thermal58)));
        assert!(is_thermal(&paper(PaperKind::Thermal80)));
        assert!(!is_thermal(&paper(PaperKind::A4)));
    }
}
