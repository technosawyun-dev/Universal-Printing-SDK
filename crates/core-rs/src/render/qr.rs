//! Shared QR-matrix helper for html.rs/svg.rs/pdf.rs previews. escpos.rs does
//! NOT use this -- thermal printers generate the QR code on-device via the
//! `GS ( k` command sequence (see escpos.rs); this is only for renderers
//! with no printer to do that for them.

use crate::types::QrEcLevel;
use crate::CoreError;
use qrcode::{EcLevel, QrCode};

pub struct QrMatrix {
    pub size: usize,
    dark: Vec<bool>,
}

impl QrMatrix {
    pub fn is_dark(&self, row: usize, col: usize) -> bool {
        self.dark[row * self.size + col]
    }
}

fn ec_level(ec: QrEcLevel) -> EcLevel {
    match ec {
        QrEcLevel::L => EcLevel::L,
        QrEcLevel::M => EcLevel::M,
        QrEcLevel::Q => EcLevel::Q,
        QrEcLevel::H => EcLevel::H,
    }
}

pub fn qr_modules(data: &str, ec: QrEcLevel) -> Result<QrMatrix, CoreError> {
    let code = QrCode::with_error_correction_level(data, ec_level(ec))
        .map_err(|e| CoreError::Render(format!("failed to generate QR code: {e}")))?;
    let size = code.width();
    let colors = code.to_colors();
    let dark = colors.iter().map(|c| *c == qrcode::Color::Dark).collect();
    Ok(QrMatrix { size, dark })
}
