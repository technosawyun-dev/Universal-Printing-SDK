//! HTTP handlers, one per docs/PROTOCOL.md endpoint.

use crate::error::AgentError;
use crate::jobs::JobState;
use crate::printers::{discover_tcp, list_all, Printer};
use crate::state::AppState;
use crate::transport;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

fn current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

fn origin_of(headers: &HeaderMap) -> String {
    headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string()
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
    platform: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", version: env!("CARGO_PKG_VERSION"), platform: current_platform() })
}

#[derive(Deserialize)]
pub struct PairRequest {
    #[serde(rename = "appName")]
    #[allow(dead_code)]
    app_name: Option<String>,
}

#[derive(Serialize)]
pub struct PairResponse {
    token: String,
}

pub async fn pair(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(_req): Json<PairRequest>) -> Json<PairResponse> {
    let token = state.tokens.pair(&origin_of(&headers));
    Json(PairResponse { token })
}

pub async fn pair_revoke(State(state): State<Arc<AppState>>, headers: HeaderMap) -> axum::http::StatusCode {
    state.tokens.revoke(&origin_of(&headers));
    axum::http::StatusCode::NO_CONTENT
}

#[derive(Serialize)]
pub struct PrintersResponse {
    printers: Vec<Printer>,
}

pub async fn printers(State(state): State<Arc<AppState>>) -> Json<PrintersResponse> {
    Json(PrintersResponse { printers: list_all(&state.discovered).await })
}

#[derive(Deserialize)]
pub struct DiscoverRequest {
    subnet: Option<String>,
    #[serde(rename = "timeoutMs")]
    timeout_ms: Option<u64>,
}

pub async fn discover(State(state): State<Arc<AppState>>, Json(req): Json<DiscoverRequest>) -> Json<PrintersResponse> {
    let found = discover_tcp(req.subnet, req.timeout_ms.unwrap_or(2000)).await;
    state.discovered.add_tcp(found.clone());
    if !found.is_empty() {
        state.broadcast_printers_changed();
    }
    Json(PrintersResponse { printers: found })
}

#[derive(Serialize)]
pub struct JobResponse {
    #[serde(rename = "jobId")]
    job_id: String,
    status: JobState,
}

async fn run_job(state: Arc<AppState>, job_id: String, printer_id: String, bytes: Vec<u8>) {
    // Held for the duration of the transport write -- a second job queued
    // for the same printer stays "queued" (not "printing") until this one
    // releases the lock, so two jobs can never interleave bytes on the wire.
    let _printer_guard = state.lock_printer(&printer_id).await;
    state.jobs.set(&job_id, JobState::Printing, None);
    state.broadcast_job_status(&job_id, JobState::Printing);
    match transport::send_to_printer(&printer_id, bytes).await {
        Ok(()) => {
            state.jobs.set(&job_id, JobState::Done, None);
            state.broadcast_job_status(&job_id, JobState::Done);
        }
        Err(e) => {
            state.jobs.set(&job_id, JobState::Error, Some(e.message().to_string()));
            state.broadcast_job_status(&job_id, JobState::Error);
        }
    }
}

fn spawn_job(state: Arc<AppState>, printer_id: String, bytes: Vec<u8>) -> JobResponse {
    let job_id = state.jobs.create();
    let job_id_for_task = job_id.clone();
    tokio::spawn(run_job(state, job_id_for_task, printer_id, bytes));
    JobResponse { job_id, status: JobState::Queued }
}

fn render_document_bytes(document: &serde_json::Value) -> Result<Vec<u8>, AgentError> {
    let doc = printers_core::validate_document(&document.to_string())?;
    Ok(printers_core::to_escpos(&doc)?)
}

#[derive(Deserialize)]
pub struct PrintRequest {
    #[serde(rename = "printerId")]
    printer_id: String,
    document: Option<serde_json::Value>,
    #[serde(rename = "rawBase64")]
    raw_base64: Option<String>,
}

