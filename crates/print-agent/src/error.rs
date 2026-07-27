//! Error shape per docs/PROTOCOL.md: `{ "error": { "code": "...", "message": "..." } }`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum AgentError {
    Unauthorized(String),
    OriginNotAllowed(String),
    PrinterNotFound(String),
    InvalidDocument(String),
    TransportError(String),
    NotImplemented(String),
    JobNotFound(String),
}

impl AgentError {
    fn code(&self) -> &'static str {
        match self {
            AgentError::Unauthorized(_) => "UNAUTHORIZED",
            AgentError::OriginNotAllowed(_) => "ORIGIN_NOT_ALLOWED",
            AgentError::PrinterNotFound(_) => "PRINTER_NOT_FOUND",
            AgentError::InvalidDocument(_) => "INVALID_DOCUMENT",
            AgentError::TransportError(_) => "TRANSPORT_ERROR",
            AgentError::NotImplemented(_) => "NOT_IMPLEMENTED",
            AgentError::JobNotFound(_) => "JOB_NOT_FOUND",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            AgentError::Unauthorized(m)
            | AgentError::OriginNotAllowed(m)
            | AgentError::PrinterNotFound(m)
            | AgentError::InvalidDocument(m)
            | AgentError::TransportError(m)
            | AgentError::NotImplemented(m)
            | AgentError::JobNotFound(m) => m,
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            AgentError::Unauthorized(_) | AgentError::OriginNotAllowed(_) => StatusCode::UNAUTHORIZED,
            AgentError::PrinterNotFound(_) | AgentError::JobNotFound(_) => StatusCode::NOT_FOUND,
            AgentError::InvalidDocument(_) => StatusCode::BAD_REQUEST,
            AgentError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AgentError::TransportError(_) => StatusCode::BAD_GATEWAY,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
    code: &'a str,
    message: &'a str,
}

impl IntoResponse for AgentError {
    fn into_response(self) -> Response {
        let body = ErrorBody { error: ErrorDetail { code: self.code(), message: self.message() } };
        (self.status(), axum::Json(body)).into_response()
    }
}

impl From<printers_core::CoreError> for AgentError {
    fn from(err: printers_core::CoreError) -> Self {
        match err {
            printers_core::CoreError::InvalidDocument(m) => AgentError::InvalidDocument(m),
            printers_core::CoreError::Render(m) => AgentError::TransportError(m),
        }
    }
}
