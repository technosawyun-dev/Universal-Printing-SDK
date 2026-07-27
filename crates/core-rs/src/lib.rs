//! Rust twin of `packages/core-ts` -- same PrintDocument JSON schema (see
//! `docs/DOCUMENT_FORMAT.md`), same four renderer entry points, plus
//! `to_pdf`, which only exists here (used by `crates/print-agent`'s silent
//! A4/A3/Letter path -- rendering PDF in a JS context is unnecessary since
//! the agent handles that transport itself, see `docs/ARCHITECTURE.md`).
//!
//! This crate has no JS runtime dependency, which is the entire reason it
//! exists alongside `core-ts`: `crates/print-agent` and `crates/core-ffi`
//! (mobile, via uniffi) can't embed a JS engine, but every desktop/browser
//! surface can just use `core-ts` directly.

pub mod render;
pub mod types;
pub mod validate;

pub use render::escpos::to_escpos;
pub use render::html::to_html;
pub use render::pdf::to_pdf;
pub use render::svg::to_svg;
pub use render::text::to_plain_text;
pub use types::PrintDocument;
pub use validate::validate_document;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid PrintDocument: {0}")]
    InvalidDocument(String),
    #[error("render error: {0}")]
    Render(String),
}