pub async fn print(State(state): State<Arc<AppState>>, Json(req): Json<PrintRequest>) -> Result<Json<JobResponse>, AgentError> {
    let bytes = match (&req.raw_base64, &req.document) {
        (Some(raw), _) => base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|e| AgentError::InvalidDocument(format!("invalid rawBase64: {e}")))?,
        (None, Some(doc)) => render_document_bytes(doc)?,
        (None, None) => return Err(AgentError::InvalidDocument("must provide either 'document' or 'rawBase64'".to_string())),
    };
    Ok(Json(spawn_job(state, req.printer_id, bytes)))
}

#[derive(Deserialize)]
pub struct PrintTcpRequest {
    host: String,
    port: u16,
    document: serde_json::Value,
}

pub async fn print_tcp(State(state): State<Arc<AppState>>, Json(req): Json<PrintTcpRequest>) -> Result<Json<JobResponse>, AgentError> {
    let bytes = render_document_bytes(&req.document)?;
    Ok(Json(spawn_job(state, format!("tcp:{}:{}", req.host, req.port), bytes)))
}

#[derive(Deserialize)]
pub struct PrintSerialRequest {
    port: String,
    #[serde(rename = "baudRate")]
    #[allow(dead_code)]
    baud_rate: Option<u32>,
    document: serde_json::Value,
}

pub async fn print_serial(State(state): State<Arc<AppState>>, Json(req): Json<PrintSerialRequest>) -> Result<Json<JobResponse>, AgentError> {
    let bytes = render_document_bytes(&req.document)?;
    Ok(Json(spawn_job(state, format!("serial:{}", req.port), bytes)))
}

#[derive(Deserialize)]
pub struct PrintBluetoothRequest {
    address: String,
    document: serde_json::Value,
}

pub async fn print_bluetooth(State(state): State<Arc<AppState>>, Json(req): Json<PrintBluetoothRequest>) -> Result<Json<JobResponse>, AgentError> {
    let bytes = render_document_bytes(&req.document)?;
    Ok(Json(spawn_job(state, format!("bt:{}", req.address), bytes)))
}

#[derive(Deserialize)]
pub struct PrintSilentRequest {
    #[serde(rename = "printerId")]
    printer_id: String,
    document: serde_json::Value,
    copies: Option<u32>,
}

pub async fn print_silent(State(state): State<Arc<AppState>>, Json(req): Json<PrintSilentRequest>) -> Result<Json<JobResponse>, AgentError> {
    let doc = printers_core::validate_document(&req.document.to_string())?;
    let pdf_bytes = printers_core::to_pdf(&doc)?;
    let printer_id = req.printer_id.clone();
    let copies = req.copies.unwrap_or(1);
    let job_id = state.jobs.create();
    let job_id_for_task = job_id.clone();
    let state_for_task = state.clone();
    tokio::spawn(async move {
        // See run_job's comment: serializes concurrent jobs targeting the same printer.
        let _printer_guard = state_for_task.lock_printer(&printer_id).await;
        state_for_task.jobs.set(&job_id_for_task, JobState::Printing, None);
        state_for_task.broadcast_job_status(&job_id_for_task, JobState::Printing);
        match transport::print_silent(&printer_id, &doc, &pdf_bytes, copies).await {
            Ok(()) => {
                state_for_task.jobs.set(&job_id_for_task, JobState::Done, None);
                state_for_task.broadcast_job_status(&job_id_for_task, JobState::Done);
            }
            Err(e) => {
                state_for_task.jobs.set(&job_id_for_task, JobState::Error, Some(e.message().to_string()));
                state_for_task.broadcast_job_status(&job_id_for_task, JobState::Error);
            }
        }
    });
    Ok(Json(JobResponse { job_id, status: JobState::Queued }))
}

pub async fn get_job(State(state): State<Arc<AppState>>, Path(job_id): Path<String>) -> Result<Json<crate::jobs::JobStatus>, AgentError> {
    state.jobs.get(&job_id).map(Json).ok_or_else(|| AgentError::JobNotFound(format!("no job with id '{job_id}'")))
}
